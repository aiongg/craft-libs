//! `RelinkStyle`.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::{Color32, Vec2};

/// The height of the candidate list in the Link Media dialog.
const CANDIDATES_HEIGHT: f32 = 108.0;
/// The least and greatest width of the Links panel's table, and its share of the window.
const TABLE_MIN: f32 = 90.0;
const TABLE_MAX: f32 = 320.0;
const TABLE_SHARE: f32 = 0.3;
/// The widest the Link Media dialog gets.
const DIALOG_MAX_WIDTH: f32 = 760.0;

/// The Links panel's and Link Media dialog's look.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RelinkStyle {
    /// `size.icon.md`.
    pub badge: f32,
    /// `size.thumb.lg` wide, 16:9.
    pub preview: Vec2,
    /// The candidate list's height.
    pub candidates_height: f32,
    /// The table's least width.
    pub table_min: f32,
    /// The table's greatest width.
    pub table_max: f32,
    /// The table's share of the window.
    pub table_share: f32,
    /// The dialog's greatest width.
    pub dialog_max_width: f32,
    /// Missing, Unreadable, Refused discs (`status.bad.solid`).
    pub bad: Color32,
    /// Modified triangle (`status.warn.solid`).
    pub warn: Color32,
    /// The glyph on a disc (`text.on_status`).
    pub on_status: Color32,
    /// Embedded glyph (`text.muted`).
    pub embedded: Color32,
    /// Same file and matches marks (`status.good.fg`).
    pub good_mark: Color32,
    /// Different file mark (`status.bad.fg`).
    pub bad_mark: Color32,
    /// Not found (`text.muted`).
    pub muted: Color32,
    /// The page number (`text.faint`).
    pub page: Color32,
    /// The preview box (`surface.sunken`).
    pub preview_fill: Color32,
}

impl RelinkStyle {
    /// The style of `tokens`.
    pub fn style(tokens: &UiTokens) -> RelinkStyle {
        let c = &tokens.color;
        let preview_width = tokens.size.thumb.lg;
        RelinkStyle {
            badge: tokens.size.icon.md,
            preview: Vec2::new(preview_width, preview_width * 9.0 / 16.0),
            candidates_height: CANDIDATES_HEIGHT,
            table_min: TABLE_MIN,
            table_max: TABLE_MAX,
            table_share: TABLE_SHARE,
            dialog_max_width: DIALOG_MAX_WIDTH,
            bad: c.status.bad.solid.to_egui(),
            warn: c.status.warn.solid.to_egui(),
            on_status: c.text.on_status.to_egui(),
            embedded: c.text.muted.to_egui(),
            good_mark: c.status.good.fg.to_egui(),
            bad_mark: c.status.bad.fg.to_egui(),
            muted: c.text.muted.to_egui(),
            page: c.text.faint.to_egui(),
            preview_fill: c.surface.sunken.to_egui(),
        }
    }
}
