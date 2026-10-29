//! Where a dialog sits: keeping it on screen, and the keys its position and size are remembered under.

use egui::{Context, Id, Vec2};

/// The offset from the window's top-left that keeps a dialog of `size` on screen in a window of
/// `content` size. A dialog that fits keeps its whole rect inside; one wider than the window keeps
/// `keep_visible` of its width inside; one taller than the window keeps its top edge and the
/// title row (`title_height`) inside. Each axis is decided on its own.
pub fn keep_on_screen(offset: Vec2, size: Vec2, title_height: f32, content: Vec2, keep_visible: f32) -> Vec2 {
    let across = |offset: f32, size: f32, content: f32| {
        if size <= content {
            offset.clamp(0.0, content - size)
        } else {
            let keep = keep_visible.min(size).min(content);
            let low = keep - size;
            offset.clamp(low, (content - keep).max(low))
        }
    };
    let down = |offset: f32, size: f32, content: f32| {
        if size <= content { offset.clamp(0.0, content - size) } else { offset.clamp(0.0, (content - title_height).max(0.0)) }
    };
    Vec2::new(across(offset.x, size.x, content.x), down(offset.y, size.y, content.y))
}

/// The offset to pin the dialog at after a laid-out pass, if it should be pinned or moved now:
/// the first laid-out pass pins where the dialog is, and a drag moves the pin. Otherwise the pin
/// stays, so a body that grows extends down and right from its top-left.
pub fn next_pin(pinned: Option<Vec2>, laid_out: Vec2, drag: Vec2, size: Vec2, title_height: f32, content: Vec2, keep_visible: f32) -> Option<Vec2> {
    (pinned.is_none() || drag != Vec2::ZERO).then(|| keep_on_screen(pinned.unwrap_or(laid_out) + drag, size, title_height, content, keep_visible))
}

/// What a dialog's remembered place and size are stored under: the host's choice, per command for
/// command dialogs and per kind otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaceKey(pub Id);

impl PlaceKey {
    /// A key from any hashable value.
    pub fn new(key: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self(Id::unique(("craft_ui::dialog::place", key)))
    }

    fn place(self) -> Id {
        self.0.with("place")
    }

    fn size(self) -> Id {
        self.0.with("size")
    }

    /// The offset a dragged dialog was left at.
    pub fn remembered_place(self, ctx: &Context) -> Option<Vec2> {
        ctx.data_mut(|d| d.get_persisted::<Vec2>(self.place()))
    }

    pub(super) fn remember_place(self, ctx: &Context, offset: Vec2) {
        ctx.data_mut(|d| d.insert_persisted(self.place(), offset));
    }

    /// The size a resized dialog was left at.
    pub fn remembered_size(self, ctx: &Context) -> Option<Vec2> {
        ctx.data_mut(|d| d.get_persisted::<Vec2>(self.size()))
    }

    pub(super) fn remember_size(self, ctx: &Context, size: Vec2) {
        ctx.data_mut(|d| d.insert_persisted(self.size(), size));
    }
}
