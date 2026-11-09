//! Where a beside label sits in the label column.

use super::group::Group;
use super::kit::{Dial, Seen};
use craft_i18n::Direction;
use craft_ui::contract::{LabelAlign, Props as _, ShowContainer as _, SideLabel};
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(410.0, 80.0) };
const SAME: f32 = 0.5;

/// The gap between the short label and its control, and the label's distance from the row's start edge.
fn placed(align: LabelAlign, rtl: bool) -> (f32, f32) {
    let mut frames = Frames::new();
    let mut root = RootEnv::new();
    root.direction = if rtl { Direction::Rtl } else { Direction::Ltr };
    frames.root(root);
    let (short, long) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let (mut a, mut b) = (1.0, 1.0);
    frames.idle(3, |ui| {
        Group::column().id_salt("align").show_at(ui, ROW, |ui, cx| {
            let side = SideLabel::new().align(align);
            cx.labelled(ui, "short", "Size".into(), side.clone(), Dial::new(&mut a, &short));
            cx.labelled(ui, "long", "Hue and saturation".into(), side, Dial::new(&mut b, &long));
        });
    });
    let (label, control) = (short.borrow().label.clone().map(|label| label.rect).unwrap_or(Rect::NOTHING), short.borrow().rect);
    let gap = if rtl { label.min.x - control.max.x } else { control.min.x - label.max.x };
    let from_start = if rtl { ROW.max.x - label.max.x } else { label.min.x - ROW.min.x };
    (gap, from_start)
}

#[trial]
fn beside_labels_end_against_the_control_by_default_and_start_labels_start_at_the_row() {
    for rtl in [false, true] {
        let (end_gap, end_from_start) = placed(LabelAlign::End, rtl);
        let (start_gap, start_from_start) = placed(LabelAlign::Start, rtl);
        assert!(start_from_start.abs() < SAME, "a start label begins at the row's start (rtl {rtl})");
        assert!(end_from_start > SAME, "an end label is moved toward the control (rtl {rtl})");
        assert!(end_gap < start_gap - SAME, "an end label sits nearer the control than a start label (rtl {rtl})");
    }
}

/// A content-sized group is as wide as its rows at the widest label: a wide control under a short label, measured
/// before the long label of a later row, stays inside the group.
#[trial]
fn a_content_sized_group_holds_a_wide_control_under_a_short_label() {
    use craft_ui::buttons::Button;
    use craft_ui::contract::ContainerProps;
    let mut frames = Frames::new();
    let wide = RefCell::new(Seen::default());
    let mut value = 1.0;
    let mut group = Rect::NOTHING;
    frames.idle(3, |ui| {
        group = Group::new(ContainerProps::new().label_column(true))
            .id_salt("held")
            .show(ui, |ui, cx| {
                cx.labelled(ui, "short", "A".into(), SideLabel::new(), Dial::new(&mut value, &wide));
                cx.labelled(ui, "long", "Hue and saturation".into(), SideLabel::new(), Button::new("OK"));
            })
            .rect;
    });
    let control = wide.borrow().rect;
    let label = wide.borrow().label.clone().map_or(Rect::NOTHING, |label| label.rect);
    assert!(control.min.x > label.max.x, "the control sits beside its label: {label:?} {control:?}");
    assert!(group.contains_rect(control), "the group holds the control: {group:?} {control:?}");
}
