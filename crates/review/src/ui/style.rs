//! [`ReviewStyle`]: the review widgets' sizes, from the `review` tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The sizes the review widgets read, one `review.*` token each (craft-theme's `review` family,
/// with the source apps' values). Colours with a fixed meaning (status badges, revision kinds, the selected
/// card's accent) are `review.*` colour tokens read by the same task.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReviewStyle {
    /// A comment pin's size: `review.pin`.
    pub pin: f32,
    /// Pins nearer than this gather into a cluster: `review.cluster_radius`.
    pub cluster_radius: f32,
    /// The thread popover's width: `review.popover_width`.
    pub popover_width: f32,
    /// The version history's timeline dot and text columns: `review.version_dot_x`,
    /// `review.version_text_x`.
    pub version_dot_x: f32,
    pub version_text_x: f32,
    /// A cursor chat bubble's width and corner radius: `review.chat_width`,
    /// `review.chat_radius`.
    pub chat_width: f32,
    pub chat_radius: f32,
    /// The join field's width: `review.join_width`.
    pub join_width: f32,
    /// The status badges: `review.status_accepted`, `review.status_rejected`,
    /// `review.status_cancelled`, `review.status_completed`.
    pub status_accepted: Color32,
    pub status_rejected: Color32,
    pub status_cancelled: Color32,
    pub status_completed: Color32,
    /// The selected comment card's accent bar: `review.card_accent`.
    pub card_accent: Color32,
    /// The revision kinds' bars: `review.revision_inserted`, `review.revision_deleted`,
    /// `review.revision_changed`.
    pub revision_inserted: Color32,
    pub revision_deleted: Color32,
    pub revision_changed: Color32,
    /// The revision card's colour bar: `review.revision_bar_width` ×
    /// `review.revision_bar_height`.
    pub revision_bar_width: f32,
    pub revision_bar_height: f32,
}

impl ReviewStyle {
    /// The style from the theme's tokens: `ReviewStyle::of(&Tokens::of(ui))`.
    pub fn of(tokens: &UiTokens) -> ReviewStyle {
        let t = &tokens.component.review;
        ReviewStyle {
            pin: t.pin,
            cluster_radius: t.cluster_radius,
            popover_width: t.popover_width,
            version_dot_x: t.version_dot_x,
            version_text_x: t.version_text_x,
            chat_width: t.chat_width,
            chat_radius: t.chat_radius,
            join_width: t.join_width,
            revision_bar_width: t.revision_bar_width,
            revision_bar_height: t.revision_bar_height,
            status_accepted: t.status_accepted.to_egui(),
            status_rejected: t.status_rejected.to_egui(),
            status_cancelled: t.status_cancelled.to_egui(),
            status_completed: t.status_completed.to_egui(),
            card_accent: t.card_accent.to_egui(),
            revision_inserted: t.revision_inserted.to_egui(),
            revision_deleted: t.revision_deleted.to_egui(),
            revision_changed: t.revision_changed.to_egui(),
        }
    }
}
