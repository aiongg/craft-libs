//! Stories of the Crop tool on the canvas tool host (task ST-TO1; task RG2 registers them and
//! their components).
//!
//! Every story runs a `CropTool` through `ToolHost` on the story canvas (`craft_ui::testing::
//! canvas_story`) over a small picture the story paints, and keeps the tool, the settings and the
//! view in egui memory. The story plays the app: it applies the tool's `Settings` events, answers
//! `Apply` with `applied(true)`, and routes the bar's events to the tool as an app does. Sizes are
//! args with defaults; the strings are story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/crop-tool/default-frame` | The tool picked on a fake page with and without a selection; a drag inside draws; a "Pick another tool" button leaves and applies |
//! | `primitives/crop-tool/image-mode` | Image mode and Classic Mode side by side with the view turning |
//! | `primitives/crop-tool/ratio-and-size` | Ratio presets and W x H x Resolution, with the resulting `CropTarget` in a status line |
//! | `primitives/crop-tool/straighten` | The armed button and the temporary Command line |
//! | `primitives/crop-tool/box-undo` | Edit › Undo and Redo buttons stepping the box |

use egui::{Color32, Painter, Shape, Stroke, Ui};
use kurbo::{Point, Rect};

use super::{
    CropBarEvent, CropBarOutput, CropDocKey, CropDocument, CropEvent, CropInput, CropLabels, CropNames, CropOptionsBar, CropRatio, CropSettings,
    CropSizeFields, CropTool, CropUserPreset,
};
use craft_ui::canvas::{CanvasView, RulerUnits, ToolHost};
use craft_ui::controls::Button;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryLabels, Surface};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document_with};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Crop tool"];

craft_ui::words! {
    /// The buttons, status lines and sample names of the Crop stories, English by default.
    #[non_exhaustive]
    pub(crate) struct CropStoryLabels {
        postcard: "cropStory.postcard" = "Postcard";
        banner: "cropStory.banner" = "Banner";
        apply: "cropStory.apply" = "Apply {w} × {h} px at {angle}°";
        apply_to: "cropStory.applyTo" = "Apply {w} × {h} px at {angle}° to {target_w} × {target_h} px";
        discard: "cropStory.discard" = "Discard";
        straightened: "cropStory.straightened" = "Straighten {angle}°";
        new_preset_named: "cropStory.newPresetNamed" = "New Crop Preset: {name}";
        new_preset: "cropStory.newPreset" = "New Crop Preset";
        delete_preset: "cropStory.deletePreset" = "Delete Crop Preset";
        shield_requested: "cropStory.shieldRequested" = "Crop Shield color requested";
        ratio: "cropStory.ratio" = "Ratio";
        ratio_editing: "cropStory.ratioEditing" = "Ratio (editing)";
        straighten: "cropStory.straighten" = "Straighten";
        commit: "cropStory.commit" = "Commit";
        cancel: "cropStory.cancel" = "Cancel";
        left_tool: "cropStory.leftTool" = "Left the tool";
        pick_other: "cropStory.pickOther" = "Pick another tool";
        pick_crop: "cropStory.pickCrop" = "Pick Crop tool";
        undo: "cropStory.undo" = "Undo";
        redo: "cropStory.redo" = "Redo";
        classic_mode: "cropStory.classicMode" = "Classic Mode";
        image_mode: "cropStory.imageMode" = "Image mode";
        mode_line: "cropStory.modeLine" = "{mode} · {frame} · view {angle}°";
        target: "cropStory.target" = "Target {w} × {h} px";
        target_ppi: "cropStory.targetPpi" = "Target {w} × {h} px at {ppi} ppi";
        free_crop: "cropStory.freeCrop" = "Free crop";
        straighten_hint: "cropStory.straightenHint" = "Hold ⌘ (Ctrl) to draw a line without arming Straighten";
        orientation: "cropStory.orientation" = "orientation {n}";
        narrow: "cropStory.narrow" = "narrow";
        wide: "cropStory.wide" = "wide";
    }
}

/// The fake selection of the default-frame story, in document units.
const SELECTION: Rect = Rect::new(160.0, 100.0, 480.0, 300.0);

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/crop-tool/default-frame",
            Level::Primitives,
            "Crop tool",
            "Default frame",
            DefaultArgs { view: ViewArgs::default(), selection: true },
            show_default_frame
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/crop-tool/image-mode",
            Level::Primitives,
            "Crop tool",
            "Image mode",
            ImageModeArgs { view: ViewArgs::default(), auto_center: true, shield: 75.0 },
            show_image_mode
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/crop-tool/ratio-and-size",
            Level::Primitives,
            "Crop tool",
            "Ratio and size",
            RatioArgs { view: ViewArgs::default(), resolution: 300.0 },
            show_ratio_and_size
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/crop-tool/straighten",
            Level::Primitives,
            "Crop tool",
            "Straighten",
            StraightenArgs { view: ViewArgs::default(), armed: true, tilt: 6.0 },
            show_straighten
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/crop-tool/box-undo",
            Level::Primitives,
            "Crop tool",
            "Box undo",
            UndoArgs { view: ViewArgs::default() },
            show_box_undo
        )
        .on(Surface::Canvas),
    ]
}

/// Two saved crop presets, for the bar's list and its New and Delete entries.
pub(super) fn presets(labels: &CropStoryLabels) -> Vec<CropUserPreset> {
    let size = |width: &str, height: &str| CropSizeFields {
        width: width.to_owned(),
        height: height.to_owned(),
        resolution: "300".to_owned(),
        ..CropSizeFields::default()
    };
    vec![
        CropUserPreset { name: labels.postcard.clone(), ratio: CropRatio::Size, size: size("6 in", "4 in") },
        CropUserPreset { name: labels.banner.clone(), ratio: CropRatio::Sides { width: 3.0, height: 1.0 }, size: CropSizeFields::default() },
    ]
}

// --- the app the stories play ---

/// What a pane keeps between passes.
#[derive(Clone)]
struct Kept {
    tool: CropTool,
    settings: CropSettings,
    /// The view the tool turned, until the view args change.
    view: Option<CanvasView>,
    view_args: ViewArgs,
    /// The tool is the picked tool: off after "Pick another tool".
    picked: bool,
    /// The app's last handling of an event, as the status line shows it.
    last: Option<String>,
}

impl Kept {
    fn new(classic: bool, view_args: ViewArgs) -> Self {
        let mut settings = CropSettings::default();
        settings.shield.classic_mode = classic;
        Self { tool: CropTool::new(), settings, view: None, view_args, picked: true, last: None }
    }
}

/// What a pane shows and runs on.
struct Pane {
    /// The canvas's accessible name.
    label: String,
    view: ViewArgs,
    key: CropDocKey,
    selection: Option<Rect>,
    resolution: f64,
    /// Degrees the picture's horizon is turned by.
    tilt: f64,
    /// The options bar above the canvas, with its straighten button.
    bar: bool,
}

impl Pane {
    fn new(label: String, view: ViewArgs) -> Self {
        Self { label, view, key: CropDocKey(1), selection: None, resolution: 300.0, tilt: 0.0, bar: false }
    }

    fn document(&self) -> CropDocument {
        CropDocument::new(self.key, DOC.size()).resolution(self.resolution).selection(self.selection)
    }
}

fn kept_key(ui: &Ui, name: &str) -> egui::Id {
    ui.make_persistent_id(("craft_draw::crop::tool_story", name))
}

fn load(ui: &Ui, name: &str, classic: bool, view: ViewArgs) -> Kept {
    let mut kept: Kept = ui.data(|d| d.get_temp(kept_key(ui, name))).unwrap_or_else(|| Kept::new(classic, view));
    if kept.view_args != view {
        kept.view_args = view;
        kept.view = None;
    }
    kept
}

fn store(ui: &Ui, name: &str, kept: Kept) {
    ui.data_mut(|d| d.insert_temp(kept_key(ui, name), kept));
}

/// The fake picture: sky, a sun and a ground whose horizon is turned by `tilt` degrees.
fn paint_picture(painter: &Painter, view: &CanvasView, tilt: f64) {
    let screen = |points: &[Point]| points.iter().map(|p| view.to_screen(*p)).collect::<Vec<_>>();
    painter.add(Shape::convex_polygon(view.quad(DOC).to_vec(), Color32::from_rgb(150, 200, 235), Stroke::NONE));
    let sun = (0..24)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::TAU / 24.0;
            Point::new(DOC.width() * 0.75 + a.cos() * DOC.height() * 0.1, DOC.height() * 0.22 + a.sin() * DOC.height() * 0.1)
        })
        .collect::<Vec<_>>();
    painter.add(Shape::convex_polygon(screen(&sun), Color32::from_rgb(250, 220, 110), Stroke::NONE));
    let rise = DOC.width() / 2.0 * tilt.to_radians().tan();
    let horizon = DOC.height() * 0.6;
    let ground = [Point::new(DOC.x0, horizon - rise), Point::new(DOC.x1, horizon + rise), Point::new(DOC.x1, DOC.y1), Point::new(DOC.x0, DOC.y1)];
    painter.add(Shape::convex_polygon(screen(&ground), Color32::from_rgb(96, 150, 90), Stroke::NONE));
    let hill = [Point::new(DOC.width() * 0.1, DOC.y1), Point::new(DOC.width() * 0.3, DOC.height() * 0.62), Point::new(DOC.width() * 0.55, DOC.y1)];
    painter.add(Shape::convex_polygon(screen(&hill), Color32::from_rgb(70, 120, 70), Stroke::NONE));
}

/// The event as the status line words it.
fn describe(numbers: &LocaleNumbers, labels: &CropStoryLabels, event: &CropEvent) -> Option<String> {
    let n = |v: f64| numbers.format(v, 0..=1);
    match event {
        CropEvent::Apply(commit) => {
            let rect = commit.frame.rect;
            let (w, h, angle) = (n(rect.width()), n(rect.height()), n(commit.frame.angle));
            Some(match commit.target {
                Some(target) => craft_i18n::fmt(
                    &labels.apply_to,
                    &[("w", &w), ("h", &h), ("angle", &angle), ("target_w", &n(target.width)), ("target_h", &n(target.height))],
                ),
                None => craft_i18n::fmt(&labels.apply, &[("w", &w), ("h", &h), ("angle", &angle)]),
            })
        }
        CropEvent::Discard => Some(labels.discard.clone()),
        CropEvent::Straightened(angle) => Some(craft_i18n::fmt(&labels.straightened, &[("angle", &n(*angle))])),
        CropEvent::Frame(_) | CropEvent::Settings(_) => None,
    }
}

/// Handles one event of the tool as an app does.
fn handle(kept: &mut Kept, numbers: &LocaleNumbers, labels: &CropStoryLabels, event: CropEvent) {
    if let Some(text) = describe(numbers, labels, &event) {
        kept.last = Some(text);
    }
    match event {
        CropEvent::Settings(next) => kept.settings = next,
        CropEvent::Apply(_) => {
            // The fake document stays as it is: the crop is always accepted.
            let _ = kept.tool.applied(true);
        }
        CropEvent::Frame(_) | CropEvent::Discard | CropEvent::Straightened(_) => {}
    }
}

/// One pass of the tool in a pane: the bar when the pane has one, the picture, and the tool over
/// it. Returns the tool's output while the tool is picked.
fn run(ui: &mut Ui, kept: &mut Kept, pane: &Pane) -> Option<super::CropOutput> {
    let numbers = LocaleNumbers::of(ui);
    let own = CropStoryLabels::of(ui);
    let document = pane.document();
    if pane.bar {
        let users = presets(&own);
        let mut out = CropBarOutput::default();
        let armed = kept.tool.straighten_armed();
        OptionsBar::new(("crop-story-bar", &pane.label)).show(ui, |bar| {
            CropOptionsBar::new("crop-story", &mut kept.settings)
                .document(&document)
                .user_presets(&users)
                .straighten(armed)
                .output(&mut out)
                .add_to(bar);
        });
        for event in out.take() {
            let tool_event = match event {
                CropBarEvent::Ratio { editing } => kept.tool.refit(&kept.settings, &document, editing),
                CropBarEvent::Straighten => {
                    kept.tool.toggle_straighten();
                    None
                }
                CropBarEvent::Commit => kept.tool.apply(&kept.settings, &document, kept.view),
                CropBarEvent::Cancel => kept.tool.discard(),
                CropBarEvent::NewPreset => {
                    kept.last = kept
                        .settings
                        .preset_name(&super::CropBarLabels::of(ui))
                        .map(|name| craft_i18n::fmt(&own.new_preset_named, &[("name", &name)]));
                    None
                }
                CropBarEvent::DeletePreset => {
                    kept.last = Some(own.delete_preset.clone());
                    None
                }
                CropBarEvent::PickShieldColor => {
                    kept.last = Some(own.shield_requested.clone());
                    None
                }
            };
            if let Some(event) = tool_event {
                handle(kept, &numbers, &own, event);
            }
        }
    }
    let saved = kept.view;
    let args = pane.view;
    let canvas = document_with(ui, args, &pane.label, 0.0, |rect, ppp| match saved {
        Some(view) => CanvasView { rect, ..view },
        None => {
            CanvasView { rect, center: DOC.center(), zoom: args.zoom / 100.0 / ppp, rotation: args.rotation, flip_x: args.flip, aspect: args.aspect }
        }
    });
    paint_picture(&ui.painter_at(canvas.view.rect), &canvas.view, pane.tilt);
    if !kept.picked {
        return None;
    }
    let px = RulerUnits::pixels(LocaleNumbers::clone(&numbers).ungrouped());
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let input =
        CropInput::new(&kept.settings, &document).units(&px, &px).names(CropNames::of(ui)).readout(CropLabels::of(ui)).canvas_colour(canvas_colour);
    let mut pass = kept.tool.pass(input);
    let host = ToolHost::new(canvas.id).document(document.size).show(ui, &canvas.view, &canvas.response, &mut pass);
    let out = pass.finish();
    if let Some(view) = host.view {
        kept.view = Some(view);
    }
    for event in out.events.clone() {
        handle(kept, &numbers, &own, event);
    }
    Some(out)
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// The last event the app handled, in the story words.
fn last_event(ui: &mut Ui, kept: &Kept) {
    let words = StoryLabels::of(ui);
    let text = match &kept.last {
        Some(event) => craft_i18n::fmt(&words.last_event, &[("event", event)]),
        None => words.no_event.clone(),
    };
    status(ui, text);
}

/// The frame as the status line shows it: size and turn.
fn frame_line(numbers: &LocaleNumbers, out: Option<&super::CropOutput>) -> String {
    match out.and_then(|o| o.frame) {
        Some(frame) => format!(
            "{} × {} px · {}°",
            numbers.format(frame.rect.width(), 0..=1),
            numbers.format(frame.rect.height(), 0..=1),
            numbers.format(frame.angle, 0..=1)
        ),
        None => String::new(),
    }
}

// --- default frame ---

#[derive(StoryArgs, Debug, PartialEq)]
struct DefaultArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// A selection on the page: the default frame is its bounds, else the whole page.
    selection: bool,
}

/// The tool picked: a frame is there at once, the selection's bounds or the page. A drag inside
/// the untouched default frame draws a new one, a drag inside a drawn frame moves it. "Pick
/// another tool" drops an untouched default frame and applies a drawn one.
fn show_default_frame(ui: &mut Ui, args: &mut DefaultArgs) {
    let words = StoryLabels::of(ui);
    let mut kept = load(ui, "default-frame", false, args.view);
    let mut pane = Pane::new(words.document.clone(), args.view);
    pane.key = CropDocKey(if args.selection { 2 } else { 1 });
    pane.selection = args.selection.then_some(SELECTION);
    let out = run(ui, &mut kept, &pane);
    let numbers = LocaleNumbers::of(ui);
    let own = CropStoryLabels::of(ui);
    ui.horizontal(|ui| {
        if kept.picked {
            if ui.add(Button::new(own.pick_other.as_str())).clicked() {
                let event = kept.tool.leave(&kept.settings, &pane.document(), false);
                kept.picked = false;
                kept.last = Some(own.left_tool.clone());
                if let Some(event) = event {
                    handle(&mut kept, &numbers, &own, event);
                }
            }
        } else if ui.add(Button::new(own.pick_crop.as_str())).clicked() {
            kept.picked = true;
        }
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept = Kept::new(false, args.view);
        }
        status(ui, frame_line(&numbers, out.as_ref()));
    });
    last_event(ui, &kept);
    store(ui, "default-frame", kept);
}

// --- image mode ---

#[derive(StoryArgs, Debug, PartialEq)]
struct ImageModeArgs {
    #[arg(flatten)]
    view: ViewArgs,
    auto_center: bool,
    /// The shield's opacity in percent.
    #[arg(range = 0.0..=100.0)]
    shield: f32,
}

/// Two panes over the same picture. In Image mode the frame stays upright and a drag outside it
/// turns the image; in Classic Mode the frame turns over a fixed image. P switches a pane.
fn show_image_mode(ui: &mut Ui, args: &mut ImageModeArgs) {
    let words = StoryLabels::of(ui);
    let own = CropStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    ui.columns(2, |columns| {
        for (index, ui) in columns.iter_mut().enumerate() {
            let name = if index == 0 { "image" } else { "classic" };
            let mut kept = load(ui, name, index == 1, args.view);
            kept.settings.shield.auto_center_preview = args.auto_center;
            kept.settings.shield.opacity = args.shield;
            let label = craft_i18n::fmt(&words.photo_n, &[("n", &(index + 1).to_string())]);
            let pane = Pane::new(label, args.view);
            let out = run(ui, &mut kept, &pane);
            ui.horizontal(|ui| {
                if ui.add(Button::new(words.reset.as_str())).clicked() {
                    kept = Kept::new(index == 1, args.view);
                }
                let mode = if kept.settings.shield.classic_mode { &own.classic_mode } else { &own.image_mode };
                let view = kept.view.map_or(args.view.rotation, |v| v.rotation);
                let frame = frame_line(&numbers, out.as_ref());
                status(ui, craft_i18n::fmt(&own.mode_line, &[("mode", mode), ("frame", &frame), ("angle", &numbers.format(view, 0..=1))]));
            });
            last_event(ui, &kept);
            store(ui, name, kept);
        }
    });
}

// --- ratio and size ---

#[derive(StoryArgs, Debug, PartialEq)]
struct RatioArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// The document's resolution in pixels per inch, used when Resolution is empty.
    #[arg(range = 72.0..=600.0)]
    resolution: f64,
}

/// The options bar over the tool: a ratio preset or typed sides holds every gesture to the ratio
/// and refits the frame; W x H x Resolution sets the size the commit resamples to, shown as the
/// `CropTarget`.
fn show_ratio_and_size(ui: &mut Ui, args: &mut RatioArgs) {
    let words = StoryLabels::of(ui);
    let mut kept = load(ui, "ratio-and-size", false, args.view);
    let mut pane = Pane::new(words.document.clone(), args.view);
    pane.resolution = args.resolution;
    pane.bar = true;
    let out = run(ui, &mut kept, &pane);
    let numbers = LocaleNumbers::of(ui);
    let own = CropStoryLabels::of(ui);
    let target = match kept.settings.target(&pane.document()) {
        Some(t) => {
            let (w, h) = (numbers.format(t.width, 0..=1), numbers.format(t.height, 0..=1));
            match t.resolution {
                Some(ppi) => craft_i18n::fmt(&own.target_ppi, &[("w", &w), ("h", &h), ("ppi", &numbers.format(ppi, 0..=2))]),
                None => craft_i18n::fmt(&own.target, &[("w", &w), ("h", &h)]),
            }
        }
        None => own.free_crop.clone(),
    };
    status(ui, format!("{target} · {}", frame_line(&numbers, out.as_ref())));
    last_event(ui, &kept);
    store(ui, "ratio-and-size", kept);
}

// --- straighten ---

#[derive(StoryArgs, Debug, PartialEq)]
struct StraightenArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// The button is armed for one line; the story clears it after the line.
    armed: bool,
    /// Degrees the picture's horizon is turned by.
    #[arg(range = -15.0..=15.0)]
    tilt: f64,
}

/// The bar's Straighten button arms one line along the horizon; holding ⌘ (Ctrl) draws a line
/// without arming it. The release levels the frame and fits it to the page.
fn show_straighten(ui: &mut Ui, args: &mut StraightenArgs) {
    let words = StoryLabels::of(ui);
    let mut kept = load(ui, "straighten", false, args.view);
    if args.armed != kept.tool.straighten_armed() {
        kept.tool.toggle_straighten();
    }
    let mut pane = Pane::new(words.document.clone(), args.view);
    pane.tilt = args.tilt;
    pane.bar = true;
    let out = run(ui, &mut kept, &pane);
    args.armed = kept.tool.straighten_armed();
    let numbers = LocaleNumbers::of(ui);
    status(ui, CropStoryLabels::of(ui).straighten_hint.clone());
    status(ui, frame_line(&numbers, out.as_ref()));
    last_event(ui, &kept);
    store(ui, "straighten", kept);
}

// --- box undo ---

#[derive(StoryArgs, Debug, PartialEq)]
struct UndoArgs {
    #[arg(flatten)]
    view: ViewArgs,
}

/// Edit › Undo and Redo, as the app routes them to the tool while a crop is pending: each draw,
/// move, resize, turn or key step is one box step.
fn show_box_undo(ui: &mut Ui, args: &mut UndoArgs) {
    let words = StoryLabels::of(ui);
    let mut kept = load(ui, "box-undo", false, args.view);
    let pane = Pane::new(words.document.clone(), args.view);
    let out = run(ui, &mut kept, &pane);
    let numbers = LocaleNumbers::of(ui);
    let own = CropStoryLabels::of(ui);
    ui.horizontal(|ui| {
        let session = kept.tool.session();
        let (can_undo, can_redo) = (session.pending() && session.can_undo(), session.can_redo());
        if ui.add_enabled(can_undo, Button::new(own.undo.as_str())).clicked() {
            kept.tool.undo();
        }
        if ui.add_enabled(can_redo, Button::new(own.redo.as_str())).clicked() {
            kept.tool.redo();
        }
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept = Kept::new(false, args.view);
        }
        status(ui, frame_line(&numbers, out.as_ref()));
    });
    store(ui, "box-undo", kept);
}
