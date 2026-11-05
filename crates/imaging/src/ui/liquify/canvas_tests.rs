//! Tests of Liquify's canvas and workspace (task WS-LQ2).

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Color32, ColorImage, Event, Key, Modifiers, PointerButton, Pos2, Shape, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::canvas::LayeredImage;
use super::{
    BackdropLayering, BackdropMode, BackdropUse, LiquifyEvent, LiquifyMesh, LiquifyOpen, LiquifySource, LiquifyTiming, LiquifyTool, LiquifyWorkspace,
    MeshSize,
};
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::{Outcome, Step};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::theme::{Base, install};

const WINDOW: egui::Vec2 = vec2(1000.0, 900.0);
const CTRL_ALT: Modifiers = Modifiers { alt: true, ctrl: true, ..Modifiers::NONE };

/// A solid picture.
struct Solid {
    size: [u32; 2],
    colour: Color32,
}

impl RasterSource for Solid {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _region: PixelRect, _factor: u32, out: &mut ColorImage) {
        out.pixels.fill(self.colour);
    }
}

/// The app's side: a picture, an optional backdrop and two layers.
struct Source {
    image: Solid,
    backdrop: Option<Solid>,
}

impl LiquifySource for Source {
    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn freeze_mask(&self) -> Option<(&dyn RasterSource, u32)> {
        None
    }

    fn mesh(&self, _size: MeshSize) -> Option<LiquifyMesh> {
        None
    }

    fn backdrop(&self) -> Option<&dyn RasterSource> {
        self.backdrop.as_ref().map(|b| b as &dyn RasterSource)
    }

    fn layers(&self) -> Vec<(u64, String)> {
        vec![(7, "Sky".to_owned()), (9, "Lake".to_owned())]
    }
}

struct App {
    workspace: LiquifyWorkspace,
    source: Source,
    events: Vec<LiquifyEvent>,
    outcome: Option<Outcome>,
    /// Whether any pass reported `settings_changed`, since the test last cleared it.
    changed: bool,
    timing: Option<LiquifyTiming>,
}

fn harness() -> Harness<'static, App> {
    let size = [800, 600];
    let app = App {
        workspace: LiquifyWorkspace::new(LiquifyOpen::new(size, "Photo")),
        source: Source { image: Solid { size, colour: Color32::from_rgb(40, 120, 200) }, backdrop: None },
        events: Vec::new(),
        outcome: None,
        changed: false,
        timing: None,
    };
    let mut h = HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let out = app.workspace.show(&ui.ctx().clone(), &app.source);
                app.events.extend(out.events);
                app.outcome = app.outcome.or(out.outcome);
                app.changed |= out.settings_changed;
                app.timing = out.timing;
            },
            app,
        );
    h.run();
    h
}

/// A point inside the image, left of the side column.
fn canvas_point(dx: f32, dy: f32) -> Pos2 {
    pos2(380.0 + dx, 350.0 + dy)
}

fn pointer(h: &mut Harness<'_, App>, to: Pos2) {
    h.event(Event::PointerMoved(to));
    h.run();
}

fn button(h: &mut Harness<'_, App>, button: PointerButton, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event_modifiers(Event::PointerButton { pos, button, pressed, modifiers }, modifiers);
    h.run();
}

fn drag(h: &mut Harness<'_, App>, with: PointerButton, from: Pos2, to: Pos2, modifiers: Modifiers) {
    pointer(h, from);
    button(h, with, from, true, modifiers);
    pointer(h, from + (to - from) * 0.5);
    pointer(h, to);
    button(h, with, to, false, modifiers);
}

fn strokes(events: &[LiquifyEvent]) -> usize {
    events.iter().filter(|e| matches!(e, LiquifyEvent::Stroke(_))).count()
}

/// The circles of the last pass: (centre, radius).
fn circles(h: &Harness<'_, App>) -> Vec<(Pos2, f32)> {
    fn walk(shape: &Shape, out: &mut Vec<(Pos2, f32)>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
            Shape::Circle(c) => out.push((c.center, c.radius)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|clipped| walk(&clipped.shape, &mut out));
    out
}

/// Whether the pass left a bitmap pointer (the brush outline) for the OS.
fn outline_shown(h: &Harness<'_, App>) -> bool {
    h.output().platform_output.cursor_image.is_some()
}

// Alt + right-drag and Control + Alt + drag change the size (horizontal) and the density
// (vertical), keep the outline at the anchor, and report no stroke.
#[test]
fn liquify_alt_right_drag_resizes_size_and_density_and_paints_nothing() {
    let mut h = harness();
    let start = h.state().workspace.settings.brush;
    let from = canvas_point(0.0, 0.0);

    drag(&mut h, PointerButton::Secondary, from, from + vec2(40.0, 30.0), Modifiers::ALT);
    let brush = h.state().workspace.settings.brush;
    assert!(brush.size > start.size, "right widens the brush: {} -> {}", start.size, brush.size);
    assert!(brush.density > start.density, "down raises the density: {} -> {}", start.density, brush.density);
    assert_eq!((strokes(&h.state().events), h.state().workspace.session.len()), (0, 0), "a resize paints nothing");
    assert!(!h.state().workspace.canvas.busy(), "the release ended the resize");

    let mid = h.state().workspace.settings.brush;
    drag(&mut h, PointerButton::Primary, from, from + vec2(-20.0, -20.0), CTRL_ALT);
    let brush = h.state().workspace.settings.brush;
    assert!(brush.size < mid.size && brush.density < mid.density, "Control + Alt + drag resizes too: {mid:?} -> {brush:?}");
    assert_eq!((strokes(&h.state().events), h.state().workspace.session.len()), (0, 0));

    // The outline stays where the press was while the pointer moves on.
    let anchor = canvas_point(60.0, 40.0);
    pointer(&mut h, anchor);
    button(&mut h, PointerButton::Secondary, anchor, true, Modifiers::ALT);
    pointer(&mut h, anchor + vec2(50.0, 0.0));
    assert!(h.state().workspace.canvas.busy());
    assert!(circles(&h).iter().any(|(centre, _)| centre.distance(anchor) < 0.5), "the outline is at the anchor: {:?}", circles(&h));
    assert!(!outline_shown(&h), "the pointer's own outline is hidden meanwhile");
    button(&mut h, PointerButton::Secondary, anchor + vec2(50.0, 0.0), false, Modifiers::ALT);
}

// Alt + primary paints with the alternate tool (the host gives the tool the primary press with Alt
// held, as well as the secondary press that resizes).
#[test]
fn liquify_alt_primary_drag_paints_with_the_alternate_tool() {
    let mut h = harness();
    h.state_mut().workspace.settings.tool = LiquifyTool::TwirlClockwise;
    let from = canvas_point(0.0, 0.0);
    // The twirls act while the brush rests and keep repainting, so the pass is stepped, not run.
    let mut step = |event: Event| {
        h.event_modifiers(event, Modifiers::ALT);
        h.step();
    };
    let to = from + vec2(40.0, 30.0);
    step(Event::PointerMoved(from));
    step(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::ALT });
    step(Event::PointerMoved(from + (to - from) * 0.5));
    step(Event::PointerMoved(to));
    step(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::ALT });
    let tools: Vec<_> = h
        .state()
        .events
        .iter()
        .filter_map(|e| match e {
            LiquifyEvent::Stroke(EditEvent::Commit { value, .. }) => Some(value.tool),
            _ => None,
        })
        .collect();
    assert_eq!(tools, [LiquifyTool::TwirlCounterclockwise]);
    assert!(!h.state().workspace.canvas.busy());
}

// With the Size number field focused, ⌘Z still undoes the last stroke (Replay is reported) and
// the field keeps its text.
#[test]
fn liquify_undo_works_while_a_number_field_has_focus() {
    let mut h = harness();
    drag(&mut h, PointerButton::Primary, canvas_point(-40.0, 0.0), canvas_point(40.0, 10.0), Modifiers::NONE);
    assert_eq!(h.state().workspace.session.len(), 1, "the stroke is recorded");
    assert!(h.state().events.iter().any(|e| matches!(e, LiquifyEvent::Stroke(EditEvent::Commit { .. }))));
    h.state_mut().events.clear();

    h.get_by_role_and_label(Role::SpinButton, "Size").click();
    h.run();
    let before = h.get_by_role_and_label(Role::SpinButton, "Size").value();
    assert!(h.ctx.text_edit_focused(), "the number field has the keyboard");

    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert!(h.state().workspace.session.is_empty(), "the stroke is undone");
    assert!(h.state().events.iter().any(|e| matches!(e, LiquifyEvent::Replay(steps) if steps.is_empty())), "{:?}", h.state().events);
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, "Size").value(), before, "the field keeps its text");

    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    h.run();
    assert_eq!(h.state().workspace.session.len(), 1, "redo brings it back");
}

// In Front, Behind and Blend tint the image and the backdrop as the table in `BackdropLayering`
// says; no backdrop leaves the image white.
#[test]
fn liquify_backdrop_layers_follow_the_mode_and_opacity() {
    let half = Color32::from_white_alpha(128);
    let white = Color32::WHITE;
    let layering = |shown, mode| BackdropLayering::of(shown, mode, 50.0);
    assert_eq!(layering(false, BackdropMode::Blend), BackdropLayering { image: white, behind: None, front: None });
    assert_eq!(layering(true, BackdropMode::InFront), BackdropLayering { image: white, behind: None, front: Some(half) });
    assert_eq!(layering(true, BackdropMode::Behind), BackdropLayering { image: half, behind: Some(white), front: None });
    assert_eq!(layering(true, BackdropMode::Blend), BackdropLayering { image: half, behind: None, front: Some(half) });
    let full = BackdropLayering::of(true, BackdropMode::Blend, 100.0);
    assert_eq!((full.image.a(), full.front), (0, Some(white)), "Blend at 100 % shows the backdrop alone");

    // The composite: a red image over a blue backdrop.
    let image = Solid { size: [1, 1], colour: Color32::from_rgb(255, 0, 0) };
    let backdrop = Solid { size: [1, 1], colour: Color32::from_rgb(0, 0, 255) };
    let pixel = |mode| {
        let layered = LayeredImage { image: &image, backdrop: &backdrop, layering: BackdropLayering::of(true, mode, 50.0) };
        let mut out = ColorImage::filled([1, 1], Color32::TRANSPARENT);
        layered.read(PixelRect::new(0, 0, 1, 1), 1, &mut out);
        out.pixels[0].to_array()
    };
    let near = |got: [u8; 4], want: [u8; 4]| got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 2);
    assert!(near(pixel(BackdropMode::InFront), [127, 0, 128, 255]), "{:?}", pixel(BackdropMode::InFront));
    assert!(near(pixel(BackdropMode::Behind), [128, 0, 127, 255]), "{:?}", pixel(BackdropMode::Behind));
    assert!(near(pixel(BackdropMode::Blend), [85, 0, 170, 192]), "{:?}", pixel(BackdropMode::Blend));
}

// Turning Show Backdrop on reports BackdropWanted(None); picking a layer in Use reports its id.
#[test]
fn liquify_show_backdrop_asks_the_app_for_the_backdrop() {
    let mut h = harness();
    assert!(!h.state().events.iter().any(|e| matches!(e, LiquifyEvent::BackdropWanted(_))), "nothing asked while it is off");

    h.get_by_role_and_label(Role::CheckBox, "Show Backdrop").click();
    h.run();
    assert!(h.state().workspace.settings.show_backdrop);
    let asked: Vec<_> = h.state().events.iter().filter(|e| matches!(e, LiquifyEvent::BackdropWanted(_))).collect();
    assert_eq!(asked, [&LiquifyEvent::BackdropWanted(None)]);

    h.state_mut().events.clear();
    h.get_by_role_and_label(Role::ComboBox, "Use").click();
    h.run();
    h.get_by_label("Lake").click();
    h.run();
    assert_eq!(h.state().workspace.settings.backdrop, BackdropUse::Layer(9));
    assert_eq!(h.state().events, [LiquifyEvent::BackdropWanted(Some(9))]);
}

// ⌘+ zooms the workspace's view, not the canvas behind the dialog; the title shows the new
// percent.
#[test]
fn liquify_zoom_keys_zoom_the_preview() {
    let mut h = harness();
    assert!(h.state().workspace.view.is_fit());
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    h.run();
    assert_eq!(h.state().workspace.view.zoom(), 1.0);
    assert!(h.query_all_by_label_contains("Liquify (Photo, 100%)").next().is_some(), "the title shows 100%");

    h.key_press_modifiers(Modifiers::COMMAND, Key::Plus);
    h.run();
    let zoom = h.state().workspace.view.zoom();
    assert!(zoom > 1.0, "zoomed in from 100 %: {zoom}");
    let percent = format!("{:.0}%", zoom * 100.0);
    assert!(h.query_all_by_label_contains(&format!("Liquify (Photo, {percent})")).next().is_some(), "the title shows {percent}");
}

// The brush outline follows the pointer, hides off the canvas, and stays at the anchor while the
// brush is resized.
#[test]
fn liquify_brush_outline_follows_the_pointer_except_while_resizing() {
    let mut h = harness();
    assert_eq!(h.state().workspace.settings.tool, LiquifyTool::ForwardWarp);
    pointer(&mut h, canvas_point(0.0, 0.0));
    assert!(outline_shown(&h), "the outline is the pointer over the canvas");

    // Over the Properties column the outline is gone.
    pointer(&mut h, pos2(900.0, 300.0));
    assert!(!outline_shown(&h), "no outline off the canvas");

    pointer(&mut h, canvas_point(0.0, 0.0));
    assert!(outline_shown(&h));
    let anchor = canvas_point(20.0, 20.0);
    pointer(&mut h, anchor);
    button(&mut h, PointerButton::Secondary, anchor, true, Modifiers::ALT);
    pointer(&mut h, anchor + vec2(30.0, 0.0));
    assert!(!outline_shown(&h), "the pointer hides while resizing");
    assert!(circles(&h).iter().any(|(centre, _)| centre.distance(anchor) < 0.5), "the outline stays at the anchor");
    button(&mut h, PointerButton::Secondary, anchor + vec2(30.0, 0.0), false, Modifiers::ALT);
    pointer(&mut h, anchor + vec2(30.0, 0.0));
    assert!(outline_shown(&h), "the outline follows the pointer again");
}

// Esc during a stroke cancels the stroke only: Cancel is followed by a Replay of the recorded
// steps, nothing is recorded and the dialog stays open; a second Esc is the dialog's Cancel.
#[test]
fn liquify_escape_cancels_the_stroke_and_replays_then_the_next_cancels_the_dialog() {
    let mut h = harness();
    let from = canvas_point(0.0, 0.0);
    pointer(&mut h, from);
    button(&mut h, PointerButton::Primary, from, true, Modifiers::NONE);
    pointer(&mut h, from + vec2(30.0, 0.0));
    assert!(h.state().workspace.canvas.busy());
    h.state_mut().events.clear();

    h.key_press(Key::Escape);
    h.run();
    let events = &h.state().events;
    assert!(
        matches!(events.as_slice(), [LiquifyEvent::Stroke(EditEvent::Cancel { .. }), LiquifyEvent::Replay(steps)] if steps.is_empty()),
        "{events:?}"
    );
    assert!(h.state().workspace.session.is_empty(), "nothing was recorded");
    assert_eq!(h.state().outcome, None, "the dialog stays open");
    button(&mut h, PointerButton::Primary, from + vec2(30.0, 0.0), false, Modifiers::NONE);

    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().outcome, Some(Outcome::Cancel));
}

// The Freeze Lasso reports only its Commit, with amount 1; Alt at the press thaws (amount 0).
#[test]
fn liquify_freeze_lasso_commits_a_polygon_and_alt_thaws() {
    let mut h = harness();
    h.state_mut().workspace.settings.tool = LiquifyTool::FreezeLasso;
    let corners = [canvas_point(0.0, 0.0), canvas_point(80.0, 0.0), canvas_point(80.0, 60.0), canvas_point(0.0, 60.0)];
    let run = |h: &mut Harness<'_, App>, modifiers: Modifiers| {
        pointer(h, corners[0]);
        button(h, PointerButton::Primary, corners[0], true, modifiers);
        for corner in &corners[1..] {
            pointer(h, *corner);
        }
        button(h, PointerButton::Primary, corners[3], false, modifiers);
    };
    run(&mut h, Modifiers::NONE);
    run(&mut h, Modifiers::ALT);
    let amounts: Vec<_> = h
        .state()
        .events
        .iter()
        .filter_map(|e| match e {
            LiquifyEvent::Stroke(EditEvent::Commit { value, .. }) => Some((value.tool, value.amount, value.points.len() >= 4)),
            _ => None,
        })
        .collect();
    assert_eq!(amounts, [(LiquifyTool::FreezeLasso, Some(1.0), true), (LiquifyTool::FreezeLasso, Some(0.0), true)]);
    assert_eq!(strokes(&h.state().events), 2, "a lasso reports only its Commit");
    assert_eq!(h.state().workspace.session.len(), 2);
}

// A tool letter pressed mid-stroke cancels the stroke through the host with `Removed`, then the app's
// field is rebuilt by a Replay; the canvas is idle, so the history settles, and the release that
// follows records nothing.
#[test]
fn liquify_a_letter_mid_stroke_cancels_the_stroke_and_leaves_the_canvas_idle() {
    let mut h = harness();
    let from = canvas_point(0.0, 0.0);
    pointer(&mut h, from);
    button(&mut h, PointerButton::Primary, from, true, Modifiers::NONE);
    pointer(&mut h, from + vec2(30.0, 0.0));
    assert!(h.state().workspace.canvas.busy());
    h.state_mut().events.clear();

    h.key_press(Key::R);
    h.run();
    assert_eq!(h.state().workspace.settings.tool, LiquifyTool::Reconstruct, "the letter picked the tool");
    assert!(!h.state().workspace.canvas.busy(), "the stroke ended with the old tool");
    let events = &h.state().events;
    assert!(
        matches!(
            events.as_slice(),
            [LiquifyEvent::Stroke(EditEvent::Cancel { reason: CancelReason::Removed, .. }), LiquifyEvent::Replay(steps)] if steps.is_empty()
        ),
        "{events:?}"
    );

    button(&mut h, PointerButton::Primary, from + vec2(30.0, 0.0), false, Modifiers::NONE);
    assert!(!h.state().events.iter().any(|e| matches!(e, LiquifyEvent::Stroke(EditEvent::Commit { .. }))), "the new tool got no release");
    assert!(h.state().workspace.session.is_empty());
}

// ⌘Z mid-stroke cancels the stroke with `UndoDuringGesture`, then steps the history: one Replay of
// the stepped session follows the Cancel.
#[test]
fn liquify_undo_mid_stroke_cancels_the_stroke_then_steps() {
    let mut h = harness();
    drag(&mut h, PointerButton::Primary, canvas_point(-40.0, 0.0), canvas_point(40.0, 10.0), Modifiers::NONE);
    assert_eq!(h.state().workspace.session.len(), 1);
    let from = canvas_point(0.0, 40.0);
    pointer(&mut h, from);
    button(&mut h, PointerButton::Primary, from, true, Modifiers::NONE);
    pointer(&mut h, from + vec2(30.0, 0.0));
    assert!(h.state().workspace.canvas.busy());
    h.state_mut().events.clear();

    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    let events = &h.state().events;
    assert!(
        matches!(
            events.as_slice(),
            [LiquifyEvent::Stroke(EditEvent::Cancel { reason: CancelReason::UndoDuringGesture, .. }), LiquifyEvent::Replay(steps)] if steps.is_empty()
        ),
        "{events:?}"
    );
    assert!(h.state().workspace.session.is_empty(), "the earlier stroke is undone");
    assert!(!h.state().workspace.canvas.busy());
}

// `step` from the app's Edit menu reports whether it stepped and queues the Replay for the next
// pass.
#[test]
fn liquify_step_from_the_menu_queues_the_replay_for_the_next_pass() {
    let mut h = harness();
    drag(&mut h, PointerButton::Primary, canvas_point(-40.0, 0.0), canvas_point(40.0, 10.0), Modifiers::NONE);
    h.state_mut().events.clear();

    assert!(h.state_mut().workspace.step(Step::Undo));
    assert!(h.state().workspace.session.is_empty());
    assert!(!h.state_mut().workspace.step(Step::Undo), "nothing left to undo");
    h.run();
    let events = &h.state().events;
    assert!(matches!(events.as_slice(), [LiquifyEvent::Replay(steps)] if steps.is_empty()), "{events:?}");
}

// A brush resize writes the brush, which the output reports as a settings change; a stroke
// reports its timing.
#[test]
fn liquify_output_reports_resize_changes_and_the_stroke_timing() {
    let mut h = harness();
    assert!(!h.state().changed, "nothing changed yet");
    let from = canvas_point(0.0, 0.0);
    drag(&mut h, PointerButton::Secondary, from, from + vec2(40.0, 0.0), Modifiers::ALT);
    assert!(h.state().changed, "Alt + right-drag changed the brush");
    assert_eq!(h.state().timing, None, "a resize is not a stroke");

    drag(&mut h, PointerButton::Primary, canvas_point(-40.0, 0.0), canvas_point(40.0, 10.0), Modifiers::NONE);
    assert!(h.state().timing.is_some_and(|t| t.dabs >= 2), "{:?}", h.state().timing);
}
