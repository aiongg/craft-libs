//! What one paint pass records for later steps: text elision, the first baseline and the faces painted.

use crate::runtime::memory::sub;
use craft_behavior::PartRef;
use egui::Id;
use std::ops::RangeInclusive;

/// What `Draw` calls record. Step 16 reads the elided text and step 17 the baseline.
#[derive(Debug, Default)]
pub(crate) struct PaintRecord {
    /// The full text of the first elided run, shown as the tooltip and accessible name when no tooltip is set.
    pub elided: Option<String>,
    /// The first text's baseline, which `LabelColumn` aligns a row label to next frame.
    pub baseline: Option<f32>,
    /// The faces painted this frame.
    #[expect(dead_code, reason = "written by `PaintUi::face` (F3)")]
    pub faces: Vec<FaceRecord>,
}

/// A face painted this frame with the facts its component's `access` declared; step 16 fills the part's
/// sub-node from it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FaceRecord {
    /// The part the face looks for.
    pub part: PartRef,
    /// The instance's resolved label, the full text when elided.
    pub name: String,
    /// The face's value flags.
    pub flags: crate::style::Flags,
    /// The face's numeric value and range.
    pub value: Option<(f64, RangeInclusive<f64>)>,
}

/// The base id of the face painted for `part` of the composite `id`: owned by the composite, so the face's hover
/// and press animations key by it.
pub(crate) fn face_id(composite: Id, part: PartRef) -> Id {
    sub(composite, ("face", part))
}
