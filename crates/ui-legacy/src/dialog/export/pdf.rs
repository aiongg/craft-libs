//! File › Save as PDF: the Save PDF dialog over [`craft_pdf::settings`].

mod sections;
mod summary;
mod words;

use std::path::PathBuf;
use std::sync::Arc;

use craft_host::{Host, PickRequest};
use craft_pdf::settings::{
    Choice, ColorMode, Compatibility, PdfPreset, PdfSettings, ProfileCatalog, SettingsError, Standard, builtin_presets, default_preset_name,
};
use craft_theme::TypeRole;
use egui::text::LayoutJob;
use egui::{Context, Label, Ui};

use self::sections::{ArtboardChoice, PasswordSwitches, Sections};
pub use self::words::PdfExportLabels;
use super::super::dialog::{Dialog, Outcome};
use super::super::dialog_host::{DialogResult, StackDialog};
use super::super::footer::Footer;
use super::super::size::Width;
use super::super::stack::{DialogId, DialogStack, dialog_id};
use super::super::state::DialogState;
use super::{ExportAsLabels, export_file_name};
use crate::behavior::{RowInfo, Selection, SelectionMode};
use crate::composites::FormGrid;
use crate::controls::{Button, Select, TextField};
use crate::edit::EditEvent;
use crate::list::{List, ListEvent, ListRow};
use crate::localized::units::LengthUnit;
use crate::localized::{LocaleNumbers, SharedLabels as _};
use crate::primitives::Text;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};

/// Rows of the settings page beside the section list.
const CONTENT_ROWS: f32 = 12.0;

/// Which artboards to write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PdfArtboards {
    /// Every artboard, one page each.
    #[default]
    All,
    /// A 1-based range such as `"1-3, 5"`, which the app reads.
    Range(String),
}

/// What Save PDF returns: the settings, the artboards and the path the user chose.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfExportResult {
    /// The settings, with the passwords typed (a preset never stores them).
    pub options: PdfSettings,
    /// The file the user chose to write.
    pub path: PathBuf,
    /// Which artboards to include.
    pub artboards: PdfArtboards,
}

/// File › Save as PDF (spec Q-S4F-6; VectorCraft's Save PDF).
///
/// The user sees Preset (the built-in presets and the app's saved ones, with Save Preset… to name
/// the current settings), Standard and Compatibility, then a list of sections: General,
/// Compression, Marks and Bleeds, Output, Advanced, Security and Summary. A standard sets the
/// PDF version it allows and turns off what its files can't have (editing data, passwords,
/// PDF layers in PDF/X-1a and PDF/X-3). Choosing a preset fills every row.
///
/// The checks of [`craft_pdf::settings`] run on every pass; while one fails its message shows
/// under the sections and Save PDF is disabled. Save PDF queues a save picker through
/// `host.files()` (purpose `"pdf"`) and finishes on a path with [`PdfExportResult`]; a cancelled
/// pick keeps the dialog open.
///
/// Save Preset… asks for a name in a row of its own; Save then makes the preset available in the
/// list and reports it through [`take_preset`](Self::take_preset), which the app stores (with
/// the passwords cleared) and hands back with [`saved_presets`](Self::saved_presets) the next
/// time it opens the dialog. Edit › PDF Presets (the preset manager) is the app's.
pub struct PdfExportDialog {
    id: DialogId,
    app: String,
    file_name: String,
    settings: PdfSettings,
    /// The name of the preset shown in the Preset list.
    preset: String,
    saved: Vec<PdfPreset>,
    flatteners: Vec<String>,
    profiles: Vec<String>,
    catalog: Arc<dyn ProfileCatalog + Send + Sync>,
    color_mode: ColorMode,
    unit: LengthUnit,
    document_bleed: [f64; 4],
    artboards: ArtboardChoice,
    artboard_count: Option<usize>,
    switches: PasswordSwitches,
    section: usize,
    /// The name being typed for Save Preset….
    naming: Option<String>,
    new_preset: Option<PdfPreset>,
    picking: Option<craft_host::PickId>,
    state: DialogState<()>,
}

impl PdfExportDialog {
    /// The dialog for the document named `file_name` (no extension) in the application called
    /// `app`, which names the default preset (`"{app} Default"`). It starts on that preset.
    pub fn new(app: impl Into<String>, file_name: impl Into<String>) -> Self {
        let app = app.into();
        let preset = default_preset_name(&app);
        let settings = builtin_presets(&app).into_iter().next().map(|preset| preset.settings).unwrap_or_default();
        Self {
            id: dialog_id("save-pdf"),
            app,
            file_name: file_name.into(),
            settings,
            preset,
            saved: Vec::new(),
            flatteners: Vec::new(),
            profiles: Vec::new(),
            catalog: Arc::new(()),
            color_mode: ColorMode::Rgb,
            unit: LengthUnit::Points,
            document_bleed: [0.0; 4],
            artboards: ArtboardChoice::default(),
            artboard_count: None,
            switches: PasswordSwitches::default(),
            section: 0,
            naming: None,
            new_preset: None,
            picking: None,
            state: DialogState::new(),
        }
    }

    /// Opens with these settings over the shown preset (a command run with arguments).
    pub fn initial(mut self, settings: PdfSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Opens on the preset called `name` (built-in or [saved](Self::saved_presets)) with its
    /// settings, unless [`initial`](Self::initial) gives others.
    pub fn preset(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if let Some(preset) = self.presets().into_iter().find(|preset| preset.name == name) {
            self.settings = preset.settings;
            self.preset = name;
        }
        self
    }

    /// The presets the user saved, after the built-in ones in the Preset list.
    pub fn saved_presets(mut self, presets: Vec<PdfPreset>) -> Self {
        self.saved = presets;
        self
    }

    /// The names of the saved flattener presets, after the three built-in ones.
    pub fn flattener_presets(mut self, names: Vec<String>) -> Self {
        self.flatteners = names;
        self
    }

    /// The colour profiles the app knows (Destination and Output intent profile list them) and
    /// the lookup the checks use for them.
    pub fn profiles(mut self, names: Vec<String>, catalog: Arc<dyn ProfileCatalog + Send + Sync>) -> Self {
        self.profiles = names;
        self.catalog = catalog;
        self
    }

    /// The colour mode of the document, which a blank destination profile follows.
    pub fn color_mode(mut self, mode: ColorMode) -> Self {
        self.color_mode = mode;
        self
    }

    /// The unit lengths (offset, bleed) are shown in; pixels and percent show points.
    pub fn length_unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    /// The document's bleed in points (top, bottom, left, right), which "Use document bleed
    /// settings" shows.
    pub fn document_bleed(mut self, bleed: [f64; 4]) -> Self {
        self.document_bleed = bleed;
        self
    }

    /// Opens with these artboards chosen.
    pub fn artboards(mut self, artboards: PdfArtboards) -> Self {
        self.artboards = match artboards {
            PdfArtboards::All => ArtboardChoice::default(),
            PdfArtboards::Range(range) => ArtboardChoice { range_only: true, range },
        };
        self
    }

    /// The number of artboards of the document, which a typed range is checked against: a range
    /// outside `1..=n` disables Save PDF with the reason.
    pub fn artboard_count(mut self, count: usize) -> Self {
        self.artboard_count = Some(count);
        self
    }

    /// The settings the dialog shows.
    pub fn settings(&self) -> &PdfSettings {
        &self.settings
    }

    #[cfg(test)]
    fn settings_mut_for_test(&mut self) -> &mut PdfSettings {
        &mut self.settings
    }

    /// The preset Save Preset… named, once: the app stores it. Its passwords are cleared.
    pub fn take_preset(&mut self) -> Option<PdfPreset> {
        self.new_preset.take()
    }

    /// Every preset the list shows: the built-in ones, then the saved ones.
    fn presets(&self) -> Vec<PdfPreset> {
        let mut presets = builtin_presets(&self.app);
        presets.extend(self.saved.iter().cloned());
        presets
    }

    /// Why Save PDF is disabled: a refused setting, or a range the document doesn't have.
    fn blocked(&self, labels: &PdfExportLabels) -> Option<String> {
        if let Some(error) = self.error() {
            return Some(labels.error_text(&error));
        }
        let text = self.artboards.range.trim();
        let count = self.artboard_count?;
        if !self.artboards.range_only || text.is_empty() {
            return None;
        }
        summary::check_range(text, count).err().map(|error| labels.range_error_text(text, count, &error))
    }

    fn error(&self) -> Option<SettingsError> {
        let catalog = &*self.catalog;
        self.settings.check(catalog).and_then(|()| self.settings.check_destination(catalog, self.color_mode)).err()
    }

    /// The picker's answer, once it has come: the first path.
    fn picked(&mut self, ctx: &Context) -> Option<PathBuf> {
        let ticket = self.picking?;
        let host = Host::try_of_ctx(ctx)?;
        let picked = host.files().take_answer(ticket)?;
        self.picking = None;
        picked.first_path()
    }

    fn forget_pick(&mut self, ctx: &Context) {
        if let Some(ticket) = self.picking.take()
            && let Some(host) = Host::try_of_ctx(ctx)
        {
            host.files().forget(ticket);
        }
    }

    fn queue_save(&mut self, ctx: &Context, labels: &PdfExportLabels) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let untitled = ExportAsLabels::shared_ctx(ctx);
        let request = PickRequest::save("pdf")
            .title(labels.title.as_str())
            .file_name(format!("{}.pdf", export_file_name(&self.file_name, &untitled.untitled)))
            .suggested_extension("pdf")
            .filter("PDF", &["pdf"]);
        self.picking = Some(host.files().pick(request));
    }

    /// A standard sets the version it allows (unless the chosen one is allowed) and turns off
    /// what its files can't have.
    fn standard_chosen(&mut self) {
        let settings = &mut self.settings;
        let standard = settings.standard;
        if !standard.allows(settings.compatibility) {
            settings.compatibility = standard.version();
        }
        if standard != Standard::None {
            settings.preserve_editing = false;
            settings.clear_passwords();
            self.switches = PasswordSwitches::default();
        }
        if !standard.allows_layers() {
            settings.create_layers = false;
        }
    }

    /// Fills every row from the preset called `name`; the artboards and the file stay.
    fn apply_preset(&mut self, name: &str) {
        let Some(preset) = self.presets().into_iter().find(|preset| preset.name == name) else { return };
        self.settings = preset.settings;
        self.preset = preset.name;
        self.switches = PasswordSwitches::default();
    }
}

/// The Preset row and, while Save Preset… asks for a name, the row taking it. Returns the
/// preset chosen and whether the name was saved.
fn top_rows(
    ui: &mut Ui,
    labels: &PdfExportLabels,
    app: &str,
    presets: &[PdfPreset],
    preset: &str,
    settings: &mut PdfSettings,
    naming: &mut Option<String>,
) -> (Option<String>, bool) {
    let mut chosen = None;
    let mut saved = false;
    FormGrid::new("pdf-top").show(ui, |form| {
        let mut current = preset.to_owned();
        let mut start_naming = false;
        form.row(&labels.preset, |ui, _| {
            let mut select = Select::new("pdf-preset", labels.preset.as_str(), &mut current);
            for preset in presets {
                select = select.option(preset.name.clone(), labels.preset_name(&preset.name, app));
            }
            let select = select.show(ui);
            let button = ui.add_enabled(naming.is_none(), Button::new(labels.save_preset.as_str())).on_hover_text(labels.save_preset_hint.as_str());
            start_naming = button.clicked();
            (select, button)
        });
        if current != preset {
            chosen = Some(current);
        }
        if start_naming {
            *naming = Some(labels.new_preset_name.clone());
        }
        if let Some(name) = naming.as_mut() {
            let mut done = None;
            form.row(&labels.save_as_preset, |ui, _| {
                let edit = TextField::new("pdf-preset-name", labels.save_as_preset.as_str(), name.as_str()).show(ui);
                if let Some(text) = edit.event.as_ref().and_then(EditEvent::value) {
                    name.clone_from(text);
                }
                let save = ui.add(Button::new(labels.name_save.as_str()));
                let cancel = ui.add(Button::new(labels.name_cancel.as_str()));
                if save.clicked() {
                    done = Some(true);
                } else if cancel.clicked() {
                    done = Some(false);
                }
                (edit, save, cancel)
            });
            match done {
                Some(true) => saved = true,
                Some(false) => *naming = None,
                None => {}
            }
        }
        form.row(&labels.standard, |ui, _| {
            let mut select = Select::new("pdf-standard", labels.standard.as_str(), &mut settings.standard);
            for standard in <Standard as Choice>::ALL {
                select = select.option(*standard, labels.choice("standard", standard.id(), standard.label()));
            }
            select.show(ui)
        });
        form.row(&labels.compatibility, |ui, _| {
            let standard = settings.standard;
            let mut select = Select::new("pdf-compatibility", labels.compatibility.as_str(), &mut settings.compatibility);
            for version in <Compatibility as Choice>::ALL {
                select = select.option(*version, version.label()).option_enabled(standard.allows(*version));
            }
            select.show(ui)
        });
    });
    (chosen, saved)
}

impl StackDialog for PdfExportDialog {
    type Result = PdfExportResult;

    fn id(&self) -> DialogId {
        self.id
    }

    fn cancelled(&mut self, ctx: &Context) {
        self.forget_pick(ctx);
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<PdfExportResult>> {
        if let Some(path) = self.picked(ctx) {
            self.state.close();
            let artboards = if self.artboards.range_only && !self.artboards.range.trim().is_empty() {
                PdfArtboards::Range(self.artboards.range.trim().to_owned())
            } else {
                PdfArtboards::All
            };
            return Some(DialogResult::Ok(PdfExportResult { options: self.settings.clone(), path, artboards }));
        }
        let labels = PdfExportLabels::shared_ctx(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let tokens = crate::theme::current(ctx);
        let pane_gap = tokens.space.pad.panel;
        let dialog_width = super::super::large_dialog_width(ctx, &tokens);
        let list_width = dialog_width / 4.0;
        let content_width = dialog_width - list_width - pane_gap;
        let content_height = CONTENT_ROWS * tokens.size.control.md;
        let error_text = self.blocked(&labels);
        let base = self.presets().into_iter().find(|preset| preset.name == self.preset).map(|preset| preset.settings).unwrap_or_default();
        let changed = summary::changes(&labels, &self.settings, &base);
        let preset_text = labels.preset_name(&self.preset, &self.app);
        let warnings: Vec<String> = self.settings.warnings(&*self.catalog).iter().map(|warning| labels.warning_text(warning)).collect();
        let saving = self.picking.is_some();
        let standard_before = self.settings.standard;
        let (mut chosen, mut saved) = (None, false);
        let mut section = self.section;
        let id = self.id;
        let presets = self.presets();
        let Self { app, settings, preset, naming, profiles, flatteners, unit, document_bleed, artboards, switches, state, .. } = self;
        let shown = Dialog::with_id(id, labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(dialog_width))
            .footer(Footer::standard().ok(labels.save.as_str()).ok_enabled(!saving && error_text.is_none()))
            .show(ctx, state, &mut (), |ui, ()| {
                ui.vertical(|ui| {
                    (chosen, saved) = top_rows(ui, &labels, app, &presets, preset, settings, naming);
                    ui.add_space(tokens.space.gap.section);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(list_width);
                            section = section_list(ui, section, &labels, tokens.size.control.md);
                        });
                        ui.add_space(pane_gap);
                        ui.vertical(|ui| {
                            ui.set_width(content_width);
                            ui.add(Text::new(section_title(&labels, section)).role(TypeRole::Title));
                            ui.add_space(tokens.space.gap.row);
                            let mut sections = Sections {
                                settings,
                                labels: &labels,
                                numbers: &numbers,
                                unit: *unit,
                                profiles,
                                flatteners,
                                document_bleed: *document_bleed,
                                artboards,
                                switches,
                                warnings: &warnings,
                                changed: &changed,
                                preset_name: &preset_text,
                            };
                            egui::ScrollArea::vertical()
                                .id_salt(("pdf-section", section))
                                .max_height(content_height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    sections.show(ui, section);
                                });
                        });
                    });
                    if let Some(text) = &error_text {
                        ui.add_space(tokens.space.gap.row);
                        let ui_tokens = Tokens::of(ui);
                        let format = role_format(&ui_tokens, TypeRole::Caption, ui_tokens.color.status.bad.fg.to_egui());
                        ui.add(Label::new(LayoutJob::single_section(text.clone(), format)).selectable(false));
                    }
                });
            });
        self.section = section;
        if self.settings.standard != standard_before {
            self.standard_chosen();
        }
        if let Some(name) = chosen {
            self.apply_preset(&name);
        }
        if saved {
            self.save_naming();
        }
        match shown.outcome {
            Some(Outcome::Ok) => {
                if self.blocked(&labels).is_none() {
                    self.queue_save(ctx, &labels);
                }
                None
            }
            Some(Outcome::Cancel | Outcome::Close) => {
                self.forget_pick(ctx);
                Some(DialogResult::Cancel)
            }
            _ => None,
        }
    }
}

impl PdfExportDialog {
    /// Stores the typed name as a preset of the current settings (without the passwords).
    fn save_naming(&mut self) {
        let Some(name) = self.naming.take().map(|name| name.trim().to_owned()).filter(|name| !name.is_empty()) else { return };
        let mut settings = self.settings.clone();
        settings.clear_passwords();
        let preset = PdfPreset { name: name.clone(), description: String::new(), settings };
        match self.saved.iter_mut().find(|saved| saved.name.eq_ignore_ascii_case(&name)) {
            Some(existing) => *existing = preset.clone(),
            None => self.saved.push(preset.clone()),
        }
        self.preset = name;
        self.new_preset = Some(preset);
    }
}

fn section_title(labels: &PdfExportLabels, section: usize) -> &str {
    section_names(labels).get(section).copied().unwrap_or(labels.section_general.as_str())
}

fn section_names(labels: &PdfExportLabels) -> [&str; 7] {
    [
        &labels.section_general,
        &labels.section_compression,
        &labels.section_marks,
        &labels.section_output,
        &labels.section_advanced,
        &labels.section_security,
        &labels.section_summary,
    ]
}

/// The list of sections; returns the one selected.
fn section_list(ui: &mut Ui, current: usize, labels: &PdfExportLabels, row_height: f32) -> usize {
    let names = section_names(labels);
    let rows: Vec<RowInfo<usize>> = (0..names.len()).map(RowInfo::leaf).collect();
    let mut selection = Selection::only(current);
    let listed = List::new("pdf-sections", labels.sections.as_str(), rows.as_slice())
        .selection(&mut selection)
        .mode(SelectionMode::Single)
        .allow_empty(false)
        .row_height(row_height)
        .max_height(row_height * names.len() as f32)
        .show(ui, |at, key| ListRow::new(*key, names[at]));
    let mut section = current;
    for event in listed.events {
        if let ListEvent::Selection(change) = event
            && let Some(row) = change.row
        {
            section = row;
        }
    }
    section
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;
