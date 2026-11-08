//! The label of a `DragPreview::Frame` starts at the reading start of the frame, inside its padding.

use super::{Seen, drag_over_slot, scene_previewing};
use craft_ui::contract::DragPreview;
use craft_ui::runtime::env::{Direction, Label, RootEnv};
use craft_ui::testing::Frames;
use craft_ui::testing::trial;
use egui::{Rect, Shape, Vec2};
use std::cell::RefCell;

const FRAME: Vec2 = Vec2::new(160.0, 80.0);
const LABEL: &str = "Layers";

/// The frame and its label's rect, painted while the drag is over the slot, in `direction`.
fn frame_and_label(direction: Direction) -> (Rect, Rect) {
    let seen = RefCell::new(Seen::default());
    let mut show = scene_previewing(&seen, DragPreview::frame(FRAME, Some(Label::Static(LABEL))));
    let mut frames = Frames::new();
    frames.root(RootEnv::new().direction(direction));
    drag_over_slot(&mut frames, &mut show);
    let shapes = frames.frame_shapes(&mut show);
    let frame = shapes
        .iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if (rect.rect.size() - FRAME).abs().max_elem() < 1.0 => Some(rect.rect),
            _ => None,
        })
        .expect("the drag paints its frame");
    let label = shapes
        .iter()
        .find_map(|shape| match shape {
            Shape::Text(text) if text.galley.job.text == LABEL => Some(text.galley.rect.translate(text.pos.to_vec2())),
            _ => None,
        })
        .expect("the frame carries its label");
    (frame, label)
}

#[trial]
fn a_frame_previews_label_starts_at_the_reading_start_inside_the_padding() {
    let (frame, label) = frame_and_label(Direction::Ltr);
    assert!(frame.contains_rect(label) && label.left() > frame.left(), "the label sits inside the padded frame: {frame:?} {label:?}");
    assert!(label.left() - frame.left() < frame.right() - label.right(), "left to right, the label starts at the left: {frame:?} {label:?}");
    let (frame, label) = frame_and_label(Direction::Rtl);
    assert!(frame.contains_rect(label) && label.right() < frame.right(), "the label sits inside the padded frame: {frame:?} {label:?}");
    assert!(frame.right() - label.right() < label.left() - frame.left(), "right to left, the label starts at the right: {frame:?} {label:?}");
}
