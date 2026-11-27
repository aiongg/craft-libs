//! The suite's cursors as winit cursors.

use craft_ui_rt::{Cursor, CursorIcon, RgbaImage, WindowId};
use winit::window::{Cursor as WCursor, CursorIcon as W, CustomCursor, CustomCursorSource};

use crate::shared::Shared;

pub(crate) fn icon(icon: CursorIcon) -> W {
    match icon {
        CursorIcon::Default | CursorIcon::None => W::Default,
        CursorIcon::ContextMenu => W::ContextMenu,
        CursorIcon::Help => W::Help,
        CursorIcon::PointingHand => W::Pointer,
        CursorIcon::Progress => W::Progress,
        CursorIcon::Wait => W::Wait,
        CursorIcon::Cell => W::Cell,
        CursorIcon::Crosshair => W::Crosshair,
        CursorIcon::Text => W::Text,
        CursorIcon::VerticalText => W::VerticalText,
        CursorIcon::Alias => W::Alias,
        CursorIcon::Copy => W::Copy,
        CursorIcon::Move => W::Move,
        CursorIcon::NoDrop => W::NoDrop,
        CursorIcon::NotAllowed => W::NotAllowed,
        CursorIcon::Grab => W::Grab,
        CursorIcon::Grabbing => W::Grabbing,
        CursorIcon::AllScroll => W::AllScroll,
        CursorIcon::ResizeHorizontal => W::EwResize,
        CursorIcon::ResizeNeSw => W::NeswResize,
        CursorIcon::ResizeNwSe => W::NwseResize,
        CursorIcon::ResizeVertical => W::NsResize,
        CursorIcon::ResizeColumn => W::ColResize,
        CursorIcon::ResizeEast => W::EResize,
        CursorIcon::ResizeSouthEast => W::SeResize,
        CursorIcon::ResizeSouth => W::SResize,
        CursorIcon::ResizeSouthWest => W::SwResize,
        CursorIcon::ResizeWest => W::WResize,
        CursorIcon::ResizeNorthWest => W::NwResize,
        CursorIcon::ResizeNorth => W::NResize,
        CursorIcon::ResizeNorthEast => W::NeResize,
        CursorIcon::ZoomIn => W::ZoomIn,
        CursorIcon::ZoomOut => W::ZoomOut,
        _ => W::Default,
    }
}

pub(crate) fn source(image: &RgbaImage, hotspot: (u32, u32)) -> Option<CustomCursorSource> {
    let small = |v: u32| u16::try_from(v).ok();
    CustomCursor::from_rgba(image.pixels.clone(), small(image.size.width)?, small(image.size.height)?, small(hotspot.0)?, small(hotspot.1)?)
        .inspect_err(|e| log::warn!("custom cursor rejected: {e}"))
        .ok()
}

/// Applies `cursor` to the window now, or queues the custom image for the loop to create.
pub(crate) fn apply(shared: &mut Shared, id: WindowId, cursor: Cursor) {
    let Some(window) = shared.native(id) else { return };
    shared.cursor_wanted.retain(|(w, ..)| *w != id);
    window.set_cursor_visible(!matches!(cursor, Cursor::Hidden | Cursor::Icon(CursorIcon::None)));
    match cursor {
        Cursor::Default | Cursor::Hidden => window.set_cursor(W::Default),
        Cursor::Icon(i) => window.set_cursor(icon(i)),
        Cursor::Image { key, hotspot } => match shared.cursor_cache.get(&(key, hotspot)) {
            Some(custom) => window.set_cursor(WCursor::Custom(custom.clone())),
            None if shared.cursor_images.contains_key(&key) => shared.cursor_wanted.push((id, key, hotspot)),
            None => window.set_cursor(W::Default),
        },
    }
}
