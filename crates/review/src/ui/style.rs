//! [`ReviewStyle`]: the review widgets' sizes, from the `review` tokens.

use craft_theme::TypeRole;
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
    /// The version text column's x position: `review.version_text_x`.
    pub version_text_x: f32,
    /// A cursor chat bubble's width and corner radius: `review.chat_width`,
    /// `review.chat_radius`.
    pub chat_width: f32,
    /// A cursor chat bubble's corner radius: `review.chat_radius`.
    pub chat_radius: f32,
    /// The join field's width: `review.join_width`.
    pub join_width: f32,
    /// The status badges: `review.status_accepted`, `review.status_rejected`,
    /// `review.status_cancelled`, `review.status_completed`.
    pub status_accepted: Color32,
    #[allow(missing_docs)]
    pub status_rejected: Color32,
    #[allow(missing_docs)]
    pub status_cancelled: Color32,
    #[allow(missing_docs)]
    pub status_completed: Color32,
    /// The selected comment card's accent bar: `review.card_accent`.
    pub card_accent: Color32,
    /// The revision kinds' bars: `review.revision_inserted`, `review.revision_deleted`,
    /// `review.revision_changed`.
    pub revision_inserted: Color32,
    #[allow(missing_docs)]
    pub revision_deleted: Color32,
    #[allow(missing_docs)]
    pub revision_changed: Color32,
    /// The revision card's colour bar: `review.revision_bar_width` ×
    /// `review.revision_bar_height`.
    pub revision_bar_width: f32,
    /// The revision card's colour bar height: `review.revision_bar_height`.
    pub revision_bar_height: f32,
    /// The selected comment card's accent bar width: `review.card_accent_width`.
    pub card_accent_width: f32,
    /// The room between a pin's bubble and the avatar circle inside it: `review.pin_inset`.
    pub pin_inset: f32,
    /// The unread dot's radius on a pin: `review.pin_dot_radius`.
    pub pin_dot_radius: f32,
    /// How far the unread dot sits in from the pin's top right corner: `review.pin_dot_inset`.
    pub pin_dot_inset: f32,
    /// How far the resolved check sits in from the pin's bottom right corner: `review.pin_check_inset`.
    pub pin_check_inset: f32,
    /// The filled and the hollow version dot's radius: `review.version_dot_radius`,
    /// `review.version_dot_hollow_radius`.
    pub version_dot_radius: f32,
    /// The hollow version dot's radius: `review.version_dot_hollow_radius`.
    pub version_dot_hollow_radius: f32,
    /// The light and the dark ink on a person's colour: `review.ink_light`, `review.ink_dark`.
    pub ink_light: Color32,
    /// The dark ink on a person's colour: `review.ink_dark`.
    pub ink_dark: Color32,
    /// The type roles the bodies draw text in: the theme's body, strong body, caption, label and
    /// title roles. A panel's title is `title_text`; a message and a hint are `body_text`; an
    /// author, a heading and a mention are `strong_text`; an age, a count, a detail and
    /// initials are `caption_text`; a form's field name is `label_text`.
    pub body_text: TypeRole,
    /// The theme's strong body role.
    pub strong_text: TypeRole,
    /// The theme's caption role.
    pub caption_text: TypeRole,
    /// The theme's label role.
    pub label_text: TypeRole,
    /// The theme's title role.
    pub title_text: TypeRole,
    /// The type role of a text box's and a callout's text: the theme's body role.
    pub annotation_text: TypeRole,
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
            card_accent_width: t.card_accent_width,
            pin_inset: t.pin_inset,
            pin_dot_radius: t.pin_dot_radius,
            pin_dot_inset: t.pin_dot_inset,
            pin_check_inset: t.pin_check_inset,
            version_dot_radius: t.version_dot_radius,
            version_dot_hollow_radius: t.version_dot_hollow_radius,
            ink_light: t.ink_light.to_egui(),
            ink_dark: t.ink_dark.to_egui(),
            body_text: TypeRole::Body,
            strong_text: TypeRole::BodyStrong,
            caption_text: TypeRole::Caption,
            label_text: TypeRole::Label,
            title_text: TypeRole::Title,
            annotation_text: TypeRole::Body,
        }
    }
}
