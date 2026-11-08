//! `ExportAsDialog`: File, Export, Export As. The settings are on the left (the format list, its options, metadata,
//! colour profile and image size); the preview with the size estimate is on the right. The dialog never encodes: it
//! reports the revision of the settings it wants a preview for and shows the answer the app gives back.

use super::{
    ExportAsResult, ExportFormat, ExportFormatDefaults, ExportMetadata, ExportOptions, ExportPreview, ExportProfile, ExportSize, ExportWords,
    SizeEstimate, export_file_name, size_error_label,
};
use crate::choice::Checkbox;
use crate::contract::{
    ActivateOn, Bind, CollectionEvent, EditResponse, Entry, NoValue, Outcome, Props as _, Remember as _, SelectMode, Show as _, ShowCollection as _,
    ShowContainer as _, Sizing,
};
use crate::dialog::{Dialog, DialogOutput};
use crate::display::{Shade, Text};
use crate::form::{Field, SliderField};
use crate::layout_components::{Row, Stack};
use crate::list::{List, ListRow};
use crate::patterns::form_dialogs::{
    HostRequests, MAX_DOCUMENT_SIDE, PathKind, PathRequest, PreviewPane, PreviewState, PreviewView, ResetFooter, ResetPick, SizeError,
    resize_from_scale, resize_from_side,
};
use crate::pickers::Select;
use crate::runtime::env::{Env, Label, UnitKind, Units};
use crate::runtime::overlay::CloseReason;
use crate::sections::{DialogRole, FooterButton, Section};
use crate::style::TypeRole;
use crate::text::NumberField;
use craft_behavior::{DisplayOrder, ItemSource, PixelSize, Selection, SelectionChange};
use craft_theme::variant::Tone;
use egui::{Response, Ui};
use std::cell::RefCell;
use std::path::PathBuf;

/// The least and greatest scale, in percent.
const SCALE_RANGE: std::ops::RangeInclusive<f64> = 1.0..=1000.0;
/// The least and greatest quality.
const QUALITY_RANGE: std::ops::RangeInclusive<f64> = 1.0..=100.0;
/// The key of the save request the dialog files at Export.
const SAVE_KEY: &str = "export";
/// The footer button that accepts: the first one `ResetFooter::buttons` gives.
const ACCEPT: usize = 0;

/// What the dialog keeps while it is open.
#[derive(Clone)]
struct Live {
    options: ExportOptions,
    /// The options the dialog opened with, which Reset restores.
    opening: ExportOptions,
    /// The newest revision of the settings.
    revision: u64,
    /// The newest revision reported in `wanted_preview`.
    taken: u64,
    shown: Option<ExportPreview>,
    view: PreviewView,
    /// The size edit the last pass refused.
    error: Option<SizeError>,
}

impl Live {
    fn open(initial: Option<ExportOptions>, defaults: ExportFormatDefaults, formats: &[ExportFormat], source: PixelSize) -> Self {
        let options =
            initial.unwrap_or_else(|| ExportOptions { quality: defaults.jpeg_quality, lossless: defaults.webp_lossless, ..ExportOptions::default() });
        let options = options.fitted(formats, source);
        Live { opening: options.clone(), options, revision: 1, taken: 0, shown: None, view: PreviewView::fitting(), error: None }
    }

    /// Takes the app's answer when it is for the newest revision asked for or later, and not older than the one
    /// shown.
    fn adopt(&mut self, given: Option<&ExportPreview>) {
        let Some(given) = given else { return };
        let newest = self.shown.as_ref().map_or(0, |shown| shown.revision);
        if given.revision >= self.taken && given.revision >= newest {
            self.shown = Some(given.clone());
        }
    }
}

/// The dialog's state between passes.
#[derive(Clone, Default)]
struct Held(Option<Live>);

/// Whether the options differ from the ones the dialog opened with: the body writes it each pass and the footer reads
/// it, so Reset's enabled state follows the rows shown in the same pass.
#[derive(Clone, Copy, Default)]
struct Changed(bool);

/// What Export As reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExportAsOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The answer, the pass the dialog closed with the path the app gave.
    pub result: Option<ExportAsResult>,
    /// The revision and settings of a preview the app should make, when the settings changed since the last pass. The
    /// app encodes a proxy and gives the answer back with [`ExportAsDialog::preview`].
    pub wanted_preview: Option<(u64, ExportOptions)>,
    /// The Export button pressed this pass files a save request here; the app opens its file dialog for it with
    /// [`file_name`](ExportAsOutput::file_name) and the format's filter, then gives the picked path back with
    /// [`ExportAsDialog::picked`]. A cancelled pick leaves the dialog open.
    pub requests: HostRequests,
    /// The file name the save dialog suggests: the document's name and the chosen format's extension.
    pub file_name: String,
    /// The chosen format; `None` while the dialog is closed.
    pub format: Option<ExportFormat>,
    /// The settings after the pass.
    pub options: Option<ExportOptions>,
}

impl Outcome for ExportAsOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// File, Export, Export As: format, size, quality, metadata, colour profile, a preview and a size estimate the app
/// computes.
///
/// - The settings: the format list, then that format's rows (Lossless, Quality, Bits/Pixel, Transparency), Metadata,
///   Color Space with Embed Color Profile, and Image Size (Scale, Width and Height with Constrain Proportions).
/// - Keys: arrows move through the format list, Tab walks the fields, Enter is Export and Escape cancels. Holding Alt
///   turns Cancel into Reset, which restores the settings the dialog opened with. Export asks the app for a file
///   to write (`ExportAsOutput::requests`) and the dialog stays open until the app gives the path back.
/// - Previews: whenever the settings change, `wanted_preview` carries a new revision. The app encodes a proxy and
///   gives it back with `preview`; a size estimate shows at once ("≈ 1.2 MB"), and the app may follow with an exact
///   one for the same revision.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct ExportAsDialog<'a> {
    file_name: String,
    source: PixelSize,
    /// The formats the list offers.
    #[prop(into)]
    formats: Vec<ExportFormat>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// The settings the dialog opens with (the last export's, or a command's arguments).
    #[prop]
    initial: Option<ExportOptions>,
    /// The quality and WebP mode a format takes when the user switches to it.
    #[prop]
    defaults: ExportFormatDefaults,
    /// The app's preview and estimate.
    #[prop]
    preview: Option<ExportPreview>,
    /// The path the app's save dialog picked, once.
    #[prop]
    picked: Option<PathBuf>,
    /// The app's save dialog is open: Export waits.
    #[prop]
    saving: bool,
}

impl<'a> ExportAsDialog<'a> {
    /// The dialog for the document named `file_name` (no extension) of size `source`.
    pub fn new(file_name: impl Into<String>, source: PixelSize) -> Self {
        ExportAsDialog {
            file_name: file_name.into(),
            source,
            formats: ExportFormat::standard(),
            open: None,
            initial: None,
            defaults: ExportFormatDefaults::default(),
            preview: None,
            picked: None,
            saving: false,
        }
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> ExportAsOutput {
        let ExportAsDialog { file_name, source, formats, open, initial, defaults, preview, picked, saving } = self;
        let env = Env::of(ui);
        let words = env.words::<ExportWords>();
        let units = env.units();

        let reset = ResetFooter::new(ui, None, true);
        let accept = FooterButton::new(words.export.clone(), DialogRole::Accept).stays_open(true).enabled(!saving);
        let resetting = reset.resetting();
        let mut dialog = Dialog::new(words.title.clone())
            .collection_body()
            .buttons(reset.buttons(accept, false))
            .settle_buttons({
                let reset = reset.clone();
                move |cx, buttons| reset.settle(buttons, cx.memory::<Changed>().0)
            })
            .close_guard(move |index, cx| {
                if resetting
                    && index != ACCEPT
                    && let Some(live) = cx.memory::<Held>().0.as_mut()
                {
                    live.options = live.opening.clone();
                    live.revision += 1;
                    live.error = None;
                }
                true
            });
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }

        let last = RefCell::new(None::<Live>);
        let wanted = RefCell::new(None::<(u64, ExportOptions)>);
        let pass = Pass { formats: &formats, source, words: &words, units: &units, defaults };
        let shown = dialog.show(ui, |ui, cx| {
            let mut live = cx.memory::<Held>().0.clone().unwrap_or_else(|| Live::open(initial.clone(), defaults, &formats, source));
            live.adopt(preview.as_ref());
            let before = live.options.clone();
            Row::new().show(ui, |ui, _| {
                Stack::new().show(ui, |ui, _| pass.settings(ui, &mut live));
                Stack::new().show(ui, |ui, _| pass.preview(ui, &mut live));
            });
            if live.options != before {
                live.revision += 1;
            }
            if live.revision != live.taken {
                live.taken = live.revision;
                *wanted.borrow_mut() = Some((live.revision, live.options.clone()));
            }
            cx.memory::<Changed>().0 = live.options != live.opening;
            if picked.is_some() {
                cx.request_close_as(CloseReason::Activated);
            }
            cx.memory::<Held>().0 = Some(live.clone());
            *last.borrow_mut() = Some(live);
        });

        let DialogOutput { response, closed, picked: pressed } = shown.output;
        let live = last.into_inner();
        let mut requests = HostRequests::new();
        if reset.pick(pressed) == Some(ResetPick::Accept) {
            requests.request_path(PathRequest::new(SAVE_KEY, PathKind::SaveFile));
        }
        let format = live.as_ref().and_then(|live| formats.iter().find(|format| format.id == live.options.format)).cloned();
        let extension = format.as_ref().map_or("png", |format| format.extension);
        let file_name = format!("{}.{extension}", export_file_name(&file_name, words.untitled.as_str()));
        let result = match (closed, picked, &live) {
            (Some(CloseReason::Activated | CloseReason::Enter), Some(path), Some(live)) => {
                Some(ExportAsResult { options: live.options.clone(), size: live.options.output_size(source), path })
            }
            _ => None,
        };
        ExportAsOutput {
            response,
            closed,
            result,
            wanted_preview: wanted.into_inner(),
            requests,
            file_name,
            format,
            options: live.map(|live| live.options),
        }
    }
}

/// What the rows read besides the dialog's state.
struct Pass<'p> {
    formats: &'p [ExportFormat],
    source: PixelSize,
    words: &'p ExportWords,
    units: &'p Units,
    defaults: ExportFormatDefaults,
}

/// The format list's rows, keyed by position.
struct FormatRows<'a>(&'a [ExportFormat]);

impl DisplayOrder<usize> for FormatRows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        self.0.get(at).map(|format| std::borrow::Cow::Borrowed(format.name))
    }
}

impl ItemSource<usize> for FormatRows<'_> {}

/// A size edit this pass.
enum Edit {
    Scale(f64),
    /// The typed side's value, and whether it is the width.
    Side(f64, bool),
}

/// The value a control's event carries.
fn edited<T: Clone>(response: &EditResponse<T>) -> Option<T> {
    response.event.as_ref().and_then(|event| event.value().cloned())
}

impl Pass<'_> {
    fn settings(&self, ui: &mut Ui, live: &mut Live) {
        let words = self.words;
        let chosen = self.formats.iter().position(|format| format.id == live.options.format).unwrap_or(0);
        let listed = List::<usize>::new(words.formats.clone())
            .select(SelectMode::Single)
            .activate(ActivateOn::Click)
            .allow_empty(false)
            .header(false)
            .sizing(Sizing::Content)
            .selection(Bind::given(Selection::single(chosen)))
            .show_items(ui, &FormatRows(self.formats), &mut |key, _| ListRow::new(self.formats[*key].name));
        let mut now = chosen;
        for event in listed.events {
            if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event
                && let Some(key) = keys.into_iter().last()
            {
                now = key;
            }
        }
        if now != chosen {
            self.switch(&mut live.options, now);
        }
        let Some(format) = self.formats.get(now) else { return };

        let options = &mut live.options;
        if format.lossless {
            check(ui, &words.lossless, "lossless", &mut options.lossless);
        }
        if format.quality && !(format.lossless && options.lossless) {
            row(ui, &words.quality, "quality", |ui| {
                let field = SliderField::new(words.quality.clone(), Bind::given(f64::from(options.quality)), QUALITY_RANGE).step(1.0).show(ui);
                if let Some(value) = edited(&field) {
                    options.quality = value.round().clamp(*QUALITY_RANGE.start(), *QUALITY_RANGE.end()) as u8;
                }
            });
        }
        if !format.bits.is_empty() {
            let entries: Vec<Entry<u8>> = std::iter::once(Entry::new(0, words.auto.clone()))
                .chain(format.bits.iter().map(|bits| Entry::new(*bits, bits.to_string())))
                .collect();
            choose(ui, &words.bits, "bits", &entries, &mut options.bits);
        }
        if format.transparency {
            check(ui, &words.transparency, "transparency", &mut options.transparency);
        }
        let metadata = [
            Entry::new(ExportMetadata::None, words.metadata_none.clone()),
            Entry::new(ExportMetadata::Exif, words.metadata_exif.clone()),
            Entry::new(ExportMetadata::All, words.metadata_all.clone()),
        ];
        choose(ui, &words.metadata, "metadata", &metadata, &mut options.metadata);
        let profiles =
            [Entry::new(ExportProfile::Document, words.document_profile.clone()), Entry::new(ExportProfile::Srgb, words.convert_srgb.clone())];
        choose(ui, &words.color_space, "profile", &profiles, &mut options.profile);
        check(ui, &words.embed_profile, "embed", &mut options.embed_profile);

        Section::fixed(words.image_size.clone()).id_salt("size").show(ui, |ui, _| self.size(ui, live, format));
    }

    /// The format list picked `index`: the format takes the app's default quality (and WebP mode).
    fn switch(&self, options: &mut ExportOptions, index: usize) {
        let Some(format) = self.formats.get(index) else { return };
        options.format = format.id.to_owned();
        let clamp = |quality: u8| quality.clamp(*QUALITY_RANGE.start() as u8, *QUALITY_RANGE.end() as u8);
        match format.id {
            "jpg" => options.quality = clamp(self.defaults.jpeg_quality),
            "webp" => {
                options.quality = clamp(self.defaults.webp_quality);
                options.lossless = self.defaults.webp_lossless;
            }
            _ => {}
        }
    }

    /// Scale, Width, Height, the proportion lock and the notes under them.
    fn size(&self, ui: &mut Ui, live: &mut Live, format: &ExportFormat) {
        let words = self.words;
        let source = self.source;
        let size = live.options.output_size(source);
        let scale = match live.options.size {
            ExportSize::Scale(scale) => scale,
            ExportSize::Pixels { width, .. } => f64::from(width) / f64::from(source.width.max(1)) * 100.0,
        };
        let mut edit = None;
        row(ui, &words.scale, "scale", |ui| {
            let field = NumberField::new(words.scale.clone(), Bind::given(Some(scale.round())))
                .unit_kind(UnitKind::Percent)
                .step(UnitKind::Percent.step())
                .range(SCALE_RANGE)
                .show(ui);
            if let Some(value) = edited(&field.edit).flatten() {
                edit = Some(Edit::Scale(value));
            }
        });
        for (salt, name, value, is_width) in [("width", &words.width, size.width, true), ("height", &words.height, size.height, false)] {
            row(ui, name, salt, |ui| {
                let field = NumberField::new(name.clone(), Bind::given(Some(f64::from(value))))
                    .unit_kind(UnitKind::Pixels)
                    .step(UnitKind::Pixels.step())
                    .range(1.0..=MAX_DOCUMENT_SIDE)
                    .show(ui);
                if let Some(value) = edited(&field.edit).flatten() {
                    edit = Some(Edit::Side(value, is_width));
                }
            });
        }
        let mut lock = live.options.lock_aspect;
        check(ui, &words.constrain, "constrain", &mut lock);
        if lock != live.options.lock_aspect {
            live.options.lock_aspect = lock;
            if lock {
                edit = Some(Edit::Side(f64::from(size.width), true));
            }
        }
        if let Some(edit) = edit {
            let result = match edit {
                Edit::Scale(scale) => resize_from_scale(source, scale).map(|_| ExportSize::Scale(scale)),
                Edit::Side(value, is_width) => {
                    let (width, height) = if is_width { (value, f64::from(size.height)) } else { (f64::from(size.width), value) };
                    resize_from_side(source, width, height, live.options.lock_aspect, is_width)
                        .map(|(size, _)| ExportSize::pixels(size.width, size.height))
                }
            };
            match result {
                Ok(new) => {
                    live.options.size = new;
                    live.error = None;
                }
                Err(error) => live.error = Some(error),
            }
        }
        let size = live.options.output_size(source);
        let dimensions = words.dimensions.fill(&[
            ("width", self.units.number(f64::from(size.width), 0..=0).as_str()),
            ("height", self.units.number(f64::from(size.height), 0..=0).as_str()),
        ]);
        Text::new(dimensions).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
        if let Some(error) = live.error {
            Text::new(size_error_label(words, error)).role(TypeRole::Caption).tone(Tone::Bad).show(ui);
        }
        if let Some(limit) = format.max_side
            && (size.width > limit || size.height > limit)
        {
            Text::new(words.ico_limit.clone()).role(TypeRole::Caption).tone(Tone::Warn).show(ui);
        }
    }

    /// The preview pane with the format and size line under it.
    fn preview(&self, ui: &mut Ui, live: &mut Live) {
        let words = self.words;
        let state = live.shown.as_ref().map_or(PreviewState::Pending(None), |shown| shown.state.clone());
        let pane = PreviewPane::state(words.preview.clone(), state, &live.view).click_centres(true).show(ui);
        live.view = pane.view.unwrap_or(pane.shown);
        let name = self.formats.iter().find(|format| format.id == live.options.format).map_or("", |format| format.name).to_uppercase();
        let estimate = match live.shown.as_ref().and_then(|shown| shown.estimate) {
            Some(SizeEstimate::Proxy(bytes)) => words.estimate.fill(&[("size", self.units.file_size(bytes).as_str())]),
            Some(SizeEstimate::Exact(bytes)) => self.units.file_size(bytes),
            None => words.no_estimate.as_str().to_owned(),
        };
        Text::new(format!("{name}  {estimate}")).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
    }
}

/// Shows `body` in a field named `name`.
fn row(ui: &mut Ui, name: &Label, salt: &str, mut body: impl FnMut(&mut Ui)) {
    Field::new(name.clone()).id_salt(salt).show(ui, |ui, _| body(ui));
}

/// A check box named `name` over `value`.
fn check(ui: &mut Ui, name: &Label, salt: &str, value: &mut bool) {
    if let Some(checked) = edited(&Checkbox::new(Bind::given(*value)).label(name.clone()).id_salt(salt).show(ui)) {
        *value = checked;
    }
}

/// A select of `entries` in a field named `name`, over `value`.
fn choose<T: craft_behavior::Value>(ui: &mut Ui, name: &Label, salt: &str, entries: &[Entry<T>], value: &mut T) {
    row(ui, name, salt, |ui| {
        if let Some(picked) = edited(&Select::new(name.clone(), Bind::given(value.clone()), entries).show(ui)) {
            *value = picked;
        }
    });
}
