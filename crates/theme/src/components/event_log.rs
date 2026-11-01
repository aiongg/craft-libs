//! Event logs: the level glyphs and the details box. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// EventLog tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EventLogTokens {
    /// Least height of the details box: `event_log.details_height`.
    pub details_height: f32,
    /// Side of a level glyph: the error square's side, twice the info dot's radius: `event_log.glyph`.
    pub glyph: f32,
    /// The room a level glyph takes in the bar and in a row: `event_log.glyph_box`.
    pub glyph_box: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventLogField {}

impl Family for EventLogTokens {
    type Field = EventLogField;
    const FIELDS: &'static [EventLogField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { EventLogTokens;
        "details_height" 0.0..=1024.0 => details_height;
        "glyph" 0.0..=64.0 => glyph;
        "glyph_box" 0.0..=64.0 => glyph_box;
    };

    fn defaults(_: &Semantic<'_>) -> EventLogTokens {
        EventLogTokens { details_height: values::EVENT_LOG_DETAILS_HEIGHT, glyph: values::EVENT_LOG_GLYPH, glyph_box: values::EVENT_LOG_GLYPH_BOX }
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
