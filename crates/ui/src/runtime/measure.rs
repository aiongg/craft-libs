//! The ctx-side view of a measuring run: the sizing pass the framework starts to measure a container's contents
//! before the real run. APIs that get only a `Context` ask here whether to skip an effect, so a measuring run
//! delivers nothing the real run then lacks.

use egui::{Context, Id};

fn depth_id() -> Id {
    Id::unique("craft-ui measuring run depth")
}

/// Runs `f` as a measuring run. Nested calls are fine.
pub fn run<R>(ctx: &Context, f: impl FnOnce() -> R) -> R {
    let set = |to: u32| ctx.data_mut(|data| data.insert_temp(depth_id(), to));
    let before = depth(ctx);
    set(before + 1);
    let out = f();
    set(before);
    out
}

fn depth(ctx: &Context) -> u32 {
    ctx.data(|data| data.get_temp::<u32>(depth_id())).unwrap_or(0)
}

/// Whether a measuring run is in progress.
pub fn active(ctx: &Context) -> bool {
    depth(ctx) > 0
}

/// Whether the effect `what` may happen: `false` in a measuring run, and the caller skips it. Under test the
/// skipped effects are logged for `take_effects`.
pub fn effect(ctx: &Context, what: &'static str) -> bool {
    if !active(ctx) {
        return true;
    }
    #[cfg(any(test, feature = "testing"))]
    let frame = ctx.cumulative_pass_nr();
    #[cfg(any(test, feature = "testing"))]
    ctx.data_mut(|data| {
        let log = data.get_temp_mut_or_default::<(u64, Vec<&'static str>)>(Id::unique("craft-ui measuring run effects"));
        if log.0 != frame {
            *log = (frame, Vec::new());
        }
        log.1.push(what);
    });
    #[cfg(not(any(test, feature = "testing")))]
    let _ = what;
    false
}

/// Takes the entries of the one-shot `queue` that `wanted` accepts. A measuring run takes none and leaves the queue
/// whole, so the real run finds them. Every take of a one-shot queue that a body or a component's
/// steps can reach goes through here.
pub fn take_queued<T>(ctx: &Context, queue: &mut Vec<T>, wanted: impl Fn(&T) -> bool) -> Vec<T> {
    if active(ctx) {
        return Vec::new();
    }
    let mut mine = Vec::new();
    let mut kept = Vec::new();
    for item in std::mem::take(queue) {
        if wanted(&item) { mine.push(item) } else { kept.push(item) }
    }
    *queue = kept;
    mine
}

/// The effects skipped in measuring runs this frame, in order; empties the log.
#[cfg(feature = "testing")]
pub fn take_effects(ctx: &Context) -> Vec<&'static str> {
    let frame = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let log = data.get_temp_mut_or_default::<(u64, Vec<&'static str>)>(Id::unique("craft-ui measuring run effects"));
        if log.0 == frame { std::mem::take(&mut log.1) } else { Vec::new() }
    })
}

#[cfg(feature = "testing")]
fn runs_id() -> Id {
    Id::unique("craft-ui measuring runs per body")
}

/// Counts a measuring run of the contents of the body `id` in this pass.
#[cfg(feature = "testing")]
pub(crate) fn count_body(ctx: &Context, id: Id) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let log = data.get_temp_mut_or_default::<(u64, Vec<(Id, u32)>)>(runs_id());
        if log.0 != pass {
            *log = (pass, Vec::new());
        }
        match log.1.iter_mut().find(|(body, _)| *body == id) {
            Some((_, count)) => *count += 1,
            None => log.1.push((id, 1)),
        }
    });
}

/// The bodies whose contents ran in a measuring run in the last pass that ran one, and how many runs each.
#[cfg(feature = "testing")]
pub fn body_runs(ctx: &Context) -> Vec<(Id, u32)> {
    ctx.data(|data| data.get_temp::<(u64, Vec<(Id, u32)>)>(runs_id())).map(|(_, runs)| runs).unwrap_or_default()
}
