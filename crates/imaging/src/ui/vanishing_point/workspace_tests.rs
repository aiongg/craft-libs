//! Tests of Vanishing Point's drawing and workspace (task WS-VP2).

use egui::epaint::ColorMode;
use egui::os::OperatingSystem;
use egui::{Color32, ColorImage, Event, Key, Modifiers, PointerButton, Rect, Shape, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::quad::Homography;
use craft_ui::canvas::{CanvasView, PixelRect, RasterSource};
use craft_ui::dialog::Outcome;
use craft_ui::theme::{Base, current, install};

use super::planes::{PlaneCheck, TornPlane, VanishingPlane, VanishingScene};
use super::{
    PasteImage, StampLimits, VanishingPointBar, VanishingPointEditor, VanishingPointEvent, VanishingPointOpen, VanishingPointSession,
    VanishingPointSettings, VanishingPointSource, VanishingPointStyle, VanishingPointWorkspace, VanishingTool, paint_editor,
};

/// The engine's clone limits, as the app gives them.
const LIMITS: StampLimits = StampLimits { strokes: 4096, dabs: 200_000 };

/// A scene without perspective maths: planes as they are, the check by index.
struct Checks(Vec<PlaneCheck>);

impl VanishingScene for Checks {
    fn normalized(&self, plane: VanishingPlane) -> VanishingPlane {
        plane
    }

    fn resolve(&self, planes: &[VanishingPlane], _: &[Option<TornPlane>]) -> Vec<VanishingPlane> {
        planes.to_vec()
    }

    fn check(&self, _: &[VanishingPlane], index: usize) -> PlaneCheck {
        self.0.get(index).copied().unwrap_or(PlaneCheck::Valid)
    }

    fn plane_at(&self, _: &[VanishingPlane], _: [f64; 2]) -> Option<usize> {
        None
    }

    fn frame(&self, planes: &[VanishingPlane], index: usize) -> Option<(Homography, f64, f64)> {
        let p = planes.get(index)?;
        let len = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        Some((p.homography()?, len(p.corners[0], p.corners[1]), len(p.corners[0], p.corners[3])))
    }

    fn stamp_ring(&self, _: &[VanishingPlane], _: [f64; 2], _: f64) -> Option<Vec<Point>> {
        None
    }

    fn clone_source(&self, _: &[VanishingPlane], _: [f64; 2], _: [f64; 2], _: [f64; 2]) -> Option<[f64; 2]> {
        None
    }
}

/// A solid layer, with or without a clipboard image.
struct Layer {
    size: [u32; 2],
    clip: bool,
}

impl RasterSource for Layer {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_rgb(90, 120, 160));
    }
}

impl VanishingPointSource for Layer {
    fn image(&self) -> &dyn RasterSource {
        self
    }

    fn stamp_preview(&self) -> Option<&dyn RasterSource> {
        None
    }

    fn paste(&self) -> Option<egui::load::SizedTexture> {
        self.clip.then(|| egui::load::SizedTexture::new(egui::TextureId::default(), vec2(40.0, 30.0)))
    }
}

fn square(x: f64, y: f64) -> VanishingPlane {
    VanishingPlane { corners: [[x, y], [x + 100.0, y], [x + 100.0, y + 100.0], [x, y + 100.0]] }
}

// Valid, questionable and invalid planes draw in accent, warning and danger; an invalid plane draws
// its outline only.
#[test]
fn vp_plane_colours_follow_the_check() {
    let planes = vec![square(20.0, 20.0), square(150.0, 20.0), square(280.0, 20.0)];
    let mut h = Harness::builder().with_size(vec2(420.0, 200.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, planes: &mut Vec<VanishingPlane>| {
            let style = VanishingPointStyle::of(&current(ui.ctx()));
            let session = VanishingPointSession { planes: planes.clone(), ..VanishingPointSession::default() };
            let scene = Checks(vec![PlaneCheck::Valid, PlaneCheck::Questionable, PlaneCheck::Invalid]);
            let view = CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(420.0, 200.0)), Point::new(210.0, 100.0), 1.0);
            let layer = Layer { size: [420, 200], clip: false };
            let painter = ui.painter().clone();
            let options = VanishingPointSettings { grid_size: 20.0, ..VanishingPointSettings::default() };
            paint_editor(&painter, &view, &VanishingPointEditor::new(LIMITS), &session, &options, &scene, &layer, &style);
        },
        planes,
    );
    h.run();
    let style = VanishingPointStyle::of(&current(&h.ctx));
    let mut shapes = Vec::new();
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(inner) => inner.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    h.output().shapes.iter().for_each(|clipped| flatten(&clipped.shape, &mut shapes));
    let paths: Vec<(Color32, f32)> = shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Path(path) => match &path.stroke.color {
                ColorMode::Solid(colour) => Some((*colour, path.stroke.width)),
                ColorMode::UV(_) => None,
            },
            _ => None,
        })
        .collect();
    for colour in [style.valid, style.questionable, style.invalid] {
        assert!(paths.contains(&(colour, style.line)), "an outline in {colour:?}");
    }
    let grid = |colour: Color32| {
        let grid = colour.gamma_multiply(style.grid_alpha);
        shapes.iter().filter(|s| matches!(s, Shape::LineSegment { stroke, .. } if stroke.color == grid)).count()
    };
    assert!(grid(style.valid) > 0 && grid(style.questionable) > 0, "valid and questionable planes have a grid");
    assert_eq!(grid(style.invalid), 0, "an invalid plane has only its outline");
}

// The pasted image is a mesh tinted white at the style's alpha (PhotoCraft: white at 230).
#[test]
fn vp_paste_is_tinted_white_at_the_styles_alpha() {
    let mut h = Harness::builder().with_size(vec2(420.0, 200.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, _: &mut ()| {
            let style = VanishingPointStyle::of(&current(ui.ctx()));
            let session = VanishingPointSession { planes: vec![square(20.0, 20.0)], ..VanishingPointSession::default() };
            let view = CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(420.0, 200.0)), Point::new(210.0, 100.0), 1.0);
            let layer = Layer { size: [420, 200], clip: true };
            let settings = VanishingPointSettings { paste: true, ..VanishingPointSettings::default() };
            let painter = ui.painter().clone();
            paint_editor(&painter, &view, &VanishingPointEditor::new(LIMITS), &session, &settings, &Checks(Vec::new()), &layer, &style);
        },
        (),
    );
    h.run();
    let style = VanishingPointStyle::of(&current(&h.ctx));
    let mut tints = Vec::new();
    fn meshes(shape: &Shape, out: &mut Vec<Color32>) {
        match shape {
            Shape::Vec(inner) => inner.iter().for_each(|s| meshes(s, out)),
            Shape::Mesh(mesh) => out.extend(mesh.vertices.iter().map(|v| v.color)),
            _ => {}
        }
    }
    h.output().shapes.iter().for_each(|clipped| meshes(&clipped.shape, &mut tints));
    assert!(!tints.is_empty(), "the paste is drawn");
    assert!(tints.iter().all(|c| *c == Color32::WHITE.gamma_multiply(style.paste_alpha)), "{:?}", tints.first());
    assert_eq!(tints[0].a(), 230);
}

struct App {
    workspace: VanishingPointWorkspace,
    scene: Checks,
    layer: Layer,
    outcome: Option<Outcome>,
    events: Vec<VanishingPointEvent>,
}

fn workspace(open: VanishingPointOpen, clip: bool) -> Harness<'static, App> {
    let size = open.size;
    let app = App {
        workspace: VanishingPointWorkspace::new(open),
        scene: Checks(Vec::new()),
        layer: Layer { size, clip },
        outcome: None,
        events: Vec::new(),
    };
    let mut h = HarnessBuilder::default()
        .with_size(vec2(1000.0, 900.0))
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let out = app.workspace.show(&ui.ctx().clone(), &app.scene, &app.layer);
                app.outcome = app.outcome.or(out.outcome);
                app.events.extend(out.events);
            },
            app,
        );
    h.run();
    h
}

fn click(h: &mut Harness<'_, App>, at: egui::Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run();
    }
}

// The workspace has the strip, the bar and the side footer; Enter is OK and Esc Cancel.
#[test]
fn vp_opens_in_its_own_window_with_ok_and_cancel() {
    let open = || VanishingPointOpen::new([800, 600], "Layer 1", LIMITS);
    let h = workspace(open(), false);
    for label in ["Tools", "Tool options", "Edit Plane Tool", "Create Plane Tool", "Stamp Tool", "Hand Tool", "Zoom Tool", "OK", "Cancel"] {
        assert!(h.query_all_by_label_contains(label).count() > 0, "{label} is shown");
    }
    assert!(h.query_all_by_label_contains("Vanishing Point (Layer 1").count() > 0, "the title names the layer and the zoom");

    let mut h = workspace(open(), false);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().outcome, Some(Outcome::Ok));

    // Esc drops half-clicked corners first, then cancels.
    let mut h = workspace(open(), false);
    click(&mut h, pos2(300.0, 400.0));
    assert_eq!(h.state().workspace.editor.pending().len(), 1);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().outcome, None);
    assert!(h.state().workspace.editor.pending().is_empty());
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().outcome, Some(Outcome::Cancel));
}

struct BarState {
    options: VanishingPointSettings,
    tool: VanishingTool,
    clip: bool,
    strokes: bool,
}

fn bar(tool: VanishingTool, clip: bool, paste: bool, strokes: bool) -> Harness<'static, BarState> {
    let state = BarState { options: VanishingPointSettings { paste, ..VanishingPointSettings::default() }, tool, clip, strokes };
    let mut h = Harness::builder().with_size(vec2(1400.0, 60.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, state: &mut BarState| {
            let _ = VanishingPointBar::new("bar", state.tool, &mut state.options, PasteImage { at: [0.25, 0.25], width: 0.5 }, state.clip)
                .has_strokes(state.strokes)
                .show(ui);
        },
        state,
    );
    h.run();
    h
}

// The bar shows Diameter, Hardness, Opacity and Aligned only for Stamp, Paste and Width only with a
// clipboard image, and New Layer with an image, strokes or the Stamp tool (PhotoCraft's rule).
#[test]
fn vp_bar_shows_each_tools_options() {
    let shown = |h: &Harness<'_, BarState>, label: &str| h.query_all_by_label_contains(label).count() > 0;
    let h = bar(VanishingTool::EditPlane, false, false, false);
    assert!(shown(&h, "Grid Size"));
    for label in ["Diameter", "Hardness", "Opacity", "Aligned", "Paste", "Width", "New Layer"] {
        assert!(!shown(&h, label), "{label} is not shown for Edit Plane without a clipboard image or strokes");
    }
    assert!(shown(&bar(VanishingTool::EditPlane, false, false, true), "New Layer"), "strokes show New Layer");
    let h = bar(VanishingTool::Stamp, false, false, false);
    for label in ["Diameter", "Hardness", "Opacity", "Aligned", "Grid Size", "New Layer"] {
        assert!(shown(&h, label), "{label} is shown for Stamp");
    }
    assert!(!shown(&h, "Paste"));
    let h = bar(VanishingTool::CreatePlane, true, false, false);
    assert!(shown(&h, "Paste") && shown(&h, "New Layer") && !shown(&h, "Width") && !shown(&h, "Diameter"));
    let h = bar(VanishingTool::CreatePlane, true, true, false);
    assert!(shown(&h, "Paste") && shown(&h, "Width"));
}

/// The screen position of plane 1's top-left corner part, from the host's AccessKit node.
fn first_corner(h: &Harness<'_, App>) -> egui::Pos2 {
    h.get_by_label_contains("Plane 1, top-left corner").rect().center()
}

/// A workspace on Edit Plane with one plane, a corner press and a drag in progress, with the plane
/// as it was before the press.
fn mid_drag() -> (Harness<'static, App>, VanishingPlane) {
    let mut open = VanishingPointOpen::new([800, 600], "Layer 1", LIMITS);
    let plane = VanishingPlane { corners: [[400.0, 300.0], [600.0, 300.0], [600.0, 500.0], [400.0, 500.0]] };
    open.session = VanishingPointSession { planes: vec![plane], torn: vec![None], ..VanishingPointSession::default() };
    let mut h = workspace(open, false);
    h.key_press(Key::V);
    h.run();
    let at = first_corner(&h);
    h.event(Event::PointerMoved(at));
    h.run();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run();
    h.event(Event::PointerMoved(at + vec2(30.0, 20.0)));
    h.run();
    assert!(h.state().workspace.editor.busy(), "the corner drag runs");
    (h, plane)
}

// A letter that picks another tool mid-drag, and ⌘Z, end the drag with a Cancel carrying the
// reason, put the plane back, and leave the editor not busy so the history settles.
#[test]
fn vp_a_tool_letter_or_undo_mid_drag_cancels_the_gesture() {
    let cancelled = |h: &Harness<'_, App>| {
        h.state().events.iter().find_map(|e| match e {
            VanishingPointEvent::Planes(craft_ui::edit::EditEvent::Cancel { reason, .. }) => Some(*reason),
            _ => None,
        })
    };
    let (mut h, before) = mid_drag();
    assert_ne!(h.state().workspace.session.planes[0], before, "the drag moved the corner");
    h.key_press(Key::S);
    h.run();
    assert_eq!(cancelled(&h), Some(craft_ui::edit::CancelReason::Removed));
    assert!(!h.state().workspace.editor.busy());
    assert_eq!(h.state().workspace.editor.tool(), VanishingTool::Stamp);
    assert_eq!(h.state().workspace.session.planes[0], before, "the plane is back");

    let (mut h, _) = mid_drag();
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(cancelled(&h), Some(craft_ui::edit::CancelReason::UndoDuringGesture));
    assert!(!h.state().workspace.editor.busy());
}
