//! Stories of Vanishing Point: not registered. Registering them adds
//! `vanishing_point::stories` to the crate's `register!` list, `VanishingPointWorkspace` to
//! `COMPONENTS` and [`VanishingPointStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The `planes` and `stamp` stories run the workspace's own parts, the options bar and the plane
//! editor on a `FullWindowCanvas`, in a frame of the story (the tool is an arg, so Create Plane's
//! pending corners and the Stamp tool show without the strip). The `selection` story runs the
//! workspace itself as a dialog. The scene is [`MockScene`]: planes are
//! flat quadrilaterals, the first is valid, the second questionable and the third invalid, plane
//! 3 is torn off the first one's right edge, and the stamp ring is the circle squashed to 0.5
//! (an affine ring, no perspective maths). The layer is `MockImage`, the stamp preview is the
//! layer (no stamp painting) and the paste is a small generated texture. Every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/vanishing-point/planes` | Valid, questionable and invalid planes, a torn-off wall; click a plane to select it and show its nodes; Create Plane's pending corners |
//! | `patterns/vanishing-point/stamp` | The source cross, the foreshortened ring under the pointer, the paste image on a plane |
//! | `patterns/vanishing-point/selection` | The whole workspace as a dialog, opened on the plane the arg picks (`VanishingPointWorkspace::selected_plane`), with its nodes shown |

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{Checkerboard, NavigationOptions, RasterSource};
use craft_ui::dialog::Outcome;
use craft_ui::dialog::full_window::{FullWindowCanvas, FullWindowView};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, claim, dialog_closed, dialog_shown, lock, reset_on, shared};
use craft_ui::theme::current;
use craft_ui::typed_story;
use egui::load::SizedTexture;
use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Ui, vec2};

use super::{
    PlaneCheck, PlaneEdge, StampLimits, TornPlane, VanishingPlane, VanishingPointBar, VanishingPointEditor, VanishingPointOpen,
    VanishingPointSession, VanishingPointSettings, VanishingPointSource, VanishingPointStyle, VanishingPointWorkspace, VanishingScene, VanishingTool,
};
use crate::ui::mock_image::MockImage;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["VanishingPointWorkspace"];

craft_ui::words! {
    /// The captions of the Vanishing Point stories, English by default.
    #[non_exhaustive]
    pub(crate) struct VanishingPointStoryLabels {
        hint_planes: "vanishingPointStory.hintPlanes" = "Click a plane to select it; drag a node to reshape it, an edge node to slide the edge; ⌘-drag an edge node to tear off a wall.";
        hint_create: "vanishingPointStory.hintCreate" = "Click four corners to make a plane.";
        hint_stamp: "vanishingPointStory.hintStamp" = "Alt-click sets the clone source; drag to paint a stroke.";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/vanishing-point/planes",
            Level::Patterns,
            "VanishingPointWorkspace",
            "Planes",
            PlanesArgs { tool: PlaneTool::Edit, grid_size: 100.0 },
            show_planes
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/vanishing-point/stamp",
            Level::Patterns,
            "VanishingPointWorkspace",
            "Stamp",
            StampArgs { paste: true, aligned: true, diameter: 80.0 },
            show_stamp
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/vanishing-point/selection",
            Level::Patterns,
            "VanishingPointWorkspace",
            "Selection",
            SelectionArgs { open: true, plane: SelectedArg::Wall },
            show_selection
        )
        .on(Surface::Own),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PlaneTool {
    #[choice(name = "Edit Plane")]
    Edit,
    #[choice(name = "Create Plane")]
    Create,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SelectedArg {
    #[choice(name = "None")]
    NoPlane,
    Wall,
    Window,
    #[choice(name = "Floor Patch")]
    FloorPatch,
    #[choice(name = "Torn Wall")]
    TornWall,
}

impl SelectedArg {
    /// The index of the plane in [`planes_session`].
    fn index(self) -> Option<usize> {
        match self {
            SelectedArg::NoPlane => None,
            SelectedArg::Wall => Some(0),
            SelectedArg::Window => Some(1),
            SelectedArg::FloorPatch => Some(2),
            SelectedArg::TornWall => Some(3),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SelectionArgs {
    /// Whether the workspace is open.
    open: bool,
    /// The plane the workspace opens on.
    plane: SelectedArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PlanesArgs {
    /// The tool on the canvas.
    tool: PlaneTool,
    /// The grid's cell size, in document pixels.
    #[arg(name = "grid size", range = 8.0..=400.0)]
    grid_size: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StampArgs {
    /// Paste on: the generated image on the first plane.
    paste: bool,
    /// The stamp is aligned.
    aligned: bool,
    /// The stamp's diameter, in document pixels.
    #[arg(range = 1.0..=400.0)]
    diameter: f64,
}

/// The planes of the scene in `MockImage::default` pixels: a wall, a questionable window, an
/// invalid floor patch.
const PLANES: [[[f64; 2]; 4]; 3] = [
    [[250.0, 300.0], [800.0, 330.0], [800.0, 780.0], [250.0, 850.0]],
    [[900.0, 200.0], [1400.0, 220.0], [1400.0, 500.0], [900.0, 520.0]],
    [[900.0, 650.0], [1300.0, 650.0], [1300.0, 950.0], [900.0, 950.0]],
];
/// The torn wall: plane 3 from plane 0's right edge.
const TORN: TornPlane = TornPlane { from: 0, edge: PlaneEdge::Right, angle: 120.0, depth: 0.5 };
/// The ring's squash (the foreshortening of a plane seen at an angle).
const RING_SQUASH: f64 = 0.5;
/// Points of the ring.
const RING_POINTS: usize = 48;
/// The canvas frame's height as a fraction of its width.
const FRAME_RATIO: f32 = 0.6;

/// A flat scene: planes are the quadrilaterals as they are, the torn wall is a quadrilateral on
/// its parent's edge turned by the angle, and plane `i` is valid, questionable or invalid by its
/// index (0, 1, 2; later planes are valid).
struct MockScene;

fn turns(q: &[[f64; 2]; 4]) -> f64 {
    let e = |i: usize| [q[(i + 1) % 4][0] - q[i][0], q[(i + 1) % 4][1] - q[i][1]];
    (0..4).map(|i| e(i)[0] * e((i + 1) % 4)[1] - e(i)[1] * e((i + 1) % 4)[0]).sum()
}

impl VanishingScene for MockScene {
    fn normalized(&self, plane: VanishingPlane) -> VanishingPlane {
        let mut q = plane.corners;
        if turns(&q) < 0.0 {
            q = [q[0], q[3], q[2], q[1]];
        }
        let first = (0..4).min_by(|&a, &b| (q[a][0] + q[a][1]).total_cmp(&(q[b][0] + q[b][1]))).unwrap_or(0);
        VanishingPlane { corners: [q[first], q[(first + 1) % 4], q[(first + 2) % 4], q[(first + 3) % 4]] }
    }

    fn resolve(&self, planes: &[VanishingPlane], torn: &[Option<TornPlane>]) -> Vec<VanishingPlane> {
        let mut out: Vec<VanishingPlane> = Vec::new();
        for (i, plane) in planes.iter().enumerate() {
            let Some(t) = torn.get(i).copied().flatten().filter(|t| t.from < i) else {
                out.push(*plane);
                continue;
            };
            let c = out[t.from].corners;
            let e = PlaneEdge::ALL.iter().position(|x| *x == t.edge).unwrap_or(0);
            let (a, b, inward) = (c[e], c[(e + 1) % 4], c[[3, 0, 1, 2][e]]);
            let v = [inward[0] - a[0], inward[1] - a[1]];
            let len = v[0].hypot(v[1]);
            let turn = (t.angle - 90.0).to_radians();
            let o = [-v[0] / len, -v[1] / len];
            let o = [o[0] * turn.cos() - o[1] * turn.sin(), o[0] * turn.sin() + o[1] * turn.cos()];
            let far = |p: [f64; 2]| [p[0] + o[0] * len * t.depth, p[1] + o[1] * len * t.depth];
            let mut corners = [[0.0; 2]; 4];
            corners[e] = far(a);
            corners[(e + 1) % 4] = far(b);
            corners[(e + 2) % 4] = b;
            corners[(e + 3) % 4] = a;
            out.push(VanishingPlane { corners });
        }
        out
    }

    fn check(&self, _: &[VanishingPlane], index: usize) -> PlaneCheck {
        match index {
            1 => PlaneCheck::Questionable,
            2 => PlaneCheck::Invalid,
            _ => PlaneCheck::Valid,
        }
    }

    fn plane_at(&self, planes: &[VanishingPlane], at: [f64; 2]) -> Option<usize> {
        let inside = |q: &[[f64; 2]; 4]| {
            let side = |i: usize| {
                let (a, b) = (q[i], q[(i + 1) % 4]);
                (b[0] - a[0]) * (at[1] - a[1]) - (b[1] - a[1]) * (at[0] - a[0])
            };
            (0..4).all(|i| side(i) >= 0.0) || (0..4).all(|i| side(i) <= 0.0)
        };
        planes.iter().rposition(|p| inside(&p.corners))
    }

    fn frame(&self, planes: &[VanishingPlane], index: usize) -> Option<(craft_ui::canvas::quad::Homography, f64, f64)> {
        let p = planes.get(index)?;
        let len = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        Some((p.homography()?, len(p.corners[0], p.corners[1]), len(p.corners[0], p.corners[3])))
    }

    fn stamp_ring(&self, _: &[VanishingPlane], at: [f64; 2], diameter: f64) -> Option<Vec<Point>> {
        let radius = diameter / 2.0;
        Some(
            (0..RING_POINTS)
                .map(|i| {
                    let (sin, cos) = (i as f64 / RING_POINTS as f64 * std::f64::consts::TAU).sin_cos();
                    Point::new(at[0] + radius * cos, at[1] + radius * RING_SQUASH * sin)
                })
                .collect(),
        )
    }

    fn clone_source(&self, planes: &[VanishingPlane], source: [f64; 2], anchor: [f64; 2], at: [f64; 2]) -> Option<[f64; 2]> {
        self.plane_at(planes, source)?;
        Some([source[0] + at[0] - anchor[0], source[1] + at[1] - anchor[1]])
    }
}

/// The layer, with the generated paste texture; the stamp preview is the layer.
struct MockSource {
    image: MockImage,
    paste: SizedTexture,
}

impl VanishingPointSource for MockSource {
    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn stamp_preview(&self) -> Option<&dyn RasterSource> {
        Some(&self.image)
    }

    fn paste(&self) -> Option<SizedTexture> {
        Some(self.paste)
    }
}

/// The paste's texture size in pixels.
const PASTE: usize = 64;
/// The engine's clone-stroke limits the stories give the editor.
const LIMITS: StampLimits = StampLimits { strokes: 4096, dabs: 200_000 };

/// A diagonal two-colour pattern: the clipboard's image.
fn paste_image() -> ColorImage {
    let mut image = ColorImage::filled([PASTE, PASTE], Color32::WHITE);
    for (i, pixel) in image.pixels.iter_mut().enumerate() {
        let (x, y) = (i % PASTE, i / PASTE);
        *pixel = if (x / 8 + y / 8) % 2 == 0 { Color32::from_rgb(230, 90, 60) } else { Color32::from_rgb(250, 220, 90) };
    }
    image
}

/// What a story keeps between passes.
struct State {
    session: VanishingPointSession,
    editor: VanishingPointEditor,
    view: FullWindowView,
    options: VanishingPointSettings,
    texture: TextureHandle,
}

impl State {
    fn new(ui: &Ui, session: VanishingPointSession) -> Self {
        let texture = ui.ctx().load_texture("vanishing-point-story-paste", paste_image(), TextureOptions::NEAREST);
        Self { session, editor: VanishingPointEditor::new(LIMITS), view: FullWindowView::new(), options: VanishingPointSettings::default(), texture }
    }
}

/// The session with the three planes and the torn wall.
fn planes_session() -> VanishingPointSession {
    let mut planes: Vec<VanishingPlane> = PLANES.iter().map(|corners| VanishingPlane { corners: *corners }).collect();
    planes.push(VanishingPlane { corners: [[0.0; 2]; 4] });
    let torn = vec![None, None, None, Some(TORN)];
    let planes = MockScene.resolve(&planes, &torn);
    VanishingPointSession { planes, torn, ..VanishingPointSession::default() }
}

/// The bar above and the canvas under it, with the editor as the canvas's tool.
fn editor(ui: &mut Ui, state: &mut State, salt: &'static str, tool: VanishingTool, hint: &str) {
    let style = VanishingPointStyle::of(&current(ui.ctx()));
    let image = MockImage::default();
    let source = MockSource { image, paste: SizedTexture::new(state.texture.id(), vec2(PASTE as f32, PASTE as f32)) };
    state.editor.set_tool(tool);
    let has_clip = true;
    VanishingPointBar::new(("vanishing-point-story-bar", salt), tool, &mut state.options, state.session.paste, has_clip)
        .has_strokes(!state.session.strokes.is_empty())
        .show(ui);
    ui.add(Text::new(hint).content(Content::Muted));
    let width = ui.available_width();
    ui.allocate_ui(vec2(width, width * FRAME_RATIO), |ui| {
        let mut pass = state.editor.pass(&mut state.session, &state.options, &MockScene, &source, &style);
        let _ = FullWindowCanvas::new(("vanishing-point-story", salt), &mut state.view)
            .source(&source.image)
            .checkerboard(Checkerboard::default())
            .navigation(NavigationOptions::default())
            .tool(&mut pass)
            .show(ui);
    });
    drop(state.editor.take_events());
}

fn show_planes(ui: &mut Ui, args: &mut PlanesArgs) {
    let words = VanishingPointStoryLabels::of(ui);
    let state = shared(ui, "planes", || State::new(ui, planes_session()));
    let mut state = lock(&state);
    state.options.grid_size = args.grid_size;
    let (tool, hint) = match args.tool {
        PlaneTool::Edit => (VanishingTool::EditPlane, words.hint_planes.as_str()),
        PlaneTool::Create => (VanishingTool::CreatePlane, words.hint_create.as_str()),
    };
    editor(ui, &mut state, "planes", tool, hint);
}

/// The stamp story's clone source, in document pixels. It has no finished stroke: with a stroke
/// the workspace shows the stamp preview and leaves the paste to the app.
const CLONE_SOURCE: [f64; 2] = [420.0, 450.0];

fn show_stamp(ui: &mut Ui, args: &mut StampArgs) {
    let words = VanishingPointStoryLabels::of(ui);
    let state = shared(ui, "stamp", || {
        let mut session = planes_session();
        session.source = Some(CLONE_SOURCE);
        State::new(ui, session)
    });
    let mut state = lock(&state);
    state.options.paste = args.paste;
    state.options.stamp.aligned = args.aligned;
    state.options.stamp.diameter = args.diameter;
    editor(ui, &mut state, "stamp", VanishingTool::Stamp, words.hint_stamp.as_str());
}

/// The workspace a `selection` story keeps, with the paste texture its source holds.
struct Opened {
    workspace: VanishingPointWorkspace,
    texture: TextureHandle,
}

fn show_selection(ui: &mut Ui, args: &mut SelectionArgs) {
    reset_on(ui, "selection", &args.plane);
    let slot = shared(ui, "selection", || None::<Opened>);
    let mut kept = lock(&slot);
    if !(dialog_shown(ui, args.open) && claim(ui.ctx(), "vanishing_point", ui.scope_id())) {
        *kept = None;
        return;
    }
    let image = MockImage::default();
    let opened = kept.get_or_insert_with(|| {
        let texture = ui.ctx().load_texture("vanishing-point-story-selection-paste", paste_image(), TextureOptions::NEAREST);
        let mut open = VanishingPointOpen::new(image.size(), StoryLabels::of(ui).photo, LIMITS);
        open.session = planes_session();
        let mut workspace = VanishingPointWorkspace::new(open);
        workspace.selected_plane(args.plane.index());
        Opened { workspace, texture }
    });
    let source = MockSource { image, paste: SizedTexture::new(opened.texture.id(), vec2(PASTE as f32, PASTE as f32)) };
    let out = opened.workspace.show(ui.ctx(), &MockScene, &source);
    // The fake stamp preview is the layer itself, so every ticket is answered in the pass that asked it.
    if let Some(ticket) = out.preview_wanted {
        opened.workspace.preview_ready(ticket);
    }
    if out.outcome.is_some_and(Outcome::closes) {
        dialog_closed(ui, &mut args.open);
    }
}
