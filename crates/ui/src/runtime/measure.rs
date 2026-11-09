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

/// The effects skipped in measuring runs this frame, in order; empties the log.
#[cfg(feature = "testing")]
pub fn take_effects(ctx: &Context) -> Vec<&'static str> {
    let frame = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let log = data.get_temp_mut_or_default::<(u64, Vec<&'static str>)>(Id::unique("craft-ui measuring run effects"));
        if log.0 == frame { std::mem::take(&mut log.1) } else { Vec::new() }
    })
}
