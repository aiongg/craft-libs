//! [`DraftScale`]: LightCraft's learned size of a slider drag's whole-frame draft.

use std::collections::VecDeque;

/// The size of a slider drag's whole-frame draft, chosen when the drag starts and kept until it
/// ends (a change in the middle would make the resolution jump), unless its full-size drafts are
/// far over budget. Opt-in per app: LightCraft uses it, PhotoCraft keeps its Low Resolution
/// Previews switch ([`ProxyPlan`](super::ProxyPlan)).
///
/// Rules (LightCraft's `DragScale`, constants exact):
/// - A drag renders at full size (1.0) when the GPU draws the view and the (item, control) pair
///   is not known slow; otherwise at [`SMALL`](Self::SMALL) (0.6).
/// - A draft's time is its render plus the frame that shows it (the upload is part of it); a
///   host frame time over 1000 ms counts as unknown (0). Each render key is timed once.
/// - Mid-drag give-up: with more than 3 full-size samples whose median after the first exceeds
///   2 × [`BUDGET_MS`](Self::BUDGET_MS) (16 ms), the drag drops to [`SMALL`](Self::SMALL).
/// - At the drag's end, a drag timed at full size with a median after the first over
///   1.25 × [`BUDGET_MS`](Self::BUDGET_MS) (20 ms) counts against its pair: a first time makes it
///   suspect (the next drag stays full size), a second slow for 30 s, and each slow retry doubles
///   the wait up to 240 s; a drag within it forgets the verdict. A drag never timed at full size
///   changes nothing.
/// - Verdicts are per item (another photo is timed afresh); at most 256 pairs are kept, the one
///   remembered longest ago forgotten first.
///
/// Times: `now` is the host's clock in seconds (egui's `input.time`), durations in milliseconds.
#[derive(Clone, Debug, Default)]
pub struct DraftScale {
    /// The drag running now: (item, control) and its scale.
    current: Option<((u64, String), f32)>,
    /// How long the current drag's full-size drafts took to show (ms), at most 1024.
    samples: Vec<f64>,
    /// The render key last timed.
    seen: Option<u64>,
    /// A shown draft's render time, waiting for the frame time of the pass that uploaded it.
    pending: Option<f64>,
    /// Pairs found over budget, the one remembered longest ago first.
    verdicts: VecDeque<((u64, String), Verdict)>,
}

/// What [`DraftScale`] remembers about the drags of one control on one item.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Verdict {
    /// One drag was over budget at full size: the next stays full size.
    Suspect,
    /// Two in a row were: drafts are small until `until` (s); `wait` is how long this holds (s).
    Slow { until: f64, wait: f64 },
}

impl DraftScale {
    /// The scale of a draft that is not full size.
    pub const SMALL: f32 = 0.6;

    /// The slider budget per draft, in milliseconds.
    pub const BUDGET_MS: f64 = 16.0;

    /// The draft scale for this pass, or `None` without a drag. `drag` is the item (the host's id
    /// of the photo or document) and the control being dragged; `gpu` whether the GPU draws this
    /// view; `shown` the key of the whole-frame render on screen (a render from before the drag
    /// is not one of its drafts). A drag of another pair, or `None`, ends the running drag first
    /// (its verdict is updated at `now`).
    pub fn scale(&mut self, drag: Option<(u64, &str)>, gpu: bool, shown: Option<u64>, now: f64) -> Option<f32> {
        // lightcraft: crates/ui-egui/src/region.rs:176 `DragScale::scale`, :99 `CPU_DRAFT_SCALE` (0.6).
        todo!()
    }

    /// Once per pass: `key` names the whole-frame render on screen and `render_ms` how long it
    /// took; `last_frame_ms` how long the previous pass took, painting included (`None` without a
    /// host report, e.g. headless). Times the current drag's full-size drafts and gives up on full
    /// size when they are far over budget (the type's rules).
    pub fn observe(&mut self, key: u64, render_ms: f64, last_frame_ms: Option<f64>) {
        // lightcraft: crates/ui-egui/src/region.rs:199 `observe`, :103 `DRAFT_BUDGET_MS` (16), :107
        // `GIVE_UP_FACTOR` (2), :110 `GIVE_UP_AFTER` (3), :118 `FRAME_MS_KNOWN` (1000), :288
        // `median_after_first`.
        todo!()
    }

    /// The drag ended: updates the pair's verdict from the drag's full-size drafts (the type's
    /// rules) and clears the drag. [`scale`](Self::scale) with `None` calls it.
    fn finish(&mut self, now: f64) {
        // lightcraft: crates/ui-egui/src/region.rs:264 `finish`, :122 `VERDICT_MS` (16 × 1.25), :127
        // `RETRY_AFTER_S` (30), :130 `RETRY_MAX_S` (240), :239 `set_verdict`, :114 `VERDICTS_KEPT`
        // (256).
        todo!()
    }
}
