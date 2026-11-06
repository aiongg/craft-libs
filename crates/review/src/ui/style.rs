//! [`ReviewStyle`]: the review widgets' sizes, from the semantic tokens.

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::{Color32, Painter};

const CLUSTER_RADIUS: f32 = 24.0;
const PIN_INSET: f32 = 3.0;
const PIN_DOT_INSET: f32 = 2.0;
const PIN_CHECK_INSET: f32 = 3.0;
/// A comment typical of a thread: the thread popover is as wide as this on one line.
const THREAD_SAMPLE: &str = "Please align this with the headline above.";
/// A line a collaborator types in a cursor chat bubble.
const CHAT_SAMPLE: &str = "Can you move this a little to the left?";
/// An invite link, the longest value the join field takes.
const JOIN_SAMPLE: &str = "https://example.com/join/abc123";
const INK_DARK: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);

/// The sizes and colours the review widgets read, from the semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReviewStyle {
    /// A comment pin's size.
    pub pin: f32,
    /// Pins nearer than this gather into a cluster.
    pub cluster_radius: f32,
    /// The version history's timeline dot column.
    pub version_dot_x: f32,
    /// The version text column's x position.
    pub version_text_x: f32,
    /// The revision card's colour bar width.
    pub revision_bar_width: f32,
    /// The revision card's colour bar height.
    pub revision_bar_height: f32,
    /// The room between a pin's bubble and the avatar circle inside it.
    pub pin_inset: f32,
    /// How far the unread dot sits in from the pin's top right corner.
    pub pin_dot_inset: f32,
    /// How far the resolved check sits in from the pin's bottom right corner.
    pub pin_check_inset: f32,
    /// The light ink on a person's colour.
    pub ink_light: Color32,
    /// The dark ink on a person's colour.
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
    /// The thread popover's width: a typical comment on one line.
    pub fn popover_width(&self, painter: &Painter, tokens: &UiTokens) -> f32 {
        self.sample_width(painter, tokens, THREAD_SAMPLE)
    }

    /// A cursor chat bubble's width: a typical chat line.
    pub fn chat_width(&self, painter: &Painter, tokens: &UiTokens) -> f32 {
        self.sample_width(painter, tokens, CHAT_SAMPLE)
    }

    /// The join field's width: an invite link.
    pub fn join_width(&self, painter: &Painter, tokens: &UiTokens) -> f32 {
        self.sample_width(painter, tokens, JOIN_SAMPLE)
    }

    /// The width of `sample` in the body role plus the horizontal control padding on both sides.
    fn sample_width(&self, painter: &Painter, tokens: &UiTokens, sample: &str) -> f32 {
        painter.layout_no_wrap(sample.to_owned(), tokens.font(self.body_text), Color32::PLACEHOLDER).size().x + 2.0 * tokens.space.pad.control
    }

    /// The style from the theme's tokens: `ReviewStyle::of(&Tokens::of(ui))`.
    pub fn of(tokens: &UiTokens) -> ReviewStyle {
        let color = &tokens.color;
        let control = tokens.size.control.md;
        ReviewStyle {
            pin: control,
            cluster_radius: CLUSTER_RADIUS,
            version_dot_x: tokens.size.icon.md,
            version_text_x: tokens.size.icon.md + 2.0 * tokens.space.pad.control,
            revision_bar_width: tokens.space.gap.tight,
            revision_bar_height: tokens.size.icon.sm,
            pin_inset: PIN_INSET,
            pin_dot_inset: PIN_DOT_INSET,
            pin_check_inset: PIN_CHECK_INSET,
            ink_light: color.text.on_strong.to_egui(),
            ink_dark: INK_DARK,
            body_text: TypeRole::Body,
            strong_text: TypeRole::BodyStrong,
            caption_text: TypeRole::Caption,
            label_text: TypeRole::Label,
            title_text: TypeRole::Title,
            annotation_text: TypeRole::Body,
        }
    }
}
