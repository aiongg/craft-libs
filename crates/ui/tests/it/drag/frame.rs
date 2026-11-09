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

