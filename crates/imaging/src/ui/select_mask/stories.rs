//! Stories of the Select and Mask workspace (task ST-WS2): not registered. Registering them
//! (task RG2) adds `select_mask::stories` to the crate's `register!` list,
//! `SelectMaskWorkspace` to `COMPONENTS` and [`SelectMaskStoryLabels`] to `WORDS`, and
//! regenerates the registry dump.
//!
//! The stories run over `MockImage` through [`MockSelectMask`], a fake `SelectMaskSource` that
//! composites the view's recipe over the picture with `MockImage::subject` as the refined mask
//! (the house; no engine, the same maths as the mask-view story) and answers each preview ticket
//! at once. `views` runs the workspace as a dialog. `tools` and `refine` run the workspace's
//! parts (options bar, canvas tool, Properties panel) in a frame of the story, because the
//! workspace opens on Quick Selection and keeps its tool private. Every string goes through
//! story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `shells/select-mask/views` | The seven views with their opacities, Show Edge and Show Original |
//! | `shells/select-mask/tools` | Each tool's options bar, a Polygonal Lasso in progress, Refine Hair and Object Aware disabled with their reasons |
//! | `shells/select-mask/refine` | Edge Detection and Global Refinements, Output To with Decontaminate Colors, the limit message |

#![expect(dead_code, reason = "task RG2 registers the stories")]

use craft_ui::canvas::{Checkerboard, PixelRect, RasterSource};
use craft_ui::dialog::full_window::FullWindowCanvas;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, Surface, dialog_closed, dialog_shown, lock, shared};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Key, Modifiers, Ui, vec2};

use super::{
    BrushSizes, OutputTo, SelectMaskBar, SelectMaskCanvas, SelectMaskEvent, SelectMaskLabels, SelectMaskLimits, SelectMaskOpen, SelectMaskPanel,
    SelectMaskSession, SelectMaskSettings, SelectMaskSource, SelectMaskTool, SelectMaskToolOptions, SelectMaskWorkspace,
};
use crate::trace::MaskSource;
use crate::ui::mask_view::{MaskCoverage, MaskViewMode, MaskViewRecipe, MaskViewState, VeilMask};
use crate::ui::mock_image::{MockImage, MockSubject};
use crate::ui::selection::SelectionOverlay;
use craft_ui::edit::EditEvent;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SelectMaskWorkspace"];

craft_ui::words! {
    /// The captions of the Select and Mask stories, English by default.
    #[non_exhaustive]
    pub(crate) struct SelectMaskStoryLabels {
        subject: "selectMaskStory.subject" = "Photo";
        limit: "selectMaskStory.limit" = "At most {n} edits: undo some, or reset";
        edits: "selectMaskStory.edits" = "Edits recorded: {n}";
        hint_polygon: "selectMaskStory.hintPolygon" = "Click to add corners; Enter closes the polygon, Esc drops it.";
        hint_draw: "selectMaskStory.hintDraw" = "Drag on the picture to draw an edit.";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "shells/select-mask/views",
            Level::Shells,
            "SelectMaskWorkspace",
            "Views",
            ViewsArgs { open: true, view: ViewArg::OnionSkin, opacity: 50.0, show_edge: false, show_original: false },
            show_views
        )
        .on(Surface::Canvas),
        typed_story!("shells/select-mask/tools", Level::Shells, "SelectMaskWorkspace", "Tools", ToolsArgs { tool: ToolArg::Polygonal }, show_tools)
            .on(Surface::Canvas),
        typed_story!(
            "shells/select-mask/refine",
            Level::Shells,
            "SelectMaskWorkspace",
            "Refine",
            RefineArgs { smart_radius: true, decontaminate: true, limit_message: false },
            show_refine
        )
        .on(Surface::Canvas),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ViewArg {
    #[choice(name = "Onion Skin")]
    OnionSkin,
    #[choice(name = "Marching Ants")]
    MarchingAnts,
    Overlay,
    #[choice(name = "On Black")]
    OnBlack,
    #[choice(name = "On White")]
    OnWhite,
    #[choice(name = "Black & White")]
    BlackAndWhite,
    #[choice(name = "On Layers")]
    OnLayers,
}

impl ViewArg {
    fn mode(self) -> MaskViewMode {
        match self {
            ViewArg::OnionSkin => MaskViewMode::OnionSkin,
            ViewArg::MarchingAnts => MaskViewMode::MarchingAnts,
            ViewArg::Overlay => MaskViewMode::Overlay,
            ViewArg::OnBlack => MaskViewMode::OnBlack,
            ViewArg::OnWhite => MaskViewMode::OnWhite,
            ViewArg::BlackAndWhite => MaskViewMode::BlackAndWhite,
            ViewArg::OnLayers => MaskViewMode::OnLayers,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ToolArg {
    #[choice(name = "Quick Selection")]
    Quick,
    #[choice(name = "Refine Edge Brush")]
    Refine,
    Brush,
    #[choice(name = "Object Selection")]
    Object,
    Lasso,
    #[choice(name = "Polygonal Lasso")]
    Polygonal,
}

impl ToolArg {
    fn tool(self) -> SelectMaskTool {
        match self {
            ToolArg::Quick => SelectMaskTool::Quick,
            ToolArg::Refine => SelectMaskTool::RefineEdge,
            ToolArg::Brush => SelectMaskTool::Brush,
            ToolArg::Object => SelectMaskTool::Object,
            ToolArg::Lasso => SelectMaskTool::Lasso,
            ToolArg::Polygonal => SelectMaskTool::PolygonalLasso,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ViewsArgs {
    open: bool,
    /// The view the workspace opens with.
    view: ViewArg,
    /// That view's Transparency or Opacity, in percent.
    #[arg(range = 0.0..=100.0)]
    opacity: f64,
    #[arg(name = "show edge")]
    show_edge: bool,
    #[arg(name = "show original")]
    show_original: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ToolsArgs {
    /// The tool on the canvas and in the bar.
    tool: ToolArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RefineArgs {
    #[arg(name = "smart radius")]
    smart_radius: bool,
    #[arg(name = "decontaminate colors")]
    decontaminate: bool,
    /// Show the limit message under the panel.
    #[arg(name = "limit message")]
    limit_message: bool,
}

/// The engine's limits the stories give the workspace.
const LIMITS: SelectMaskLimits = SelectMaskLimits { edits: 4096, edit_points: 100_000, strokes: 4096, stroke_points: 100_000 };
/// The tools frame's height as a fraction of its width.
const FRAME_RATIO: f32 = 0.6;
/// Show Edge's band: its half width as a fraction of the picture's width.
const BAND: f64 = 0.008;

/// The view the engine would build from a recipe, drawn on the spot: the layer (the picture) with
/// its mask raised to the recipe's floor, then each veil over it.
struct Composite {
    image: MockImage,
    recipe: MaskViewRecipe,
    revision: u64,
}

impl Composite {
    fn coverage(&self, x: f64, y: f64) -> f32 {
        match self.recipe.coverage {
            MaskCoverage::Refined => self.image.subject(x, y),
            MaskCoverage::Original => f32::from(self.image.subject_distance(x, y) < 0.0),
            MaskCoverage::EdgeBand => {
                let reach = BAND * f64::from(self.image.size()[0]);
                (1.0 - (self.image.subject_distance(x, y).abs() / reach).clamp(0.0, 1.0)) as f32
            }
        }
    }

    fn at(&self, x: f64, y: f64) -> Color32 {
        let [r, g, b, _] = self.image.pixel(x, y);
        if self.recipe.hidden {
            return Color32::from_rgb(r, g, b);
        }
        let c = self.coverage(x, y);
        let mut colour = [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0];
        let mut alpha = self.recipe.layer_floor.map_or(1.0, |floor| c + (1.0 - c) * floor);
        for veil in &self.recipe.veils {
            let mask = match veil.mask {
                VeilMask::Everywhere => 1.0,
                VeilMask::Coverage => c,
                VeilMask::Inverse => 1.0 - c,
            };
            let on = veil.opacity * mask * veil.color.a;
            let out = on + alpha * (1.0 - on);
            if out > 0.0 {
                let over = [veil.color.r, veil.color.g, veil.color.b];
                for (channel, over) in colour.iter_mut().zip(over) {
                    *channel = (over * on + *channel * alpha * (1.0 - on)) / out;
                }
            }
            alpha = out;
        }
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(byte(colour[0]), byte(colour[1]), byte(colour[2]), byte(alpha))
    }
}

impl RasterSource for Composite {
    fn size(&self) -> [u32; 2] {
        self.image.size()
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let factor = f64::from(factor.max(1));
        for row in 0..rows {
            let y = f64::from(region.y0) + (row as f64 + 0.5) * factor;
            for column in 0..columns {
                out.pixels[row * columns + column] = self.at(f64::from(region.x0) + (column as f64 + 0.5) * factor, y);
            }
        }
    }
}

/// The fake source: the composite and the subject as the refined selection.
struct MockSelectMask {
    composite: Composite,
    subject: MockSubject,
}

impl MockSelectMask {
    fn new(recipe: MaskViewRecipe) -> Self {
        let image = MockImage::default();
        Self { composite: Composite { image, recipe, revision: 1 }, subject: MockSubject(image) }
    }

    /// Takes the workspace's recipe; the picture recomputes when it changed.
    fn follow(&mut self, recipe: MaskViewRecipe) {
        if self.composite.recipe != recipe {
            self.composite.recipe = recipe;
            self.composite.revision += 1;
        }
    }
}

impl SelectMaskSource for MockSelectMask {
    fn view(&self) -> &dyn RasterSource {
        &self.composite
    }

    fn ants(&self) -> Option<(&dyn MaskSource, u32)> {
        Some((&self.subject, 1))
    }
}

/// What the views story keeps: the workspace, the source, and what they were built from.
struct Held {
    workspace: SelectMaskWorkspace,
    source: MockSelectMask,
    signature: (MaskViewMode, u64, bool, bool),
}

impl Held {
    fn new(settings: SelectMaskSettings, subject: &str, signature: (MaskViewMode, u64, bool, bool)) -> Self {
        let recipe = settings.view_state().recipe();
        let workspace = SelectMaskWorkspace::new(SelectMaskOpen::new(MockImage::default().size(), subject, LIMITS).settings(settings));
        Held { workspace, source: MockSelectMask::new(recipe), signature }
    }
}

fn show_views(ui: &mut Ui, args: &mut ViewsArgs) {
    let words = SelectMaskStoryLabels::of(ui);
    if !dialog_shown(ui, args.open) {
        return;
    }
    let mode = args.view.mode();
    let mut state = MaskViewState { mode, show_edge: args.show_edge, show_original: args.show_original, ..MaskViewState::default() };
    state.set_opacity(mode, args.opacity as f32);
    let mut settings = SelectMaskSettings::default();
    settings.take_view(&state);
    let signature = (mode, args.opacity.to_bits(), args.show_edge, args.show_original);

    let held = shared(ui, "views", || Held::new(settings.clone(), &words.subject, signature));
    let mut held = lock(&held);
    if held.signature != signature {
        *held = Held::new(settings.clone(), &words.subject, signature);
    }
    let ctx = ui.ctx().clone();
    let Held { workspace, source, .. } = &mut *held;
    let out = workspace.show(&ctx, source);
    if let Some(ticket) = out.preview_wanted {
        workspace.preview_ready(ticket);
    }
    source.follow(workspace.recipe());
    if out.outcome.is_some() {
        *held = Held::new(settings, &words.subject, signature);
        dialog_closed(ui, &mut args.open);
    }
}

/// What the tools story keeps.
struct Tools {
    session: SelectMaskSession,
    settings: SelectMaskSettings,
    view_state: MaskViewState,
    options: SelectMaskToolOptions,
    canvas: SelectMaskCanvas,
    view: craft_ui::dialog::full_window::FullWindowView,
    error: Option<String>,
}

fn show_tools(ui: &mut Ui, args: &mut ToolsArgs) {
    let words = SelectMaskStoryLabels::of(ui);
    let labels = SelectMaskLabels::of(ui);
    let state = shared(ui, "tools", || {
        let settings = SelectMaskSettings::default();
        Tools {
            session: SelectMaskSession::default(),
            view_state: settings.view_state(),
            settings,
            options: SelectMaskToolOptions { sizes: BrushSizes::default(), ..SelectMaskToolOptions::default() },
            canvas: SelectMaskCanvas::new(),
            view: craft_ui::dialog::full_window::FullWindowView::new(),
            error: None,
        }
    });
    let mut state = lock(&state);
    let Tools { session, settings, view_state, options, canvas, view, error } = &mut *state;
    let tool = args.tool.tool();
    if !tool.brushes() && canvas.polygon_open() && tool != SelectMaskTool::PolygonalLasso {
        canvas.drop_polygon();
    }

    let bar = SelectMaskBar::new("select-mask-story-bar", tool, options, &labels).show(ui);
    if bar.select_subject {
        let edit = super::SelectMaskEdit::Subject { sample_all_layers: options.sample_all_layers };
        record(session, error, &words, |session| session.add_edit(edit, &LIMITS));
    }
    let hint = if tool == SelectMaskTool::PolygonalLasso { &words.hint_polygon } else { &words.hint_draw };
    ui.add(Text::new(hint.as_str()).content(Content::Muted));

    let image = MockImage::default();
    let subject = MockSubject(image);
    ui.columns(2, |columns| {
        let ui = &mut columns[0];
        let width = ui.available_width();
        ui.allocate_ui(vec2(width, width * FRAME_RATIO), |ui| {
            let ctx = ui.ctx().clone();
            if canvas.polygon_open() {
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                    canvas.close_polygon();
                } else if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                    canvas.drop_polygon();
                }
            }
            let mut active = canvas.tool(tool, options, settings.real_time);
            let shown = FullWindowCanvas::new("select-mask-story-canvas", view)
                .source(&image)
                .checkerboard(Checkerboard::default())
                .tool(&mut active)
                .show(ui);
            SelectionOverlay::new("select-mask-story-ants", &subject).show(ui, &shown.response, &shown.view);
        });
        for event in canvas.take_events() {
            match event {
                SelectMaskEvent::Edit(EditEvent::Commit { value, changed: true, .. }) => {
                    record(session, error, &words, |s| s.add_edit(value, &LIMITS))
                }
                SelectMaskEvent::Refine(EditEvent::Commit { value, changed: true, .. }) => {
                    record(session, error, &words, |s| s.add_stroke(value, &LIMITS));
                }
                _ => {}
            }
        }
        let count = (session.edits.len() + session.strokes.len()).to_string();
        let text = craft_i18n::fmt(&words.edits, &[("n", count.as_str())]);
        columns[0].add(Text::new(text).content(Content::Muted));

        SelectMaskPanel::new("select-mask-story-panel", settings, &mut session.refine, view_state, &labels)
            .error(error.as_deref())
            .show(&mut columns[1]);
    });
}

/// Records an edit in the session; a refusal becomes the limit message.
fn record(
    session: &mut SelectMaskSession,
    error: &mut Option<String>,
    words: &SelectMaskStoryLabels,
    add: impl FnOnce(&mut SelectMaskSession) -> Result<(), String>,
) {
    match add(session) {
        Ok(()) => *error = None,
        Err(_) => *error = Some(craft_i18n::fmt(&words.limit, &[("n", LIMITS.edits.to_string().as_str())])),
    }
}

/// What the refine story keeps.
struct Refine {
    settings: SelectMaskSettings,
    session: SelectMaskSession,
    view_state: MaskViewState,
}

fn show_refine(ui: &mut Ui, args: &mut RefineArgs) {
    let words = SelectMaskStoryLabels::of(ui);
    let labels = SelectMaskLabels::of(ui);
    let state = shared(ui, "refine", || {
        let settings = SelectMaskSettings::default();
        Refine { view_state: settings.view_state(), session: SelectMaskSession::default(), settings }
    });
    let mut state = lock(&state);
    let Refine { settings, session, view_state } = &mut *state;
    session.refine.smart_radius = args.smart_radius;
    if session.refine.decontaminate != args.decontaminate {
        session.refine.decontaminate = args.decontaminate;
        session.refine.follow_decontaminate();
    }
    if !session.refine.output.allowed(session.refine.decontaminate) {
        session.refine.output = OutputTo::NewLayerWithMask;
    }
    let message = craft_i18n::fmt(&words.limit, &[("n", LIMITS.edits.to_string().as_str())]);
    let error = args.limit_message.then_some(message.as_str());
    ui.columns(2, |columns| {
        SelectMaskPanel::new("select-mask-story-refine", settings, &mut session.refine, view_state, &labels).error(error).show(&mut columns[0]);
        let ui = &mut columns[1];
        let image = MockImage::default();
        let width = ui.available_width();
        let (response, view) = image.canvas(ui, "select-mask-story-refine-picture", width, None, None);
        SelectionOverlay::new("select-mask-story-refine-ants", &MockSubject(image)).show(ui, &response, &view);
    });
}
