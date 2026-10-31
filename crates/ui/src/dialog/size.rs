//! Dialog widths, resizable sizes and scrolling bodies: the rules, as pure functions.

use craft_theme::DialogTokens;
use egui::Vec2;

/// How wide a dialog's content may be.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Width {
    /// A few short fields (Fit Image): at most `dialog.width_compact`, no least width.
    Compact,
    /// At least `dialog.width_min`, at most `dialog.width_standard`.
    #[default]
    Standard,
    /// At least `dialog.width_min`, at most `dialog.width_wide`.
    Wide,
    /// Exactly this wide (New Document 800, About 700), within the window.
    Fixed(f32),
    /// At least `min`, at most `max` (Shortcuts 460 to 720), within the window.
    Range { min: f32, max: f32 },
}

impl Width {
    /// The least and greatest content width in a window `window_width` wide, in points. Every
    /// width is capped at the window minus `dialog.window_margin` on each side; the least is at
    /// most the greatest.
    pub fn range(self, tokens: &DialogTokens, window_width: f32) -> (f32, f32) {
        let cap = (window_width - 2.0 * tokens.window_margin).max(0.0);
        let (min, max) = match self {
            Width::Compact => (0.0, tokens.width_compact),
            Width::Standard => (tokens.width_min, tokens.width_standard),
            Width::Wide => (tokens.width_min, tokens.width_wide),
            Width::Fixed(width) => (width, width),
            Width::Range { min, max } => (min, max.max(min)),
        };
        let max = max.min(cap);
        (min.min(cap).min(max), max)
    }
}

/// The greatest size of a resizable dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ResizeMax {
    /// The window less `dialog.resize_margin` on each side, never below the least size.
    #[default]
    Window,
    /// This size, within the window rule above.
    Fixed(Vec2),
}

/// A resizable dialog's sizes, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resize {
    /// The size it opens at (before a remembered size).
    pub default: Vec2,
    pub min: Vec2,
    pub max: ResizeMax,
}

impl Resize {
    /// The greatest size in a window of `window` size.
    pub fn max_size(&self, tokens: &DialogTokens, window: Vec2) -> Vec2 {
        let room = (window - Vec2::splat(2.0 * tokens.resize_margin)).max(self.min);
        match self.max {
            ResizeMax::Window => room,
            ResizeMax::Fixed(max) => max.min(room).max(self.min),
        }
    }
}

/// The size to remember after a resize, if any: only a size that differs from the size the dialog
/// opened at and is below `max` on both axes. A size at the window's limit is the window's, not
/// the user's, and a larger one remembered from a bigger window must survive a small one.
pub fn remembered_size(size: Vec2, opened: Vec2, max: Vec2) -> Option<Vec2> {
    (size != opened.min(max) && size.x < max.x - 0.5 && size.y < max.y - 0.5).then_some(size)
}

/// How a dialog's body fits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Body {
    /// The body takes the height it needs, at most [`scroll_height`]; a taller body scrolls
    /// vertically under the title and above the footer, so the dialog never outgrows the window.
    #[default]
    Fit,
    /// As `Fit`, with the scroll area filling the content width. A region drawn with [`Dialog::show_with_below`](super::Dialog::show_with_below)
    /// stays under it, and the area leaves room for that region's measured height.
    Scroll,
}

/// The greatest height of a scrolling body: the window height less the dialog's margins, the
/// measured `chrome` (everything but the body) and the measured `below` region, and at least `dialog.scroll_min`.
pub fn scroll_height(tokens: &DialogTokens, window_height: f32, chrome: f32, below: f32) -> f32 {
    (window_height - 2.0 * tokens.window_margin - chrome - below).max(tokens.scroll_min)
}
