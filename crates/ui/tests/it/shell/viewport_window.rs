//! `ViewportWindow`: the close contract it shares with `DocumentWindow`.

use crate::windows_kit::Windows;
use craft_ui::contract::Show as _;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::shell::ViewportWindow;
use craft_ui::testing::layout::Block;
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2, ViewportEvent, ViewportInfo};
use std::cell::{Cell, RefCell};

const BODY: Vec2 = Vec2::new(120.0, 80.0);
const WINDOW: Vec2 = Vec2::new(400.0, 300.0);
const SETTLED_FRAMES: usize = 3;

fn reporting() -> ViewportInfo {
    ViewportInfo {
        focused: Some(true),
        inner_rect: Some(Rect::from_min_size(Pos2::new(100.0, 100.0), WINDOW)),
        native_pixels_per_point: Some(1.0),
        ..ViewportInfo::default()
    }
}

