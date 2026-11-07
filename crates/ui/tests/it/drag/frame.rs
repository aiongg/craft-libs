//! `DragPreview::Frame`: a frame-sized outline follows the pointer on the ghost layer.

use super::{Seen, drag_over_slot, scene_previewing};
use craft_ui::contract::DragPreview;
use craft_ui::runtime::env::Label;
use craft_ui::testing::Frames;
use craft_ui::testing::trial;
use egui::{Rect, Shape, Vec2};
use std::cell::RefCell;

const FRAME: Vec2 = Vec2::new(120.0, 80.0);

fn outlines(shapes: &[Shape]) -> Vec<Rect> {
    shapes
        .iter()
        .filter_map(|shape| if let Shape::Rect(rect) = shape { Some(rect.rect) } else { None })
        .filter(|rect| (rect.size() - FRAME).abs().max_elem() < 1.0)
        .collect()
}

#[trial]
fn a_frame_preview_paints_a_frame_of_its_size_around_the_pointer_for_the_drag() {
    let seen = RefCell::new(Seen::default());
    let mut show = scene_previewing(&seen, DragPreview::frame(FRAME, Some(Label::Static("Layers"))));
    let mut frames = Frames::new();
    frames.idle(2, &mut show);
    assert!(outlines(&frames.frame_shapes(&mut show)).is_empty(), "no frame before the drag");
    let over = drag_over_slot(&mut frames, &mut show);
    let shapes = frames.frame_shapes(&mut show);
    let found = outlines(&shapes);
    assert!(!found.is_empty(), "the drag paints a frame of the requested size");
    assert!(found.iter().all(|rect| rect.contains(over)), "the frame is centred on the pointer: {found:?} {over:?}");
    assert!(shapes.iter().any(|shape| matches!(shape, Shape::Text(text) if text.galley.job.text == "Layers")), "the frame carries its label");
}
