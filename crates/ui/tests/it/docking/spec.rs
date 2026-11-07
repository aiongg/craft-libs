//! `PanelLimits`, `DockHeights` and the defaults of `DockPanels`.

use craft_behavior::{DockTree, GroupId};
use craft_ui::contract::{Bind, BodyCx, ShowContainer as _};
use craft_ui::docking::{DockHeights, DockPanels, PanelLimits, PanelSpec, SplitArea};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};

#[trial]
fn limits_and_heights_refuse_values_that_are_not_finite_or_out_of_range() {
    assert!(PanelLimits::default().valid());
    assert!(PanelLimits::new(Vec2::new(100.0, 50.0), Vec2::new(400.0, 300.0)).valid());
    for bad in [
        PanelLimits::new(Vec2::new(f32::NAN, 0.0), Vec2::splat(10.0)),
        PanelLimits::new(Vec2::ZERO, Vec2::new(f32::INFINITY, 10.0)),
        PanelLimits::new(Vec2::new(-1.0, 0.0), Vec2::splat(10.0)),
        PanelLimits::new(Vec2::splat(20.0), Vec2::splat(10.0)),
        PanelLimits::at_most(Vec2::splat(PanelLimits::BOUND + 1.0)),
    ] {
        assert!(!bad.valid(), "{bad:?}");
    }
    assert!(DockHeights::new(200.0).compact(120.0).fill(400.0).valid());
    for bad in [DockHeights::new(f32::NAN), DockHeights::new(-1.0), DockHeights::new(10.0).fill(PanelLimits::BOUND + 1.0)] {
        assert!(!bad.valid(), "{bad:?}");
    }
}

struct Plain;

impl DockPanels<u8> for Plain {
    fn spec(&self, panel: &u8) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn body(&mut self, _: &u8, _: &mut Ui, _: &mut BodyCx<'_>) {}
}

#[trial]
fn a_dock_panels_implementation_with_defaults_leaves_groups_to_the_dock() {
    let mut panels = Plain;
    let mut tree = DockTree::group(GroupId::new(0));
    let mut frames = Frames::new();
    let mut shown = None;
    frames.idle(1, |ui| {
        SplitArea::new("Panels", Bind::bound(&mut tree)).show(ui, |ui, body| {
            let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(50.0));
            shown = Some(panels.group_view(GroupId::new(0), ui, rect, body));
            assert!(panels.panel_rows(&0).is_empty());
        });
    });
    assert_eq!(shown, Some(false), "the default group_view shows nothing and returns false");
    assert_eq!(panels.spec(&3).label.as_str(), "Panel 3");
}
