//! Review: the sizes of comment pins, thread popovers, version history, cursor chat and revision
//! bars, and the colours of its status badges, revision kinds and the inks on person colours.
//! The colours are overridable.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Review tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReviewTokens {
    /// A comment pin's size: `values::REVIEW_PIN`.
    pub pin: f32,
    /// Pins nearer than this gather into a cluster: `values::REVIEW_CLUSTER_RADIUS`.
    pub cluster_radius: f32,
    /// The thread popover's width: `values::REVIEW_POPOVER_WIDTH`.
    pub popover_width: f32,
    /// The version history's timeline dot column: `values::REVIEW_VERSION_DOT_X`.
    pub version_dot_x: f32,
    /// The version history's text column: `values::REVIEW_VERSION_TEXT_X`.
    pub version_text_x: f32,
    /// A cursor chat bubble's width: `values::REVIEW_CHAT_WIDTH`.
    pub chat_width: f32,
    /// A cursor chat bubble's corner radius: `values::REVIEW_CHAT_RADIUS`.
    pub chat_radius: f32,
    /// The join field's width: `values::REVIEW_JOIN_WIDTH`.
    pub join_width: f32,
    /// The revision card's colour bar width: `values::REVIEW_REVISION_BAR_WIDTH`.
    pub revision_bar_width: f32,
    /// The revision card's colour bar height: `values::REVIEW_REVISION_BAR_HEIGHT`.
    pub revision_bar_height: f32,
    /// The selected comment card's accent bar width: `values::REVIEW_CARD_ACCENT_WIDTH`.
    pub card_accent_width: f32,
    /// The room between a pin's bubble and the avatar circle inside it: `values::REVIEW_PIN_INSET`.
    pub pin_inset: f32,
    /// The filled version dot's radius: `values::REVIEW_VERSION_DOT_RADIUS`.
    pub version_dot_radius: f32,
    /// The hollow version dot's radius: `values::REVIEW_VERSION_DOT_HOLLOW_RADIUS`.
    pub version_dot_hollow_radius: f32,
    /// The accepted status badge: `review.status_accepted`, `status.good.fg`.
    pub status_accepted: Rgba,
    /// The rejected status badge: `review.status_rejected`, `status.bad.fg`.
    pub status_rejected: Rgba,
    /// The cancelled status badge: `review.status_cancelled`, `text.faint`.
    pub status_cancelled: Rgba,
    /// The completed status badge: `review.status_completed`, `status.good.fg`.
    pub status_completed: Rgba,
    /// The selected comment card's accent bar: `review.card_accent`, `accent.solid`.
    pub card_accent: Rgba,
    /// The inserted-revision bar: `review.revision_inserted`, `status.good.fg`.
    pub revision_inserted: Rgba,
    /// The deleted-revision bar: `review.revision_deleted`, `status.bad.fg`.
    pub revision_deleted: Rgba,
    /// The changed-revision bar: `review.revision_changed`, `status.info.fg`.
    pub revision_changed: Rgba,
    /// The light ink on a dark person colour: `review.ink_light`, `text.on_strong`.
    pub ink_light: Rgba,
    /// The dark ink on a light person colour: `review.ink_dark`, `values::REVIEW_INK_DARK`.
    pub ink_dark: Rgba,
}

/// An overridable review field. Paths: `status_accepted`, `status_rejected`, `status_cancelled`,
/// `status_completed`, `card_accent`, `revision_inserted`, `revision_deleted`, `revision_changed`,
/// `ink_light`, `ink_dark`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReviewField {
    /// `review.status_accepted`
    StatusAccepted,
    /// `review.status_rejected`
    StatusRejected,
    /// `review.status_cancelled`
    StatusCancelled,
    /// `review.status_completed`
    StatusCompleted,
    /// `review.card_accent`
    CardAccent,
    /// `review.revision_inserted`
    RevisionInserted,
    /// `review.revision_deleted`
    RevisionDeleted,
    /// `review.revision_changed`
    RevisionChanged,
    /// `review.ink_light`
    InkLight,
    /// `review.ink_dark`
    InkDark,
}

impl ReviewField {
    const ALL: [ReviewField; 10] = [
        ReviewField::StatusAccepted,
        ReviewField::StatusRejected,
        ReviewField::StatusCancelled,
        ReviewField::StatusCompleted,
        ReviewField::CardAccent,
        ReviewField::RevisionInserted,
        ReviewField::RevisionDeleted,
        ReviewField::RevisionChanged,
        ReviewField::InkLight,
        ReviewField::InkDark,
    ];

    const fn name(self) -> &'static str {
        match self {
            ReviewField::StatusAccepted => "status_accepted",
            ReviewField::StatusRejected => "status_rejected",
            ReviewField::StatusCancelled => "status_cancelled",
            ReviewField::StatusCompleted => "status_completed",
            ReviewField::CardAccent => "card_accent",
            ReviewField::RevisionInserted => "revision_inserted",
            ReviewField::RevisionDeleted => "revision_deleted",
            ReviewField::RevisionChanged => "revision_changed",
            ReviewField::InkLight => "ink_light",
            ReviewField::InkDark => "ink_dark",
        }
    }
}

impl ReviewTokens {
    fn color(&self, f: ReviewField) -> Rgba {
        match f {
            ReviewField::StatusAccepted => self.status_accepted,
            ReviewField::StatusRejected => self.status_rejected,
            ReviewField::StatusCancelled => self.status_cancelled,
            ReviewField::StatusCompleted => self.status_completed,
            ReviewField::CardAccent => self.card_accent,
            ReviewField::RevisionInserted => self.revision_inserted,
            ReviewField::RevisionDeleted => self.revision_deleted,
            ReviewField::RevisionChanged => self.revision_changed,
            ReviewField::InkLight => self.ink_light,
            ReviewField::InkDark => self.ink_dark,
        }
    }

    fn color_mut(&mut self, f: ReviewField) -> &mut Rgba {
        match f {
            ReviewField::StatusAccepted => &mut self.status_accepted,
            ReviewField::StatusRejected => &mut self.status_rejected,
            ReviewField::StatusCancelled => &mut self.status_cancelled,
            ReviewField::StatusCompleted => &mut self.status_completed,
            ReviewField::CardAccent => &mut self.card_accent,
            ReviewField::RevisionInserted => &mut self.revision_inserted,
            ReviewField::RevisionDeleted => &mut self.revision_deleted,
            ReviewField::RevisionChanged => &mut self.revision_changed,
            ReviewField::InkLight => &mut self.ink_light,
            ReviewField::InkDark => &mut self.ink_dark,
        }
    }
}

impl Family for ReviewTokens {
    type Field = ReviewField;
    const FIELDS: &'static [ReviewField] = &ReviewField::ALL;
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ReviewTokens;
        "pin" 8.0..=96.0 => pin;
        "cluster_radius" 4.0..=128.0 => cluster_radius;
        "popover_width" 120.0..=640.0 => popover_width;
        "version_dot_x" 0.0..=128.0 => version_dot_x;
        "version_text_x" 0.0..=256.0 => version_text_x;
        "chat_width" 80.0..=640.0 => chat_width;
        "chat_radius" 0.0..=48.0 => chat_radius;
        "join_width" 64.0..=640.0 => join_width;
        "revision_bar_width" 1.0..=16.0 => revision_bar_width;
        "revision_bar_height" 4.0..=64.0 => revision_bar_height;
        "card_accent_width" 1.0..=16.0 => card_accent_width;
        "pin_inset" 0.0..=16.0 => pin_inset;
        "version_dot_radius" 1.0..=16.0 => version_dot_radius;
        "version_dot_hollow_radius" 1.0..=16.0 => version_dot_hollow_radius;
    };

    fn defaults(s: &Semantic<'_>) -> ReviewTokens {
        let c = s.color;
        ReviewTokens {
            pin: values::REVIEW_PIN,
            cluster_radius: values::REVIEW_CLUSTER_RADIUS,
            popover_width: values::REVIEW_POPOVER_WIDTH,
            version_dot_x: values::REVIEW_VERSION_DOT_X,
            version_text_x: values::REVIEW_VERSION_TEXT_X,
            chat_width: values::REVIEW_CHAT_WIDTH,
            chat_radius: values::REVIEW_CHAT_RADIUS,
            join_width: values::REVIEW_JOIN_WIDTH,
            revision_bar_width: values::REVIEW_REVISION_BAR_WIDTH,
            revision_bar_height: values::REVIEW_REVISION_BAR_HEIGHT,
            card_accent_width: values::REVIEW_CARD_ACCENT_WIDTH,
            pin_inset: values::REVIEW_PIN_INSET,
            version_dot_radius: values::REVIEW_VERSION_DOT_RADIUS,
            version_dot_hollow_radius: values::REVIEW_VERSION_DOT_HOLLOW_RADIUS,
            status_accepted: c.status.good.fg,
            status_rejected: c.status.bad.fg,
            status_cancelled: c.text.faint,
            status_completed: c.status.good.fg,
            card_accent: c.accent.solid,
            revision_inserted: c.status.good.fg,
            revision_deleted: c.status.bad.fg,
            revision_changed: c.status.info.fg,
            ink_light: c.text.on_strong,
            ink_dark: values::REVIEW_INK_DARK,
        }
    }

    fn path(f: ReviewField) -> String {
        f.name().to_owned()
    }

    fn from_path(path: &str) -> Option<ReviewField> {
        ReviewField::ALL.into_iter().find(|f| f.name() == path)
    }

    fn kind(_: ReviewField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: ReviewField) -> OverrideValue {
        OverrideValue::Color(self.color(f))
    }

    fn set(&mut self, f: ReviewField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    /// The status badges are icons (WCAG 1.4.11), so each meets 3:1 (4.5:1 with high contrast) on
    /// the card's surface. A badge on the selected card (`state.selected`) has no pair: one colour
    /// can't read on both fills in every built-in (classic light's panel is `#d4d0c8` and its
    /// selected fill `#0a246a`).
    fn pairs() -> Vec<ContrastPair> {
        [ReviewField::StatusAccepted, ReviewField::StatusRejected, ReviewField::StatusCancelled, ReviewField::StatusCompleted]
            .into_iter()
            .map(|f| ContrastPair {
                fg: TokenRef::Component(ComponentField::Review(f)),
                bg: TokenRef::Color(ColorToken::SurfacePanel),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            })
            .collect()
    }
}
