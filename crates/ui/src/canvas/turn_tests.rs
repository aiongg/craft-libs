use egui::{Context, CursorIcon, Pos2, Shape, pos2};
use egui_kittest::Harness;

use super::{SelectionBadge, selection_cursor, turn_cursor, turn_cursor_with};
use crate::theme::{Base, install};

/// Runs one frame of `add` and returns the cursor icon it left and the shapes it painted.
fn pass(add: impl Fn(&Context)) -> (CursorIcon, usize) {
    let h = Harness::builder().with_size([200.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|ui| add(ui.ctx()));
    let out = h.output();
    (out.platform_output.cursor_icon, out.shapes.iter().map(|s| count(&s.shape)).sum())
}

fn count(shape: &Shape) -> usize {
    match shape {
        Shape::Vec(v) => v.iter().map(count).sum(),
        _ => 1,
    }
}

const AT: Pos2 = pos2(100.0, 100.0);

#[test]
fn native_false_paints_the_cursor_and_true_sets_the_icon() {
    let base = pass(|_| {}).1;
    let native = pass(|ctx| selection_cursor(ctx, AT, SelectionBadge::Add, true));
    let painted = pass(|ctx| selection_cursor(ctx, AT, SelectionBadge::Add, false));
    assert_eq!(native.0, CursorIcon::Crosshair);
    assert_eq!(painted.0, CursorIcon::None);
    // The painted crosshair is on top of the badge both draw.
    assert!(painted.1 > native.1 && native.1 > base, "{base} {native:?} {painted:?}");
    // New has no badge: the native cursor is the bare icon.
    assert_eq!(pass(|ctx| selection_cursor(ctx, AT, SelectionBadge::New, true)), (CursorIcon::Crosshair, base));

    // The rotate pointer hides the system one unless the host keeps it.
    let toward = egui::vec2(1.0, 1.0);
    let old = pass(|ctx| turn_cursor(ctx, AT, toward));
    assert!(old.1 > base);
    assert_eq!(old, pass(|ctx| turn_cursor_with(ctx, AT, toward, true)));
    assert_eq!(old.0, CursorIcon::None);
    let kept = pass(|ctx| turn_cursor_with(ctx, AT, toward, false));
    assert_eq!((kept.0, kept.1), (CursorIcon::Default, old.1));
}
