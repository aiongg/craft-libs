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
//! | `patterns/select-mask/views` | The seven views with their opacities, Show Edge and Show Original |
//! | `patterns/select-mask/tools` | Each tool's options bar, a Polygonal Lasso in progress, Refine Hair and Object Aware disabled with their reasons |
//! | `patterns/select-mask/refine` | Edge Detection and Global Refinements, Output To with Decontaminate Colors, the limit message |

use craft_ui::canvas::kurbo::{Point, Vec2};
use craft_ui::canvas::{CanvasView, Checkerboard, RasterSource};
use craft_ui::dialog::full_window::FullWindowCanvas;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, Surface, claim, dialog_closed, dialog_shown, lock, reset_on, shared};
use craft_ui::testing::canvas_story::{ViewArgs, document_with};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;
use egui::{Key, Modifiers, Painter, Pos2, Shape, Stroke, Ui, vec2};

use super::{
    BrushSizes, EditMode, ObjectTarget, OutputTo, SelectMaskBar, SelectMaskCanvas, SelectMaskEvent, SelectMaskLimits, SelectMaskOpen,
    SelectMaskPanel, SelectMaskSession, SelectMaskSettings, SelectMaskSource, SelectMaskTool, SelectMaskToolOptions, SelectMaskWorkspace,
};
use crate::trace::MaskSource;
use crate::ui::mask_view::stories::Composite;
use crate::ui::mask_view::{MaskViewMode, MaskViewRecipe, MaskViewState};
use crate::ui::mock_image::{MockImage, MockSubject};
use crate::ui::selection::{SelectionOverlay, SelectionOverlayLabels};
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
        picture: "selectMaskStory.picture" = "Refined picture";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/select-mask/views",
            Level::Patterns,
            "SelectMaskWorkspace",
            "Views",
            ViewsArgs { open: true, view: ViewArg::OnionSkin, opacity: 50.0, show_edge: false, show_original: false },
            show_views
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/select-mask/tools",
            Level::Patterns,
            "SelectMaskWorkspace",
            "Tools",
            ToolsArgs { tool: ToolArg::Polygonal },
            show_tools
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/select-mask/refine",
            Level::Patterns,
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
    fn image(&self) -> &dyn RasterSource {
        &self.composite.image
    }

    fn view(&self) -> &dyn RasterSource {
        &self.composite
    }

    fn ants(&self) -> Option<(&dyn MaskSource, u32)> {
        Some((&self.subject, 1))
    }
}

/// What the views story keeps: the workspace and the source.
struct Held {
    workspace: SelectMaskWorkspace,
    source: MockSelectMask,
}

impl Held {
    fn new(settings: SelectMaskSettings, subject: &str) -> Self {
        let recipe = settings.view_state().recipe();
        let workspace = SelectMaskWorkspace::new(SelectMaskOpen::new(MockImage::default().size(), subject, LIMITS).settings(settings));
        Held { workspace, source: MockSelectMask::new(recipe) }
    }
}

fn show_views(ui: &mut Ui, args: &mut ViewsArgs) {
    let words = SelectMaskStoryLabels::of(ui);
    if !(dialog_shown(ui, args.open) && claim(ui.ctx(), "select_mask", ui.scope_id())) {
        return;
    }
    let mode = args.view.mode();
    let mut state = MaskViewState { mode, show_edge: args.show_edge, show_original: args.show_original, ..MaskViewState::default() };
    state.set_opacity(mode, args.opacity as f32);
    let mut settings = SelectMaskSettings::default();
    settings.take_view(&state);
    let signature = (mode, args.opacity.to_bits(), args.show_edge, args.show_original);

    reset_on(ui, "views", &signature);
    let held = shared(ui, "views", || Held::new(settings.clone(), &words.subject));
    let mut held = lock(&held);
    let ctx = ui.ctx().clone();
    let Held { workspace, source } = &mut *held;
    let out = workspace.show(&ctx, source);
    if let Some(ticket) = out.preview_wanted {
        workspace.preview_ready(ticket);
    }
    source.follow(workspace.recipe());
    if out.outcome.is_some() {
        *held = Held::new(settings, &words.subject);
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

    let bar = SelectMaskBar::new("select-mask-story-bar", tool, options).show(ui);
    if bar.select_subject {
        let edit = super::SelectMaskEdit::Subject { sample_all_layers: options.sample_all_layers };
        record(session, error, &words, |session| session.add_edit(edit, &LIMITS));
    }
    let hint = if tool == SelectMaskTool::PolygonalLasso { &words.hint_polygon } else { &words.hint_draw };
    ui.add(Text::new(hint.as_str()).content(Content::Muted));

    let image = MockImage::default();
    let subject = MockSubject(image);
    let tokens = Tokens::of(ui);
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
            SelectionOverlay::new("select-mask-story-ants", &subject).labels(SelectionOverlayLabels::of(ui)).show(ui, &shown.response, &shown.view);
            paint_edits(&ui.painter_at(shown.response.rect), &shown.view, session, &tokens);
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

        SelectMaskPanel::new("select-mask-story-panel", settings, &mut session.refine, view_state).error(error.as_deref()).show(&mut columns[1]);
    });
}

/// Draws the session's recorded edits over the picture: additions in the accent, subtractions in
/// the bad status color; the Refine Edge Brush strokes in the same two. Select Subject,
/// Clear and Invert have nothing to draw.
fn paint_edits(painter: &Painter, view: &CanvasView, session: &SelectMaskSession, tokens: &UiTokens) {
    const FILL_SHARE: f32 = 0.35;
    const HAIRLINE: f32 = 1.0;
    let color = |mode: EditMode| match mode {
        EditMode::Add => tokens.color.accent.strong.to_egui(),
        EditMode::Subtract => tokens.color.status.bad.fg.to_egui(),
    };
    let screen = |points: &[[f64; 2]]| -> Vec<Pos2> { points.iter().map(|p| view.to_screen(Point::new(p[0], p[1]))).collect() };
    let length = |size: f64| view.map_vec(Vec2::new(size, 0.0)).length();
    let trail = |points: Vec<Pos2>, width: f32, color: egui::Color32| match points.as_slice() {
        [] => {}
        [only] => {
            painter.circle_filled(*only, width / 2.0, color);
        }
        _ => {
            painter.add(Shape::line(points, Stroke::new(width, color)));
        }
    };
    for edit in &session.edits {
        match edit {
            super::SelectMaskEdit::Quick { points, size, mode, .. } | super::SelectMaskEdit::Brush { points, size, mode, .. } => {
                trail(screen(points), length(*size), color(*mode).gamma_multiply(FILL_SHARE));
            }
            super::SelectMaskEdit::Lasso { points, mode } => {
                painter.add(Shape::closed_line(screen(points), Stroke::new(HAIRLINE, color(*mode))));
            }
            super::SelectMaskEdit::Object { target, mode, .. } => match target {
                ObjectTarget::Point(at) => {
                    painter.circle_stroke(view.to_screen(Point::new(at[0], at[1])), HAIRLINE * 4.0, Stroke::new(HAIRLINE, color(*mode)));
                }
                ObjectTarget::Rect([x, y, w, h]) => {
                    let corners = [[*x, *y], [x + w, *y], [x + w, y + h], [*x, y + h]];
                    painter.add(Shape::closed_line(screen(&corners), Stroke::new(HAIRLINE, color(*mode))));
                }
            },
            super::SelectMaskEdit::Subject { .. } | super::SelectMaskEdit::Clear | super::SelectMaskEdit::Invert => {}
        }
    }
    for stroke in &session.strokes {
        trail(screen(&stroke.points), length(stroke.size), color(stroke.mode).gamma_multiply(FILL_SHARE));
    }
}

/// Records an edit in the session; a refusal becomes the limit message.
fn record(
    session: &mut SelectMaskSession,
    error: &mut Option<String>,
    words: &SelectMaskStoryLabels,
    add: impl FnOnce(&mut SelectMaskSession) -> Result<(), super::LimitRefused>,
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
        SelectMaskPanel::new("select-mask-story-refine", settings, &mut session.refine, view_state).error(error).show(&mut columns[0]);
        let ui = &mut columns[1];
        let image = MockImage::default();
        let picture = document_with(ui, ViewArgs::default(), words.picture.as_str(), 0.0, image.view(None, None));
        image.show(ui, &picture);
        SelectionOverlay::new("select-mask-story-refine-ants", &MockSubject(image)).labels(SelectionOverlayLabels::of(ui)).show(
            ui,
            &picture.response,
            &picture.view,
        );
    });
}
