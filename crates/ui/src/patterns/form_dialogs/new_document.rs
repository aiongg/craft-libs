//! `NewDocumentDialog`: File > New. Category tabs over a grid of preset cards, a details column, saved presets and the
//! document settings the app creates the document from.

mod cards;
mod details;
mod presets;
#[cfg(feature = "testing")]
mod stories;

use super::{ColorRequest, HostRequests, LinkedSize, ResetFooter, ResetPick};
use crate::contract::{Bind, BodyCx, NoValue, Outcome, Remember as _, SaveKey, ShowContainer as _};
use crate::dialog::{Dialog, DialogId, DialogOutput, DialogResult, DialogStack, StackDialog, dialog_id};
use crate::layout_components::{Row, Stack};
use crate::runtime::env::{Env, Label, UnitKind, Units};
use crate::runtime::overlay::CloseReason;
use crate::sections::{DialogRole, FooterButton};
use craft_behavior::{EditEvent, GestureId, Mixed, PixelSize};
use craft_theme::Srgba;
use egui::{Response, Ui, Vec2};
use std::cell::RefCell;
use std::sync::Arc;

use cards::{Left, TabId};
use details::Right;
use presets::{LastOptions, Recents, SavedPresets};

pub use presets::{
    BackgroundContents, BitDepth, ColorMode, ColorProfiles, DocumentOptions, DocumentPreset, NameError, Orientation, PresetCategory, PresetWords,
    preset_name_error, push_recent, size_line,
};

/// The key of the colour request Custom makes.
pub const BACKGROUND_COLOR_KEY: &str = "newDocument.backgroundColor";
/// The key of the recent documents.
const RECENT_KEY: SaveKey = SaveKey::new("craft_ui.newDocument.recent");
/// The key of the saved presets.
const PRESETS_KEY: SaveKey = SaveKey::new("craft_ui.newDocument.presets");
/// The key of the settings of the last created document.
const LAST_KEY: SaveKey = SaveKey::new("craft_ui.newDocument.last");

/// The size the dialog opens at until the user resizes it: three columns of cards beside the details column.
const OPENS_AT: Vec2 = Vec2::new(800.0, 560.0);
/// The least the dialog can be resized to.
const LEAST: Vec2 = Vec2::new(560.0, 420.0);

craft_ui::words! {
    /// The words of New Document, beside the common OK and Cancel and the names of the built-in presets (`PresetWords`).
    pub struct NewDocumentWords {
        title: "newDocument.title" = "New Document";
        create: "newDocument.create" = "Create";
        close: "newDocument.close" = "Close";
        tabs: "newDocument.tabs" = "Preset Categories";
        cards: "newDocument.cards" = "Blank Document Presets";
        saved_list: "newDocument.savedList" = "Saved Presets";
        recent: "newDocument.recent" = "Recent";
        saved: "newDocument.saved" = "Saved";
        photo: "newDocument.photo" = "Photo";
        print: "newDocument.print" = "Print";
        art: "newDocument.art" = "Art & Illustration";
        web: "newDocument.web" = "Web";
        mobile: "newDocument.mobile" = "Mobile";
        film: "newDocument.film" = "Film & Video";
        clipboard: "newDocument.clipboard" = "Clipboard";
        custom: "newDocument.custom" = "Custom";
        untitled: "newDocument.untitled" = "Untitled-1";
        name: "newDocument.name" = "Name";
        width: "newDocument.width" = "Width";
        height: "newDocument.height" = "Height";
        width_unit: "newDocument.widthUnit" = "Width Unit";
        height_unit: "newDocument.heightUnit" = "Height Unit";
        resolution_unit: "newDocument.resolutionUnit" = "Resolution Unit";
        portrait: "newDocument.portrait" = "Portrait";
        landscape: "newDocument.landscape" = "Landscape";
        artboards: "newDocument.artboards" = "Artboards";
        resolution: "newDocument.resolution" = "Resolution";
        color_mode: "newDocument.colorMode" = "Color Mode";
        depth: "newDocument.depth" = "Depth";
        profile: "newDocument.profile" = "Color Profile";
        /// `{name}` is the working profile's name.
        working_profile: template "newDocument.workingProfile" = "Working: {name}";
        working: "newDocument.working" = "Working";
        background: "newDocument.background" = "Background Contents";
        white: "newDocument.white" = "White";
        black: "newDocument.black" = "Black";
        background_color: "newDocument.backgroundColor" = "Background Color";
        transparent: "newDocument.transparent" = "Transparent";
        custom_color: "newDocument.customColor" = "Custom…";
        custom_swatch: "newDocument.customSwatch" = "Custom background color";
        save_preset: "newDocument.savePreset" = "Save Preset…";
        preset_name: "newDocument.presetName" = "Preset Name";
        save: "newDocument.save" = "Save";
        delete: "newDocument.delete" = "Delete";
        no_saved: "newDocument.noSaved" = "No saved presets yet.";
        name_length: "newDocument.nameLength" = "Preset names must contain 1–255 characters after trimming";
        /// `{name}` is the name typed.
        name_taken: template "newDocument.nameTaken" = "A preset named \"{name}\" already exists; choose another name";
        too_many: "newDocument.tooMany" = "At most 256 document presets can be saved";
        grayscale: "newDocument.grayscale" = "Grayscale";
        rgb_color: "newDocument.rgbColor" = "RGB Color";
        cmyk_color: "newDocument.cmykColor" = "CMYK Color";
        lab_color: "newDocument.labColor" = "Lab Color";
        depth_8: "newDocument.depth8" = "8 bit";
        depth_16: "newDocument.depth16" = "16 bit";
        depth_32: "newDocument.depth32" = "32 bit (float)";
        orientation: "newDocument.orientation" = "Orientation";
        /// `{n}` is how many cards the open tab lists.
        presets_heading: template "newDocument.presetsHeading" = "Blank Document Presets ({n})";
        preset_details: "newDocument.presetDetails" = "Preset Details";
        default_size: "newDocument.defaultSizePlain" = "Default Size";
        /// `{app}` is the application's name.
        default_size_named: template "newDocument.defaultSize" = "Default {app} Size";
        /// A card's size line: `{width}`, `{height}`, `{unit}` and `{resolution}`.
        size_line: template "newDocument.sizeLine" = "{width} × {height} {unit} @ {resolution}";
        /// A recent card's hover text: `{title}`, `{size}`, `{mode}` and `{background}`.
        recent_detail: template "newDocument.recentDetail" = "{title} · {size} · {mode} · {background}";
        /// A saved preset's second line: `{size}`, `{mode}` and `{depth}`.
        saved_detail: template "newDocument.savedDetail" = "{size} · {mode}, {depth}";
    }
}

/// What the dialog is set up with: the cards, the clipboard, the profiles and the resolutions.
struct Setup {
    categories: Vec<PresetCategory>,
    recent_defaults: Vec<DocumentPreset>,
    clipboard: Option<PixelSize>,
    background: [f32; 3],
    print_ppi: f64,
    screen_ppi: f64,
    profiles: Vec<ColorProfiles>,
    app_name: Option<Label>,
}

/// What the columns read besides the session: the setup, the remembered lists, the words and the units.
#[derive(Clone, Copy)]
struct Read<'a> {
    setup: &'a Setup,
    recents: &'a [DocumentOptions],
    saved: &'a [DocumentOptions],
    words: &'a NewDocumentWords,
    preset_words: &'a PresetWords,
    units: &'a Units,
}

/// What the dialog keeps while it is open: the settings the details column shows, the open tab and card, and the
/// state of the Save Preset row. It lives in the dialog's body slot.
#[derive(Clone)]
struct Session {
    seeded: bool,
    settings: DocumentOptions,
    /// The width, height and resolution fields of `settings`.
    size: LinkedSize,
    /// The size before a running edit of a size field, for putting it back when the edit is cancelled.
    gesture: Option<LinkedSize>,
    /// The settings the dialog opened with, which Reset restores.
    opening: Option<DocumentOptions>,
    /// The settings last written to the dialog's `options` binding.
    written: Option<DocumentOptions>,
    tab: TabId,
    /// The card the user picked in the open tab (the saved row in Saved).
    card: Option<usize>,
    /// The orientation picked with the control; built-in cards turn with it.
    orientation: Option<Orientation>,
    /// The Save Preset name row, when open.
    naming: Option<String>,
    /// A save was refused and its reason shows under the name field.
    name_tried: bool,
    focus_name: bool,
    focus_preset_name: bool,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            seeded: false,
            settings: DocumentOptions::default(),
            size: LinkedSize::new(PixelSize::new(1, 1)),
            gesture: None,
            opening: None,
            written: None,
            tab: TabId::Recent,
            card: None,
            orientation: None,
            naming: None,
            name_tried: false,
            focus_name: true,
            focus_preset_name: false,
        }
    }
}

impl Session {
    /// Whether the settings differ from the ones the dialog opened with.
    fn changed(&self) -> bool {
        self.opening.as_ref().is_some_and(|opening| *opening != self.settings)
    }

    /// Puts back the settings the dialog opened with.
    fn reset(&mut self) {
        if let Some(opening) = self.opening.clone() {
            self.size = opening.linked();
            self.settings = opening;
        }
        self.card = None;
        self.orientation = None;
        self.naming = None;
        self.name_tried = false;
        self.gesture = None;
    }

    /// Takes the settings from outside the dialog.
    fn adopt(&mut self, settings: DocumentOptions) {
        if settings.size != self.settings.size
            || settings.pixels_per_inch != self.settings.pixels_per_inch
            || settings.unit != self.settings.unit
            || settings.resolution_unit != self.settings.resolution_unit
        {
            self.size = settings.linked();
        }
        if !self.settings.same_document(&settings) {
            self.card = None;
        }
        self.settings = settings;
    }

    /// Applies a card: a recent document or saved preset restores everything but the name, a built-in card sets the
    /// size and unit.
    fn apply_card(&mut self, card: &cards::Card) {
        if card.restores_all {
            let name = std::mem::take(&mut self.settings.name);
            self.settings = DocumentOptions { name, ..card.settings.clone() };
        } else {
            self.settings.size = card.settings.size;
            self.settings.pixels_per_inch = card.settings.pixels_per_inch;
            self.settings.unit = card.settings.unit;
        }
        self.size = self.settings.linked();
        self.gesture = None;
    }
}

/// The settings a Create makes: the name trimmed, "Untitled" when empty. It stores the last values and the newest recent
/// entry.
fn create(cx: &mut BodyCx<'_>, mut settings: DocumentOptions, untitled: &str) -> DocumentOptions {
    settings.name = settings.name.trim().to_owned();
    if settings.name.is_empty() {
        untitled.clone_into(&mut settings.name);
    }
    let mut remembered = settings.clone();
    remembered.name.clear();
    push_recent(&mut cx.saved::<Recents>(&RECENT_KEY).0, remembered.clone(), presets::MAX_RECENT);
    cx.saved::<LastOptions>(&LAST_KEY).0 = Some(remembered);
    settings
}

/// Reads what the dialog remembers on its first pass: the last settings (not with a clipboard image, which opens the
/// dialog on its own card).
fn seed(session: &mut Session, cx: &mut BodyCx<'_>, setup: &Setup, untitled: &str) {
    let mut settings = DocumentOptions::default();
    match setup.clipboard {
        Some(size) => {
            settings.size = size;
            settings.unit = UnitKind::Pixels;
            settings.pixels_per_inch = setup.screen_ppi;
            session.card = Some(0);
        }
        None => {
            if let Some(last) = cx.saved::<LastOptions>(&LAST_KEY).0.clone().filter(DocumentOptions::fits) {
                settings = last;
            }
        }
    }
    untitled.clone_into(&mut settings.name);
    session.size = settings.linked();
    session.opening = Some(settings.clone());
    session.settings = settings;
    session.seeded = true;
}

fn mode_label(words: &NewDocumentWords, mode: ColorMode) -> &Label {
    match mode {
        ColorMode::Grayscale => &words.grayscale,
        ColorMode::Rgb => &words.rgb_color,
        ColorMode::Cmyk => &words.cmyk_color,
        ColorMode::Lab => &words.lab_color,
    }
}

fn depth_label(words: &NewDocumentWords, depth: BitDepth) -> &Label {
    match depth {
        BitDepth::Eight => &words.depth_8,
        BitDepth::Sixteen => &words.depth_16,
        BitDepth::ThirtyTwo => &words.depth_32,
    }
}

fn background_label(words: &NewDocumentWords, background: BackgroundContents) -> &Label {
    match background {
        BackgroundContents::White => &words.white,
        BackgroundContents::Black => &words.black,
        BackgroundContents::BackgroundColor => &words.background_color,
        BackgroundContents::Transparent => &words.transparent,
        BackgroundContents::Custom(_) => &words.custom_color,
    }
}

/// The colour the background shows as: `None` for a transparent one.
fn background_rgb(background: BackgroundContents, app_background: [f32; 3]) -> Option<[f32; 3]> {
    match background {
        BackgroundContents::White => Some([1.0; 3]),
        BackgroundContents::Black => Some([0.0; 3]),
        BackgroundContents::BackgroundColor => Some(app_background),
        BackgroundContents::Transparent => None,
        BackgroundContents::Custom(color) => Some(color),
    }
}

/// What the dialog reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NewDocumentOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The settings to create the document from, the pass the dialog closed as Create.
    pub result: Option<DocumentOptions>,
    /// The background colour the app's picker is asked for: Custom was chosen or the colour button pressed in this
    /// pass, keyed `BACKGROUND_COLOR_KEY`. The app answers by setting `BackgroundContents::Custom` in the dialog's
    /// `options`.
    pub pick_color: Option<ColorRequest>,
}

impl Outcome for NewDocumentOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// What one pass reports besides the dialog's own response.
#[derive(Default)]
struct Pass {
    created: Option<DocumentOptions>,
    request: Option<ColorRequest>,
}

/// File > New: category tabs over a grid of preset cards three wide, a details column and Create and Close.
///
/// - Tabs: Recent, the categories and Saved. Recent lists the clipboard's image size first when the app gives one,
///   then the recent documents newest first (up to 20), then the built-in defaults not yet among them. Saved lists the
///   user's presets, with Delete under the details and "No saved presets yet." when there are none.
/// - Cards: a click or an arrow fills the details; Enter on a card or a double click creates. A built-in card turns with
///   the orientation control. Saved and recent cards restore every setting but the name.
/// - Details: Name, Width and Height with their units, Orientation, Artboards, Resolution, Color Mode, Depth, Color
///   Profile and Background Contents. Save Preset opens a name row under the column; Enter there saves the preset and
///   does not create the document. A name is trimmed and must have 1 to 255 characters, differ from the saved names
///   (ignoring ASCII case), and at most 256 presets are kept.
/// - Keys: Tab walks the fields, Enter is Create unless a field owns it, Escape closes. Holding Alt turns Close into
///   Reset, which restores the settings from when the dialog opened and keeps it open. Ctrl+Z inside the dialog steps
///   the dialog's own settings.
/// - Custom under Background Contents, or the colour button, asks the app for a colour: the output's `pick_color`
///   carries the request for one pass, and the app answers by setting `BackgroundContents::Custom` in the `options`
///   value.
/// - Memory: the last created settings, the recent documents and the saved presets are kept across runs.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct NewDocumentDialog<'a> {
    /// Binds the settings the details column shows to `options`: a change from outside (the picked colour) reaches the
    /// dialog, and the dialog's edits are written back.
    #[prop(name = "options")]
    bound: Option<Bind<'a, DocumentOptions>>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// The categories in tab order, in place of `PresetCategory::standard()`.
    #[prop]
    categories: Vec<PresetCategory>,
    /// The cards Recent shows after the recent documents, in place of `PresetCategory::default_recent()`. A card with
    /// the id `"default"` is named "Default {app} Size".
    #[prop]
    recent_defaults: Vec<DocumentPreset>,
    /// The clipboard holds an image of this size: Recent shows a Clipboard card first and the dialog opens on it.
    #[prop]
    clipboard: Option<PixelSize>,
    /// The app's background colour, offered as the Background Color choice.
    #[prop]
    background_color: Option<Srgba>,
    /// The resolution print presets open at, in pixels per inch.
    #[prop(quantity = "resolution in pixels per inch")]
    print_resolution: f64,
    /// The resolution screen presets open at, in pixels per inch.
    #[prop(quantity = "resolution in pixels per inch")]
    screen_resolution: f64,
    /// The colour profiles each mode offers.
    #[prop]
    profiles: Vec<ColorProfiles>,
    /// The application's name, for the "Default {app} Size" card.
    #[prop(into)]
    app_name: Option<Label>,
    /// Keeps the dialog's place and size across runs under `key`.
    #[prop]
    remember: Option<SaveKey>,
}

impl NewDocumentDialog<'_> {
    /// The dialog with `PresetCategory::standard()`, print resolution 300 and screen resolution 72.
    pub fn new() -> Self {
        NewDocumentDialog {
            bound: None,
            open: None,
            categories: PresetCategory::standard(),
            recent_defaults: PresetCategory::default_recent(),
            clipboard: None,
            background_color: None,
            print_resolution: 300.0,
            screen_resolution: 72.0,
            profiles: Vec::new(),
            app_name: None,
            remember: None,
        }
    }
}

impl Default for NewDocumentDialog<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> NewDocumentDialog<'a> {
    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> NewDocumentOutput {
        let NewDocumentDialog {
            mut bound,
            open,
            categories,
            recent_defaults,
            clipboard,
            background_color,
            print_resolution,
            screen_resolution,
            profiles,
            app_name,
            remember,
        } = self;
        let env = Env::of(ui);
        let words = env.words::<NewDocumentWords>();
        let preset_words = env.words::<PresetWords>();
        let units = env.units();
        let setup = Setup {
            categories,
            recent_defaults,
            clipboard,
            background: background_color.map_or([1.0; 3], |color| color.to_rgb_array()),
            print_ppi: print_resolution,
            screen_ppi: screen_resolution,
            profiles,
            app_name,
        };
        let title: Arc<str> = words.title.as_str().into();
        let untitled = words.untitled.as_str().to_owned();

        let reset = ResetFooter::new(ui, Some(words.close.clone()), true);
        let accept = FooterButton::new(words.create.clone(), DialogRole::Accept);
        let pass = RefCell::new(Pass::default());
        let mut dialog = Dialog::new(words.title.clone())
            .collection_body()
            .resizable(true)
            .overlay(|overlay| overlay.size(OPENS_AT).min_size(LEAST))
            .outer_gesture(true)
            .close_on_enter(true)
            .buttons(reset.buttons(accept, false))
            .settle_buttons({
                let reset = reset.clone();
                move |cx, buttons| {
                    let changed = cx.memory::<Session>().changed();
                    reset.settle(buttons, changed);
                }
            })
            .close_guard(|at, cx| {
                match reset.pick(Some(at)) {
                    Some(ResetPick::Accept) => {
                        let settings = cx.memory::<Session>().settings.clone();
                        pass.borrow_mut().created = Some(create(cx, settings, &untitled));
                    }
                    Some(ResetPick::Reset) => cx.memory::<Session>().reset(),
                    _ => {}
                }
                true
            });
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }
        if let Some(key) = remember {
            dialog = dialog.remember(key);
        }

        let shown = dialog.show(ui, |ui, cx| {
            let mut session = cx.memory::<Session>().clone();
            if !session.seeded {
                seed(&mut session, cx, &setup, &untitled);
            }
            let held = bound.as_mut().and_then(|bind| bind.reborrow().read(cx)).and_then(|held| held.same().cloned());
            if let Some(held) = &held
                && session.written.as_ref().is_some_and(|written| written != held)
            {
                session.adopt(held.clone());
                session.written = Some(held.clone());
            }
            let mut recents = cx.saved::<Recents>(&RECENT_KEY).0.clone();
            recents.retain(DocumentOptions::fits);
            let mut saved = cx.saved::<SavedPresets>(&PRESETS_KEY).0.clone();
            saved.retain(DocumentOptions::fits);

            let read = Read { setup: &setup, recents: &recents, saved: &saved, words: &words, preset_words: &preset_words, units: &units };
            let mut left = Left::default();
            let mut right = Right::default();
            Row::new().fill().show(ui, |ui, _| {
                Stack::new().fill().show(ui, |ui, _| {
                    cards::show(ui, &session, &read, &mut left);
                });
                Stack::new().show(ui, |ui, _| {
                    details::show(ui, &mut session, &read, &mut right);
                });
            });

            if let Some(tab) = left.tab.filter(|tab| *tab != session.tab) {
                session.tab = tab;
                session.card = None;
            }
            if let Some(picked) = left.picked {
                session.apply_card(&picked.card);
                session.card = Some(picked.index);
                if picked.activated {
                    pass.borrow_mut().created = Some(create(cx, session.settings.clone(), &untitled));
                    cx.request_close_as(CloseReason::Activated);
                }
            }
            if right.create {
                pass.borrow_mut().created = Some(create(cx, session.settings.clone(), &untitled));
                cx.request_close_as(CloseReason::Activated);
            }
            if right.request_color {
                let current = background_rgb(session.settings.background, setup.background).unwrap_or([1.0; 3]);
                pass.borrow_mut().request =
                    Some(ColorRequest::new(BACKGROUND_COLOR_KEY, Srgba::from_rgb_array(current)).title(words.custom_swatch.clone()));
            }
            if right.delete
                && let Some(at) = session.card.filter(|_| session.tab == TabId::Saved)
            {
                let kept = &mut cx.saved::<SavedPresets>(&PRESETS_KEY).0;
                if at < kept.len() {
                    kept.remove(at);
                }
                session.card = None;
            }
            if right.save {
                let name = session.naming.clone().unwrap_or_default();
                if preset_name_error(&saved, &name).is_none() {
                    let preset = DocumentOptions { name: name.trim().to_owned(), ..session.settings.clone() };
                    let kept = &mut cx.saved::<SavedPresets>(&PRESETS_KEY).0;
                    kept.push(preset);
                    session.tab = TabId::Saved;
                    session.card = Some(kept.len() - 1);
                    session.naming = None;
                    session.name_tried = false;
                } else {
                    session.name_tried = true;
                }
            }

            if let Some(bind) = bound.as_mut()
                && session.written.as_ref() != Some(&session.settings)
            {
                let before = held.unwrap_or_default();
                let event = EditEvent::set(GestureId::default(), Mixed::Same(before), session.settings.clone(), Arc::clone(&title), None);
                bind.reborrow().write(cx, &event);
                session.written = Some(session.settings.clone());
            }
            *cx.memory::<Session>() = session;
        });

        let DialogOutput { response, closed, .. } = shown.output;
        let Pass { created, request } = pass.into_inner();
        let result = created.filter(|_| matches!(closed, Some(CloseReason::Enter | CloseReason::Activated)));
        NewDocumentOutput { response, closed, result, pick_color: request }
    }
}

type Build = dyn for<'b> Fn(NewDocumentDialog<'b>) -> NewDocumentDialog<'b> + Send;

/// A [`NewDocumentDialog`] the [`DialogHost`](crate::dialog::DialogHost) owns: the host keeps the settings and the
/// colour request, and `build` sets the dialog up each pass.
///
/// ```ignore
/// host.open(HostedNewDocument::new(|dialog| dialog.clipboard(size).app_name("PhotoCraft")));
/// // each pass, for a colour request:
/// if let Some(dialog) = host.get_mut::<HostedNewDocument>() {
///     if let Some(request) = dialog.pick_color() { /* open the colour picker, then */ dialog.set_color(&request.key, color); }
/// }
/// ```
pub struct HostedNewDocument {
    id: DialogId,
    options: DocumentOptions,
    build: Box<Build>,
    requests: HostRequests,
}

impl HostedNewDocument {
    /// A hosted dialog; `build` sets the dialog up (the clipboard, the profiles, the app's name).
    pub fn new(build: impl for<'b> Fn(NewDocumentDialog<'b>) -> NewDocumentDialog<'b> + Send + 'static) -> Self {
        HostedNewDocument {
            id: dialog_id("new-document"),
            options: DocumentOptions::default(),
            build: Box::new(build),
            requests: HostRequests::new(),
        }
    }

    /// The settings the details column shows.
    pub fn options(&self) -> &DocumentOptions {
        &self.options
    }

    /// The colour the dialog asks the app's picker for, once: Custom was chosen or the colour button pressed.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.requests.take_color()
    }

    /// The colour the app's picker returned for `key`: Background Contents becomes `BackgroundContents::Custom`. Other
    /// keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key == BACKGROUND_COLOR_KEY {
            self.options.background = BackgroundContents::Custom(color.to_rgb_array());
        }
    }

    /// The picker was cancelled: the request is dropped and Background Contents stays as it was.
    pub fn cancel_color(&mut self) {
        self.requests.cancel();
    }
}

impl StackDialog for HostedNewDocument {
    type Result = DocumentOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<DocumentOptions>> {
        let shown = (self.build)(NewDocumentDialog::new().options(Bind::bound(&mut self.options))).show(ui);
        if let Some(request) = shown.pick_color {
            self.requests.request_color(request);
        }
        match (shown.result, shown.closed) {
            (Some(result), _) => Some(DialogResult::Ok(result)),
            (None, Some(_)) => {
                self.requests.cancel();
                Some(DialogResult::Cancel)
            }
            (None, None) => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some("new-document")
    }

    fn cancelled(&mut self, _: &egui::Context) {
        self.requests.cancel();
    }
}
