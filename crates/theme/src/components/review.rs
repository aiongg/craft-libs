//! Review: the sizes of comment pins, thread popovers, version history, cursor chat and revision
//! bars, and the colours of its status badges and revision kinds. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Review tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReviewTokens {
    /// A comment pin's size: [`values::REVIEW_PIN`].
    pub pin: f32,
    /// Pins nearer than this gather into a cluster: [`values::REVIEW_CLUSTER_RADIUS`].
    pub cluster_radius: f32,
    /// The thread popover's width: [`values::REVIEW_POPOVER_WIDTH`].
    pub popover_width: f32,
    /// The version history's timeline dot column: [`values::REVIEW_VERSION_DOT_X`].
    pub version_dot_x: f32,
    /// The version history's text column: [`values::REVIEW_VERSION_TEXT_X`].
    pub version_text_x: f32,
    /// A cursor chat bubble's width: [`values::REVIEW_CHAT_WIDTH`].
    pub chat_width: f32,
    /// A cursor chat bubble's corner radius: [`values::REVIEW_CHAT_RADIUS`].
    pub chat_radius: f32,
    /// The join field's width: [`values::REVIEW_JOIN_WIDTH`].
    pub join_width: f32,
    /// The revision card's colour bar width: [`values::REVIEW_REVISION_BAR_WIDTH`].
    pub revision_bar_width: f32,
    /// The revision card's colour bar height: [`values::REVIEW_REVISION_BAR_HEIGHT`].
    pub revision_bar_height: f32,
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
}

/// Review tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReviewField {}

impl Family for ReviewTokens {
    type Field = ReviewField;
    const FIELDS: &'static [ReviewField] = &[];
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
            status_accepted: c.status.good.fg,
            status_rejected: c.status.bad.fg,
            status_cancelled: c.text.faint,
            status_completed: c.status.good.fg,
            card_accent: c.accent.solid,
            revision_inserted: c.status.good.fg,
            revision_deleted: c.status.bad.fg,
            revision_changed: c.status.info.fg,
        }
    }

    fn path(f: ReviewField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ReviewField> {
        None
    }

    fn kind(f: ReviewField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ReviewField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ReviewField, _: OverrideValue) {
        match f {}
    }

    // Glyphs and bars drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
