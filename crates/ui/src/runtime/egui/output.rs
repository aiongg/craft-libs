//! The cursor icon and IME output in `PlatformOutput`.

use craft_behavior::CursorIcon;
use egui::Context;

/// Sets the system cursor of the running pass.
pub(crate) fn set_cursor(ctx: &Context, icon: CursorIcon) {
    ctx.set_cursor_icon(egui_icon(icon));
}

fn egui_icon(icon: CursorIcon) -> egui::CursorIcon {
    use egui::CursorIcon as E;
    match icon {
        CursorIcon::Default => E::Default,
        CursorIcon::None => E::None,
        CursorIcon::ContextMenu => E::ContextMenu,
        CursorIcon::Help => E::Help,
        CursorIcon::PointingHand => E::PointingHand,
        CursorIcon::Progress => E::Progress,
        CursorIcon::Wait => E::Wait,
        CursorIcon::Cell => E::Cell,
        CursorIcon::Crosshair => E::Crosshair,
        CursorIcon::Text => E::Text,
        CursorIcon::VerticalText => E::VerticalText,
        CursorIcon::Alias => E::Alias,
        CursorIcon::Copy => E::Copy,
        CursorIcon::Move => E::Move,
        CursorIcon::NoDrop => E::NoDrop,
        CursorIcon::NotAllowed => E::NotAllowed,
        CursorIcon::Grab => E::Grab,
        CursorIcon::Grabbing => E::Grabbing,
        CursorIcon::AllScroll => E::AllScroll,
        CursorIcon::ResizeHorizontal => E::ResizeHorizontal,
        CursorIcon::ResizeNeSw => E::ResizeNeSw,
        CursorIcon::ResizeNwSe => E::ResizeNwSe,
        CursorIcon::ResizeVertical => E::ResizeVertical,
        CursorIcon::ResizeEast => E::ResizeEast,
        CursorIcon::ResizeSouthEast => E::ResizeSouthEast,
        CursorIcon::ResizeSouth => E::ResizeSouth,
        CursorIcon::ResizeSouthWest => E::ResizeSouthWest,
        CursorIcon::ResizeWest => E::ResizeWest,
        CursorIcon::ResizeNorthWest => E::ResizeNorthWest,
        CursorIcon::ResizeNorth => E::ResizeNorth,
        CursorIcon::ResizeNorthEast => E::ResizeNorthEast,
        CursorIcon::ZoomIn => E::ZoomIn,
        CursorIcon::ZoomOut => E::ZoomOut,
        _ => E::Default,
    }
}
