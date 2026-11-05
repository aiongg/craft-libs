//! Tests of Content-Aware Fill's panels and workspace.

use egui::os::OperatingSystem;
use egui::{Color32, ColorImage, Event, Key, Modifiers, PointerButton, Shape, TextureId, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::overlay::tint_image;
use super::{
    ContentAwareFillWorkspace, FillAreaEdit, FillEvent, FillLimits, FillOpen, FillOutput, FillSource, FillTool, OVERLAY_COLOR_KEY, SamplingPlane,
};
use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Rect as DocRect;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::Outcome;
use craft_ui::dialog::full_window::PreviewTicket;
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::theme::{Base, install};

const WINDOW: egui::Vec2 = vec2(1200.0, 800.0);

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

/// The app's side: the picture, a sampling plane over the left half, a fill area in the middle,
/// and a preview.
struct Source {
    image: Solid,
    preview: Solid,
    sampling: SamplingPlane,
    fill_area: SamplingPlane,
}

impl FillSource for Source {
    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn sampling(&self) -> Option<&SamplingPlane> {
        Some(&self.sampling)
    }

    fn fill_area(&self) -> Option<&SamplingPlane> {
        Some(&self.fill_area)
    }

    fn sampling_window(&self) -> Option<DocRect> {
        Some(DocRect::new(100.0, 100.0, 200.0, 150.0))
    }

    fn preview(&self) -> Option<&dyn RasterSource> {
        Some(&self.preview)
    }
}

struct App {
    workspace: ContentAwareFillWorkspace,
    source: Source,
    last: Option<FillOutput>,
    outcomes: Vec<Outcome>,
    events: Vec<FillEvent>,
    tickets: Vec<PreviewTicket>,
    settings_changes: usize,
}

fn harness(os: OperatingSystem) -> Harness<'static, App> {
    let size = [400, 300];
    let plane = |origin: [i32; 2], cells: [u32; 2], bits: Vec<bool>| SamplingPlane { origin, size: cells, texel: 1, bits };
    let app = App {
        workspace: ContentAwareFillWorkspace::new(FillOpen::new(size, "Layer 1", FillLimits { edits: 1000, points: 200_000 })),
        source: Source {
            image: Solid { size, colour: Color32::from_rgb(40, 120, 200) },
            preview: Solid { size, colour: Color32::from_rgb(200, 120, 40) },
            sampling: plane([0, 0], [4, 1], vec![true, true, false, false]),
            fill_area: plane([0, 0], [1, 1], vec![true]),
        },
        last: None,
        outcomes: Vec::new(),
        events: Vec::new(),
        tickets: Vec::new(),
        settings_changes: 0,
    };
    let mut h =
        HarnessBuilder::default().with_size(WINDOW).with_os(os).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let out = app.workspace.show(&ui.ctx().clone(), &app.source);
                app.outcomes.extend(out.outcome);
                app.events.extend(out.events.iter().cloned());
                app.tickets.extend(out.preview_wanted);
                app.settings_changes += usize::from(out.settings_changed);
                app.last = Some(out);
            },
            app,
        );
    h.run_steps(5);
    h
}

/// The textured meshes of the last pass that are not the font atlas: images and overlays.
fn pictures(h: &Harness<'_, App>) -> usize {
    fn walk(shape: &Shape) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(walk).sum(),
            Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => 1,
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|clipped| walk(&clipped.shape)).sum()
}

/// A node's screen bounds.
fn bounds(node: &egui_kittest::Node<'_>) -> egui::accesskit::Rect {
    let Some(rect) = node.accesskit_node().bounding_box() else { panic!("the node has bounds") };
    rect
}

// Apply reports `Outcome::Apply` and the workspace stays open; OK reports `Outcome::Ok`; Cancel
// `Outcome::Cancel`.
#[test]
fn fill_apply_keeps_the_workspace_open_and_ok_closes() {
    let mut h = harness(OperatingSystem::Nix);
    h.get_by_label("Apply").click();
    h.run_steps(5);
    assert_eq!(h.state().outcomes, [Outcome::Apply]);
    h.get_by_label("Apply").click();
    h.run_steps(5);
    assert_eq!(h.state().outcomes, [Outcome::Apply, Outcome::Apply], "the workspace shows again after an Apply");
    h.get_by_label("OK").click();
    h.run_steps(5);
    assert_eq!(h.state().outcomes.last(), Some(&Outcome::Ok));
    let mut h = harness(OperatingSystem::Nix);
    h.get_by_label("Cancel").click();
    h.run_steps(5);
    assert_eq!(h.state().outcomes, [Outcome::Cancel]);
}

// The tint covers the sampling plane outside the fill area (Indicates › Excluded Area: the rest);
// ⌘H hides it without changing Show Sampling Area.
#[test]
fn fill_overlay_tints_the_sampling_area_or_the_excluded_area() {
    let tint = Color32::from_rgba_unmultiplied(142, 208, 106, 128);
    let h = harness(OperatingSystem::Nix);
    let source = &h.state().source;
    // Cells 0 and 1 are sampled; cell 0 is also the fill area, which is never tinted.
    let image = tint_image(&source.sampling, Some(&source.fill_area), tint, false);
    assert_eq!(image.pixels, [Color32::TRANSPARENT, tint, Color32::TRANSPARENT, Color32::TRANSPARENT]);
    let image = tint_image(&source.sampling, Some(&source.fill_area), tint, true);
    assert_eq!(image.pixels, [Color32::TRANSPARENT, Color32::TRANSPARENT, tint, tint]);

    let mut h = h;
    let shown = pictures(&h);
    h.key_press_modifiers(Modifiers::COMMAND, Key::H);
    h.run_steps(5);
    assert_eq!(pictures(&h) + 1, shown, "the tint is not drawn");
    assert!(h.state().workspace.settings().show_overlay);
    h.key_press_modifiers(Modifiers::COMMAND, Key::H);
    h.run_steps(5);
    assert_eq!(pictures(&h), shown);
}

// The Preview panel keeps its own view, opens framed on the sampling window, and shows the
// progress and the failure text.
#[test]
fn fill_preview_panel_has_its_own_view_framed_on_the_sampling_window() {
    let mut h = harness(OperatingSystem::Nix);
    let w = &h.state().workspace;
    assert!(!w.preview_view.is_fit(), "the view is framed, not fitted");
    let centre = w.preview_view.center();
    assert_eq!((centre.x, centre.y), (150.0, 125.0));
    assert!(w.preview_view.zoom() > w.view.zoom(), "the 100 x 50 window fills a pane the whole 400 x 300 image does not");
    // The first pass asked for a preview: the panel shows it pending.
    assert_eq!(h.state().tickets.len(), 1);
    assert!(h.query_all_by_label("Updating preview").count() > 0);

    let ticket = h.state().tickets[0];
    h.state_mut().workspace.preview_failed(ticket, "nothing to sample from".to_owned());
    h.run_steps(5);
    assert!(h.query_by_label_contains("Preview failed: nothing to sample from").is_some());
    // Zooming the main view leaves the pane's alone.
    let framed = h.state().workspace.preview_view;
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    h.run_steps(5);
    assert_eq!(h.state().workspace.preview_view, framed);
    assert!((h.state().workspace.view.zoom() - 1.0).abs() < 1e-6);
}

// The options bar spans the window above the strip; the footer has Reset, OK, Apply, Cancel in
// the platform's order.
#[test]
fn fill_bar_spans_the_window_and_footer_follows_the_platform_order() {
    for (os, order) in [(OperatingSystem::Nix, ["OK", "Cancel", "Apply"]), (OperatingSystem::Mac, ["Cancel", "Apply", "OK"])] {
        let h = harness(os);
        // The tool's icon is in the bar and its slot in the strip: the bar is the upper of the two
        // and starts over the strip's column.
        let mut icons: Vec<_> = h.get_all_by_label("Sampling Brush Tool").map(|node| bounds(&node)).collect();
        icons.sort_by(|a, b| a.y0.total_cmp(&b.y0));
        let (bar, slot) = (icons[0], icons[icons.len() - 1]);
        assert_eq!(icons.len(), 2, "one in the bar, one in the strip");
        assert!(bar.y1 <= slot.y0 && bar.x0 < slot.x1, "the bar runs over the strip: {bar:?} {slot:?}");
        let size = bounds(&h.get_by_label("Size"));
        assert!(size.x0 > bar.x1 && size.y0 < slot.y0, "the bar's body follows its icon above the strip");

        let x = |label: &str| bounds(&h.get_by_label(label)).x0;
        assert!(x(order[0]) < x(order[1]) && x(order[1]) < x(order[2]), "{os:?}: {order:?}");
        // Reset is the footer's leading button: the lowest of the Reset buttons on the screen.
        let reset = h.get_all_by_label("Reset").map(|node| bounds(&node)).max_by(|a, b| a.y0.total_cmp(&b.y0));
        let Some(reset) = reset else { panic!("a Reset button") };
        // Reset is laid out first and the three buttons share the rest of the row: they don't
        // overlap it.
        assert!(reset.x1 <= x(order[0]).min(x(order[1])).min(x(order[2])), "{os:?}: Reset leads the footer without overlapping a button");
    }
}

/// Presses the primary button at `at` (over the canvas) and lets the pass run.
fn press_canvas(h: &mut Harness<'_, App>, at: egui::Pos2) {
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// The reasons of the stroke cancels the workspace reported.
fn cancels(h: &Harness<'_, App>) -> Vec<CancelReason> {
    h.state()
        .events
        .iter()
        .filter_map(|event| match event {
            FillEvent::Stroke(EditEvent::Cancel { reason, .. }) => Some(*reason),
            _ => None,
        })
        .collect()
}

// A tool letter pressed mid-stroke cancels the stroke with `Removed`, leaves the workspace not
// busy (so the history settles again) and gives the new tool no part of the gesture.
#[test]
fn fill_letter_mid_stroke_cancels_the_stroke_and_ends_busy() {
    let mut h = harness(OperatingSystem::Nix);
    press_canvas(&mut h, pos2(300.0, 400.0));
    assert!(h.state().workspace.canvas.busy(), "the press began a stroke");
    h.key_press(Key::H);
    h.run_steps(3);
    assert_eq!(cancels(&h), [CancelReason::Removed]);
    assert!(!h.state().workspace.canvas.busy());
    assert_eq!(h.state().workspace.tool, FillTool::Hand);
    assert!(h.state().workspace.session().strokes.is_empty(), "a cancelled stroke is not recorded");
}

// ⌘Z mid-stroke cancels the stroke with `UndoDuringGesture` before the history steps.
#[test]
fn fill_undo_mid_stroke_cancels_the_stroke() {
    let mut h = harness(OperatingSystem::Nix);
    press_canvas(&mut h, pos2(300.0, 400.0));
    assert!(h.state().workspace.canvas.busy());
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(cancels(&h), [CancelReason::UndoDuringGesture]);
    assert!(!h.state().workspace.canvas.busy());
    assert!(h.state().workspace.session().strokes.is_empty());
}

// A preview failure shows under the Preview panel and not again under the settings.
#[test]
fn fill_preview_failure_shows_once() {
    let mut h = harness(OperatingSystem::Nix);
    let ticket = h.state().tickets[0];
    h.state_mut().workspace.preview_failed(ticket, "nothing to sample from".to_owned());
    h.run_steps(5);
    assert_eq!(h.query_all_by_label_contains("nothing to sample from").count(), 1);
}

// A color the app's picker answers is reported by the next pass, once; a setting the user changes
// is reported in its pass and asks for a new preview.
#[test]
fn fill_settings_changes_are_reported_by_the_pass_that_sees_them() {
    let mut h = harness(OperatingSystem::Nix);
    let before = h.state().settings_changes;
    h.state_mut().workspace.set_color(OVERLAY_COLOR_KEY, Srgba::new(0.0, 0.0, 1.0, 1.0));
    h.step();
    assert_eq!(h.state().settings_changes, before + 1);
    h.step();
    assert_eq!(h.state().settings_changes, before + 1, "reported once");
    // The same color again changes nothing.
    h.state_mut().workspace.set_color(OVERLAY_COLOR_KEY, Srgba::new(0.0, 0.0, 1.0, 1.0));
    h.step();
    assert_eq!(h.state().settings_changes, before + 1);

    let tickets = h.state().tickets.len();
    h.get_by_label("Sample All Layers").click();
    h.run_steps(3);
    assert!(h.state().workspace.settings().sample_all_layers);
    assert_eq!(h.state().settings_changes, before + 2);
    assert_eq!(h.state().tickets.len(), tickets + 1, "a fill setting asks for a preview");
}

// The Lasso's Expand takes the amount from the bar's field (a number cell captioned "By").
#[test]
fn fill_expand_uses_the_bars_amount() {
    let mut h = harness(OperatingSystem::Nix);
    h.key_press(Key::L);
    h.run_steps(3);
    assert!(h.query_all_by_label("By").count() > 0, "the amount is a captioned field");
    h.get_by_label("Expand").click();
    h.run_steps(3);
    assert!(h.state().events.contains(&FillEvent::FillArea(FillAreaEdit::grow(true, 3.0))));
}
