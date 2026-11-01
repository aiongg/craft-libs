//! [`HoverLookup`]: the debounced background lookup of the object under a resting pointer.

use std::collections::VecDeque;
use std::sync::Arc;

use craft_host::Jobs;
use craft_host::queue::{Cancel, JobQueue};
use egui::Context;
use kurbo::Point;

/// A miss suppresses new lookups within this many document pixels of it, on each axis.
const MISS_REACH: i64 = 2;
/// How many misses are kept; the oldest goes first.
const MISSES_KEPT: usize = 64;

/// What a lookup job found at a point.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum HoverAnswer<R> {
    /// The object under the point.
    Found(R),
    /// Nothing there (the background): the point becomes a miss.
    Nothing,
}

/// Why nothing can be found under the pointer, for the host's status line. The words are the
/// host's (they name its own options, such as Sample All Layers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HoverHint {
    /// The layer the lookup samples has no pixels under the pointer
    /// ([`HoverQuery::empty_here`]).
    EmptySampledLayer,
}

/// One pass's question to a [`HoverLookup`].
pub struct HoverQuery<'a, R> {
    /// The pointer in document units; `None` off the canvas.
    pub at: Option<Point>,
    /// The document's revision: a change drops the shown result, the misses and a running job.
    pub revision: u64,
    /// What a result depends on besides the revision (the document, the target layer, Sample
    /// All Layers), hashed by the host: a change drops the same as a revision change.
    pub key: u64,
    /// Whether hovering looks up at all: the host's option and tool (PhotoCraft: Object Selection
    /// with Object Finder on, no drag, no transform). The lookup clears itself while a pointer
    /// button is down, so the host need not fold that in.
    pub enabled: bool,
    /// The sampled layer has no pixels under the pointer (the host's quick test): the update gives
    /// [`HoverHint::EmptySampledLayer`], shows nothing and starts no job.
    pub empty_here: bool,
    /// Whether a found result already answers the point (PhotoCraft: coverage at the pixel ≥ 0.5),
    /// so moving inside an object found once shows it again with no new job.
    pub covers: &'a dyn Fn(&R, Point) -> bool,
}

/// What [`HoverLookup::update`] decided this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct HoverUpdate<'a, R> {
    /// The object to outline, if one answers the pointer.
    pub shown: Option<&'a R>,
    /// A lookup is waiting out the debounce or running; the lookup has asked for the repaint that
    /// picks it up.
    pub pending: bool,
    /// Why nothing can be found here; `None` once the pointer moves on.
    pub hint: Option<HoverHint>,
}

/// The lookup of the object under a resting pointer: PhotoCraft's Object Finder hover.
///
/// Each pass the host calls [`update`](Self::update) with the pointer and a job maker:
/// - Off the canvas, disabled, with a pointer button down or with a point that isn't finite, it
///   clears (shown result, misses and job) and gives nothing.
/// - A revision or key change drops the shown result, the misses and a running job (its result is
///   discarded when it arrives).
/// - [`HoverQuery::empty_here`] gives the hint and nothing else.
/// - A shown result that [`covers`](HoverQuery::covers) the point stays shown, with no job.
/// - Otherwise, once the pointer has rested on one document pixel for `hover.debounce_ms`
///   (120 ms) and no miss lies within 2 pixels on each axis, one job runs off the UI thread on the
///   host's [`Jobs`] ([`JobQueue`]); a job for an older pixel is cancelled. Its answer arrives on a
///   later pass: [`HoverAnswer::Found`] becomes the shown result, [`HoverAnswer::Nothing`] a miss
///   (the newest 64 are kept). The queue's wake repaints when the job ends; while the debounce
///   runs the lookup asks for one repaint at its end, so nothing polls.
///
/// `R` is the host's result (a region, a mask, both); [`HoverOutline`](super::HoverOutline) draws
/// what the host makes of it, usually a [`HoverMask`](super::HoverMask) the job built.
pub struct HoverLookup<R> {
    queue: JobQueue<u64, HoverAnswer<R>>,
    /// The revision and key the state below belongs to.
    revision: u64,
    key: u64,
    /// The pointer's document pixel and the input time it got there.
    pixel: Option<[i64; 2]>,
    since: f64,
    /// The running job's number (its queue key) and the pixel it looks at.
    job: Option<(u64, [i64; 2])>,
    /// The next job number.
    next_job: u64,
    misses: VecDeque<[i64; 2]>,
    shown: Option<R>,
}

const _: () = {
    const fn send<T: Send>() {}
    send::<HoverLookup<Vec<u8>>>();
};

impl<R: Send + 'static> HoverLookup<R> {
    /// A lookup whose jobs run on `jobs`, or the installed host's
    /// ([`Host::jobs_shared`](craft_host::Host::jobs_shared)), or a thread per job (inline on
    /// wasm32) with no host. One lane; the wake is `ctx.request_repaint()`. Stories and tests pass
    /// a [`ManualJobs`](craft_host::ManualJobs).
    pub fn new(ctx: &Context, jobs: Option<Arc<dyn Jobs>>) -> Self {
        // new: as `TextureLoader::in_context` (crates/ui/src/canvas/textures/loader.rs:182-200: the
        // host's jobs, the repaint wake), with `QueueOptions { lanes: 1, name: "craft-hover" }`.
        todo!()
    }

    /// Advances the lookup for this pass (see the type's rules) and returns what to show.
    /// `job(at)` is called only when a lookup starts, with the pointer's document point; the job
    /// it returns runs off the UI thread and checks its [`Cancel`] between steps.
    pub fn update<J>(&mut self, ctx: &Context, query: HoverQuery<'_, R>, job: impl FnOnce(Point) -> J) -> HoverUpdate<'_, R>
    where
        J: FnOnce(&Cancel) -> HoverAnswer<R> + Send + 'static,
    {
        // pc: crates/ui-egui/src/tools/object_finder.rs:81-137 `update` (freshness by revision and key
        // :92-95, empty-layer hint :97-101, pixel and time :103-106, a finished job into the shown
        // result or the misses (64) :108-126, the shown region serves points inside it :127-130, the
        // 2 px miss test and the debounce :131-134, busy :135); :26 `DEBOUNCE` 0.12 s, read here
        // from `HoverOutline::style` (`hover.debounce_ms`). The thread per lookup becomes one
        // `JobQueue` lane.
        todo!()
    }

    /// Drops the shown result, the misses and a running job (a tool change, the canvas closed).
    pub fn clear(&mut self) {
        // pc: crates/ui-egui/src/tools/object_finder.rs:159-162 `clear`.
        todo!()
    }

    /// The result shown on the last update, if any.
    pub fn shown(&self) -> Option<&R> {
        // new: the field.
        todo!()
    }
}
