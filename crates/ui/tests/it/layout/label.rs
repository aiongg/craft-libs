//! `LabelColumn`: the label follows the control's baseline of last pass.

use craft_behavior::{Flow, LabelColumn};
use craft_ui::layout::{place_label, record_control_baseline};
use craft_ui::runtime::env::Env;
use craft_ui::testing::layout::entry_id;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Vec2};

#[trial]
fn the_label_aligns_to_the_control_baseline_of_last_frame() {
    let mut frames = Frames::new();
    let column = LabelColumn::new(80.0, 120.0, 4.0);
    let mut tops = Vec::new();
    let mut frame = |frames: &mut Frames| {
        tops.clear();
        let passes = frames.frame(vec![], |ui| {
            let rtl = Env::of(ui).direction.is_rtl();
            let min = ui.available_rect_before_wrap().min;
            let row = Rect::from_min_size(min, Vec2::new(300.0, 30.0));
            let id = entry_id(ui, "row");
            let placed = place_label(ui, id, &column, Flow::new(row, rtl), Vec2::new(40.0, 12.0), 9.0);
            tops.push(placed.label.top() - min.y);
            record_control_baseline(ui, id, 14.0);
        });
        (passes, tops.clone())
    };
    let (passes, first) = frame(&mut frames);
    assert_eq!(passes, 2, "the first appearance runs a discard pass that uses the stored baseline");
    assert!(first[1] > first[0], "the stored baseline moves the label: {first:?}");
    assert_eq!(frame(&mut frames), (1, vec![first[1]]), "the next frame places from last frame's baseline in one pass");
}
