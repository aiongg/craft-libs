//! Stories of the dialogs on the host, one per kind. Each keeps a [`DialogHost`] in egui memory,
//! opens the real dialog on it and shows the result after the dialog answers.

use std::sync::{Arc, Mutex};

use craft_quit::CloseScope;
use egui::{Context, Ui};

use super::{
    Alert, CanvasSizeDialog, CommandForm, DialogHost, DialogId, DialogLabels, DialogResult, DialogStack, ImageSizeDialog, NoteTone, Param,
    ParamDialog, ParamLayout, PixelSize, PromptKind, PropertiesDialog, PropertiesTab, PropertyRow, ReportDialog, ReportNote, Resample, StackDialog,
    StandardPrompt, StartupStage, UnsavedChangesDialog, dialog_id,
};
use crate::localized::Labels as _;
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum LayoutArg {
    SideColumn,
    BottomFooter,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ParamsArgs {
    open: bool,
    layout: LayoutArg,
    preview: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FormArgs {
    open: bool,
    #[arg(name = "ok enabled")]
    ok_enabled: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizeKindArg {
    Image,
    Canvas,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SizeArgs {
    open: bool,
    kind: SizeKindArg,
    resample: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PromptKindArg {
    ConfirmDelete,
    DamagedFile,
    ChangedOnDisk,
    ReplaceFile,
    TrashMask,
    Unsaved,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PromptArgs {
    open: bool,
    kind: PromptKindArg,
    #[arg(name = "several documents")]
    several: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PropertiesArgs {
    open: bool,
    #[arg(name = "read only")]
    read_only: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ReportArgs {
    open: bool,
    #[arg(range = 0..=40)]
    notes: usize,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct QueueArgs {
    recovery: bool,
    warning: bool,
    update: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/dialog/params",
            Level::Patterns,
            "Dialog",
            "Parameters",
            ParamsArgs { open: true, layout: LayoutArg::SideColumn, preview: true },
            show_params
        ),
        typed_story!("patterns/dialog/command-form", Level::Patterns, "Dialog", "Command form", FormArgs { open: true, ok_enabled: true }, show_form),
        typed_story!(
            "patterns/dialog/size",
            Level::Patterns,
            "Dialog",
            "Image and canvas size",
            SizeArgs { open: true, kind: SizeKindArg::Image, resample: true },
            show_size
        ),
        typed_story!(
            "patterns/dialog/standard-prompt",
            Level::Patterns,
            "Dialog",
            "Standard prompts",
            PromptArgs { open: true, kind: PromptKindArg::ConfirmDelete, several: false },
            show_prompt
        ),
        typed_story!(
            "patterns/dialog/properties",
            Level::Patterns,
            "Dialog",
            "Properties",
            PropertiesArgs { open: true, read_only: false },
            show_properties
        ),
        typed_story!("patterns/dialog/report", Level::Patterns, "Dialog", "Report", ReportArgs { open: true, notes: 6 }, show_report),
        typed_story!(
            "patterns/dialog/startup-queue",
            Level::Patterns,
            "Dialog",
            "Startup queue",
            QueueArgs { recovery: true, warning: true, update: true },
            show_queue
        ),
    ]
}

type SharedHost = Arc<Mutex<DialogHost>>;

fn lock(host: &SharedHost) -> std::sync::MutexGuard<'_, DialogHost> {
    host.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The story's host and its last result text, kept in egui memory under `name`.
fn state(ui: &Ui, name: &str) -> (SharedHost, egui::Id) {
    let key = ui.make_persistent_id(("craft_ui::dialog::framework_stories", name));
    let host: SharedHost = ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, SharedHost::default).clone());
    (host, key.with("result"))
}

/// Whether this story instance may draw the dialog `name` in this pass. The size, prompt and
/// unsaved dialogs have one fixed id each, so two instances of a story drawn in the same pass
/// (a component tab and a variants view) must not both show it.
fn claim(ctx: &Context, name: &str, instance: egui::Id) -> bool {
    let key = egui::Id::new(("craft_ui::dialog::framework_stories::claim", name));
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let (claimed_pass, owner) = d.get_temp::<(u64, egui::Id)>(key).unwrap_or((u64::MAX, instance));
        if claimed_pass == pass && owner != instance {
            return false;
        }
        d.insert_temp(key, (pass, instance));
        true
    })
}

/// Opens `D` while the story is shown and `open` is on, reopens it when `signature` changes (the
/// arguments that build the dialog), and shows the answer as text under the tile once it closes.
fn drive<D: StackDialog, S: PartialEq + Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    name: &str,
    exclusive: bool,
    open: &mut bool,
    signature: S,
    make: impl FnOnce() -> D,
    tune: impl FnOnce(&mut D),
    describe: impl FnOnce(D::Result) -> String,
) {
    let (host, result_key) = state(ui, name);
    let signature_key = result_key.with("signature");
    let ctx = ui.ctx().clone();
    if exclusive && !claim(&ctx, name, ui.id()) {
        return;
    }
    let mut host = lock(&host);
    if crate::story::dialog_shown(ui, *open) {
        let previous: Option<S> = ui.data(|d| d.get_temp::<Option<S>>(signature_key)).flatten();
        if previous.is_some_and(|previous| previous != signature) {
            host.cancel::<D>();
            host.take::<D>();
        }
        ui.data_mut(|d| d.insert_temp(signature_key, Some(signature)));
        if !host.is_open::<D>() {
            host.open(make());
            ui.data_mut(|d| d.remove_temp::<String>(result_key));
        }
        if let Some(dialog) = host.get_mut::<D>() {
            tune(dialog);
        }
        host.show(&ctx);
        if let Some(result) = host.take::<D>() {
            let text = match result {
                DialogResult::Ok(value) => describe(value),
                DialogResult::Cancel => "Cancelled".to_owned(),
            };
            ui.data_mut(|d| d.insert_temp(result_key, text));
            crate::story::dialog_closed(ui, open);
        }
    } else {
        host.cancel::<D>();
        host.take::<D>();
        ui.data_mut(|d| d.insert_temp(signature_key, None::<S>));
    }
    if let Some(text) = ui.data(|d| d.get_temp::<String>(result_key)) {
        ui.label(format!("Result: {text}"));
    }
}

fn fields_text(fields: &super::DialogFields) -> String {
    let listed: Vec<String> = fields.iter().map(|(key, value)| format!("{key} = {value:?}")).collect();
    if listed.is_empty() { "no fields".to_owned() } else { listed.join(", ") }
}

const FILTER_SPEC: &str = r#"{"radius":0.1..1000=1,"method":"spin|zoom","angle":angle=30,"monochromatic":bool,"seed":u32=0}"#;

/// Gaussian Blur-style parameters on the host; OK shows the fields.
fn show_params(ui: &mut Ui, args: &mut ParamsArgs) {
    let salt = ui.id();
    let layout = match args.layout {
        LayoutArg::SideColumn => ParamLayout::SideColumn,
        LayoutArg::BottomFooter => ParamLayout::BottomFooter,
    };
    let preview = args.preview;
    drive(
        ui,
        "params",
        false,
        &mut args.open,
        (args.layout, preview),
        || ParamDialog::new(("story-params", salt), "Radial Blur", Param::parse_list(FILTER_SPEC)).preview(preview).layout(layout),
        |_| {},
        |fields| format!("OK, {}", fields_text(&fields)),
    );
}

/// A command form with Reset-free OK; OK is disabled while `ok enabled` is off.
fn show_form(ui: &mut Ui, args: &mut FormArgs) {
    let salt = ui.id();
    let enabled = args.ok_enabled;
    drive(
        ui,
        "form",
        false,
        &mut args.open,
        (),
        || CommandForm::new(("story-form", salt), "Go to Page", Param::parse_list(r#"{"page":u32=1,"select":bool}"#)).ok_label("Go"),
        |form| form.set_ok_enabled(enabled),
        |fields| format!("OK, {}", fields_text(&fields)),
    );
}

const ORIGINAL: PixelSize = PixelSize { width: 1200, height: 800, pixels_per_inch: 72.0 };

/// Image Size or Canvas Size on the host.
fn show_size(ui: &mut Ui, args: &mut SizeArgs) {
    match args.kind {
        SizeKindArg::Image => {
            let method = if args.resample { Resample::Automatic } else { Resample::None };
            drive(
                ui,
                "size-image",
                true,
                &mut args.open,
                method,
                || ImageSizeDialog::new(ORIGINAL).resample(method),
                |_| {},
                |result| {
                    format!("OK, {} x {} px at {} ppi, {:?}", result.size.width, result.size.height, result.size.pixels_per_inch, result.resample)
                },
            );
        }
        SizeKindArg::Canvas => drive(
            ui,
            "size-canvas",
            true,
            &mut args.open,
            (),
            || CanvasSizeDialog::new(ORIGINAL),
            |_| {},
            |result| format!("OK, {} x {} px, anchor {:?}", result.size.width, result.size.height, result.anchor),
        ),
    }
}

/// The standard prompts and the unsaved-changes question.
fn show_prompt(ui: &mut Ui, args: &mut PromptArgs) {
    let several = args.several;
    let kind = match args.kind {
        PromptKindArg::ConfirmDelete => Some(if several {
            PromptKind::ConfirmDelete { names: vec!["Background".to_owned(), "Layer 1".to_owned()], count: 5 }
        } else {
            PromptKind::ConfirmDelete { names: vec!["Background".to_owned()], count: 1 }
        }),
        PromptKindArg::DamagedFile => Some(PromptKind::DamagedFile { name: "poster.psd".to_owned() }),
        PromptKindArg::ChangedOnDisk => Some(PromptKind::ChangedOnDisk { name: "poster.psd".to_owned() }),
        PromptKindArg::ReplaceFile => {
            Some(PromptKind::ReplaceFile { name: "poster.png".to_owned(), answered: several.then(|| "poster.png.png".to_owned()) })
        }
        PromptKindArg::TrashMask => Some(PromptKind::TrashMask),
        PromptKindArg::Unsaved => None,
    };
    match kind {
        Some(kind) => {
            drive(ui, "prompt", true, &mut args.open, (args.kind, several), || StandardPrompt::new(kind), |_| {}, |choice| format!("{choice:?}"))
        }
        None => drive(
            ui,
            "unsaved",
            true,
            &mut args.open,
            several,
            || UnsavedChangesDialog::new(CloseScope::Close, "poster.psd").remaining(if several { 3 } else { 1 }),
            |_| {},
            |answer| format!("{:?}{}", answer.answer, if answer.apply_to_all { ", apply to all" } else { "" }),
        ),
    }
}

/// A two-tab Document Properties; read only shows values as text with a single OK.
fn show_properties(ui: &mut Ui, args: &mut PropertiesArgs) {
    let salt = ui.id();
    let read_only = args.read_only;
    let tabs = || {
        vec![
            PropertiesTab::new(
                "General",
                vec![
                    PropertyRow::Value { label: "File:".to_owned(), text: "poster.psd".to_owned() },
                    PropertyRow::Value { label: "Size:".to_owned(), text: "1200 \u{d7} 800 px".to_owned() },
                    PropertyRow::Space,
                    PropertyRow::Heading("Description".to_owned()),
                    PropertyRow::Field(Param::new("title", "Title:", super::ParamKind::Text { default: "Poster".to_owned() })),
                    PropertyRow::Field(Param::new("author", "Author:", super::ParamKind::Text { default: String::new() })),
                ],
            ),
            PropertiesTab::new(
                "Advanced",
                vec![
                    PropertyRow::Field(Param::new("compress", "Compress:", super::ParamKind::Bool { default: true })),
                    PropertyRow::Value { label: "Colour profile:".to_owned(), text: "sRGB IEC61966-2.1".to_owned() },
                ],
            ),
        ]
    };
    drive(
        ui,
        "properties",
        false,
        &mut args.open,
        read_only,
        || PropertiesDialog::new(("story-properties", salt), "Document Properties", tabs()).read_only(read_only),
        |_| {},
        |fields| format!("OK, {}", fields_text(&fields)),
    );
}

/// An import report with `notes` lines, tones cycling.
fn show_report(ui: &mut Ui, args: &mut ReportArgs) {
    let salt = ui.id();
    let count = args.notes;
    drive(
        ui,
        "report",
        false,
        &mut args.open,
        count,
        || {
            let notes = (0..count)
                .map(|n| match n % 3 {
                    0 => ReportNote::new(NoteTone::Info, format!("Page {} imported.", n + 1)),
                    1 => ReportNote::new(NoteTone::Warning, format!("Font on page {} was substituted.", n + 1)),
                    _ => ReportNote::new(NoteTone::Error, format!("Image on page {} could not be read.", n + 1)),
                })
                .collect();
            ReportDialog::new(("story-report", salt), "Import Report", format!("Imported {count} pages."), notes)
        },
        |_| {},
        |()| "OK".to_owned(),
    );
}

/// A notice that the host can queue.
struct Notice {
    id: DialogId,
    key: (&'static str, egui::Id),
    title: &'static str,
    message: &'static str,
}

impl StackDialog for Notice {
    type Result = ();

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        Alert::new(self.key, self.title, self.message).ok_label(DialogLabels::of_ctx(ctx).ok).stack(stack).show(ctx).then_some(DialogResult::Ok(()))
    }
}

fn notice(key: (&'static str, egui::Id), title: &'static str, message: &'static str) -> Notice {
    Notice { id: dialog_id(key), key, title, message }
}

/// Three alerts enqueued in the ticked stages; they open one at a time in stage order.
fn show_queue(ui: &mut Ui, args: &mut QueueArgs) {
    let salt = ui.id();
    let (host, result_key) = state(ui, "queue");
    let signature_key = result_key.with("signature");
    let signature = (args.recovery, args.warning, args.update);
    let mut open = true;
    if !crate::story::dialog_shown(ui, open) {
        ui.data_mut(|d| d.remove_temp::<(bool, bool, bool)>(signature_key));
        return;
    }
    let ctx = ui.ctx().clone();
    let mut host = lock(&host);
    let started: Option<(bool, bool, bool)> = ui.data(|d| d.get_temp(signature_key));
    if started != Some(signature) {
        *host = DialogHost::new();
        // Enqueued in reverse stage order: the host sorts them.
        if args.update {
            host.enqueue(StartupStage::Update, notice(("story-queue-update", salt), "Update available", "Version 2.0 is ready to download."));
        }
        if args.warning {
            host.enqueue(StartupStage::Warning, notice(("story-queue-warning", salt), "Graphics", "The GPU is not supported; drawing uses the CPU."));
        }
        if args.recovery {
            host.enqueue(
                StartupStage::Recovery,
                notice(("story-queue-recovery", salt), "Recover documents", "2 documents were saved before the last quit."),
            );
        }
        ui.data_mut(|d| d.insert_temp(signature_key, signature));
    }
    host.show(&ctx);
    if host.is_empty() {
        crate::story::dialog_closed(ui, &mut open);
        if ui.button("Run again").clicked() {
            ui.data_mut(|d| d.remove_temp::<(bool, bool, bool)>(signature_key));
        }
    } else {
        ui.label("Recovery, then warning, then update.");
    }
}
