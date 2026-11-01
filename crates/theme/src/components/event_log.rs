//! Event logs: the details box. The level glyphs are the notices' tone icons. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// EventLog tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EventLogTokens {
    /// Least height of the details box: `event_log.details_height`.
    pub details_height: f32,
    /// Vertical inset of the details box's text: `event_log.details_pad_y`.
    pub details_pad_y: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventLogField {}

impl Family for EventLogTokens {
    type Field = EventLogField;
    const FIELDS: &'static [EventLogField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { EventLogTokens;
        "details_height" 0.0..=1024.0 => details_height;
        "details_pad_y" 0.0..=64.0 => details_pad_y;
    };

    fn defaults(_: &Semantic<'_>) -> EventLogTokens {
        EventLogTokens { details_height: values::EVENT_LOG_DETAILS_HEIGHT, details_pad_y: values::EVENT_LOG_DETAILS_PAD_Y }
    }

    fn path(f: EventLogField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<EventLogField> {
        None
    }

    fn kind(f: EventLogField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: EventLogField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: EventLogField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
