//! Style of the `dock` family.

use craft_theme::Tokens;

/// Least width or height of a pane in a split.
const MIN_PANE: f32 = 40.0;
/// Widest an iconic column gets by dragging its inner edge.
const ICONIC_MAX_WIDTH: f32 = 240.0;
/// Widest an iconic column's labels get before they wrap.
const ICONIC_LABEL_WIDTH: f32 = 96.0;
/// Height of a flyout panel.
const FLYOUT_HEIGHT: f32 = 320.0;
/// Greatest height of an overflow menu.
const MENU_MAX_HEIGHT: f32 = 360.0;

/// Sizes and alphas of docking areas that the semantic tokens don't name. [`crate::docking::DockLook`]
/// reads them; applications configure the flat [`crate::docking::DockStyle`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockMetrics {
    /// Width of an iconic column: an icon button (`icon.lg` and its `pad.control` on both sides)
    /// and `gap.tight` between the button and the column's edges.
    pub iconic_width: f32,
    /// Least width or height of a pane in a split.
    pub min_pane: f32,
    /// Least width of the drag bar left of a group's header.
    pub drag_bar_width: f32,
    /// Widest an iconic column gets by dragging its inner edge.
    pub iconic_max_width: f32,
    /// Width of an iconic column's labels.
    pub iconic_label_width: f32,
    /// Height of a flyout panel.
    pub flyout_height: f32,
    /// Width of the overflow button.
    pub overflow_width: f32,
    /// Greatest height of a menu.
    pub menu_max_height: f32,
    /// Opacity of the drop zone fill.
    pub drop_fill: f32,
    /// Opacity of the fill of a dragged group's ghost.
    pub ghost_fill: f32,
    /// Opacity of the outline of a dragged group's ghost.
    pub ghost_stroke: f32,
    /// Opacity of a tab's fill.
    pub tab_fill: f32,
    /// Opacity of a resize grip at rest.
    pub grip_alpha: f32,
}

impl DockMetrics {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let (space, size) = (&tokens.space, &tokens.size);
        Self {
            iconic_width: size.icon.lg + 2.0 * space.pad.control + 2.0 * space.gap.tight,
            min_pane: MIN_PANE,
            drag_bar_width: size.icon.lg + 2.0 * space.pad.panel,
            iconic_max_width: ICONIC_MAX_WIDTH,
            iconic_label_width: ICONIC_LABEL_WIDTH,
            flyout_height: FLYOUT_HEIGHT,
            overflow_width: size.icon.md + 2.0 * space.pad.control,
            menu_max_height: MENU_MAX_HEIGHT,
            drop_fill: 0.15,
            ghost_fill: 0.3,
            ghost_stroke: 0.6,
            tab_fill: 0.08,
            grip_alpha: 0.5,
        }
    }
}
