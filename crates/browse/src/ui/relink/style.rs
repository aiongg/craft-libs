//! `RelinkStyle`.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::{Color32, Vec2};

/// The Links panel's and Link Media dialog's look. Until task TK3 the values come from the
/// tokens they will derive from and the spec's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RelinkStyle {
    /// `link.badge` (16).
    pub badge: f32,
    /// `link.preview` (192 × 108).
    pub preview: Vec2,
    /// `link.candidates_height` (108).
    pub candidates_height: f32,
    /// `link.table_min` (90).
    pub table_min: f32,
    /// `link.table_max` (320).
    pub table_max: f32,
    /// `link.table_share` (0.3 of the window).
    pub table_share: f32,
    /// `link.dialog_max_width` (760).
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
    pub fn of(tokens: &UiTokens) -> RelinkStyle {
        let c = &tokens.color;
        RelinkStyle {
            badge: 16.0,
            preview: Vec2::new(192.0, 108.0),
            candidates_height: 108.0,
            table_min: 90.0,
            table_max: 320.0,
            table_share: 0.3,
            dialog_max_width: 760.0,
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
