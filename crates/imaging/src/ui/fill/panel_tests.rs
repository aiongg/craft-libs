//! Tests of Content-Aware Fill's panels and workspace (task WS-CA2).

use egui::os::OperatingSystem;
use egui::{Color32, ColorImage, Key, Modifiers, Shape, TextureId, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::overlay::tint_image;
use super::{ContentAwareFillWorkspace, FillEvent, FillLimits, FillOpen, FillOutput, FillSource, SamplingPlane};
use craft_ui::canvas::kurbo::Rect as DocRect;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::Outcome;
use craft_ui::dialog::full_window::PreviewTicket;
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
    };
    let mut h =
        HarnessBuilder::default().with_size(WINDOW).with_os(os).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let out = app.workspace.show(&ui.ctx().clone(), &app.source);
                app.outcomes.extend(out.outcome);
                app.events.extend(out.events.iter().cloned());
                app.tickets.extend(out.preview_wanted);
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
        // At Standard width the three buttons fill the row, so Reset starts where the first one
        // does (the kernel's footer leaves it no room of its own).
        assert!(reset.x0 <= x(order[0]).min(x(order[1])).min(x(order[2])), "{os:?}: Reset leads the footer");
    }
}
