use egui::{Context, Id, Vec2};

/// How big a [`Popover`](super::Popover)'s content area is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PopoverSize {
    /// As large as the contents; the host limits the width with `ui.set_max_width`.
    Fit,
    /// Exactly this size.
    Fixed(Vec2),
    /// The remembered size, else `default`, never below `min`, with a [`ResizeGrip`](super::ResizeGrip)
    /// at the bottom trailing corner. The contents get `ui.set_width(size.x)` and
    /// `ui.set_min_height(size.y)` and fill them.
    Resizable { default: Vec2, min: Vec2 },
}

/// Where the remembered content size of the popover `id` is stored in egui's persisted memory.
pub(super) fn key(id: Id) -> Id {
    id.with("content-size")
}

/// The remembered size of the popover `id`, or `default` when there is none or it is not finite;
/// kept between `min` and `max` (`min` wins when the window is too small for it).
pub(super) fn read(ctx: &Context, id: Id, default: Vec2, min: Vec2, max: Vec2) -> Vec2 {
    let stored = ctx.data_mut(|d| d.get_persisted::<[f32; 2]>(key(id)));
    let size = stored.map(|[x, y]| Vec2::new(x, y)).filter(|v| v.x.is_finite() && v.y.is_finite()).unwrap_or(default);
    clamp(size, min, max)
}

/// Remembers `size` as the content size of the popover `id`.
pub(super) fn write(ctx: &Context, id: Id, size: Vec2) {
    ctx.data_mut(|d| d.insert_persisted(key(id), [size.x, size.y]));
}

pub(super) fn clamp(size: Vec2, min: Vec2, max: Vec2) -> Vec2 {
    let max = max.max(min);
    size.max(min).min(max)
}
