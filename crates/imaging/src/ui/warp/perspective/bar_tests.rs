//! Tests of the perspective bar.

use egui::Modifiers;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::*;
use craft_ui::edit::EditEvent;
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct Host {
    session: PerspectiveSession,
    out: Vec<PerspectiveEdit>,
    apply: bool,
    discard: bool,
}

fn plane(x: f64) -> Plane {
    Plane::identity([Point::new(x, 0.0), Point::new(x + 10.0, 0.0), Point::new(x + 10.0, 10.0), Point::new(x, 10.0)])
}

fn harness(count: usize, mode: PlaneMode) -> Harness<'static, Host> {
    let mut session = PerspectiveSession::new(Rect::new(0.0, 0.0, 100.0, 100.0));
    if count > 0 {
        let planes = (0..count).map(|i| plane(20.0 * i as f64)).collect();
        session.apply(PerspectiveEdit::Planes(EditEvent::set(planes, Modifiers::NONE)), 0.0);
    }
    session.apply(PerspectiveEdit::Mode(mode), 0.0);
    let mut h = Harness::builder().with_size([1200.0, 80.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let mut r = PerspectiveBarOutput::default();
            OptionsBar::new("options").label("Options").show(ui, |slots| PerspectiveBar::new("bar", &host.session).output(&mut r).add_to(slots));
            host.out.extend(r.edits);
            host.apply |= r.apply;
            host.discard |= r.discard;
        },
        Host { session, out: Vec::new(), apply: false, discard: false },
    );
    h.run_steps(3);
    h
}

// The count reads "1 plane" and "2 planes" through the count word.
#[test]
fn the_count_reads_plane_and_planes() {
    assert!(harness(1, PlaneMode::Layout).query_by_label("1 plane").is_some());
    assert!(harness(2, PlaneMode::Layout).query_by_label("2 planes").is_some());
    assert!(harness(0, PlaneMode::Layout).query_by_label("0 planes").is_some());
}

// Layout shows the count and Remove Planes; Warp shows Straighten, Level and Both; the Warp tab is
// disabled with no plane.
#[test]
fn the_bar_shows_each_modes_controls() {
    let layout = harness(2, PlaneMode::Layout);
    assert!(layout.query_by_label("Remove Planes").is_some());
    assert!(layout.query_by_label("Straighten").is_none());
    assert!(layout.query_by_role_and_label(Role::RadioGroup, "Mode").is_some());

    let mut warp = harness(2, PlaneMode::Warp);
    for name in ["Straighten", "Level", "Both"] {
        assert!(warp.query_by_label(name).is_some(), "{name}");
    }
    assert!(warp.query_by_label("Remove Planes").is_none());
    warp.get_by_label("Level").click();
    warp.run_steps(3);
    assert_eq!(warp.state().out, vec![PerspectiveEdit::Straighten(Straighten::Horizontal)]);

    let empty = harness(0, PlaneMode::Layout);
    assert!(empty.get_by_role_and_label(Role::RadioButton, "Warp").accesskit_node().is_disabled());
}

// Commit and Cancel sit in the trailing slot and report `apply` and `discard`.
#[test]
fn commit_and_cancel_report_through_the_output() {
    let mut h = harness(1, PlaneMode::Layout);
    h.get_by_label("Commit Perspective Warp").click();
    h.run_steps(3);
    assert!(h.state().apply && !h.state().discard);
    h.get_by_label("Cancel Perspective Warp").click();
    h.run_steps(3);
    assert!(h.state().discard);
}
