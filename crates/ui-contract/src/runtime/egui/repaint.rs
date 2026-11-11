//! `request_repaint_after`, the one timed wake, the immediate repaint request every other caller goes through, and
//! the record of immediate requests the settle query reads.

use egui::{Context, RequestRepaintInfo, ViewportId};

use super::memory::Service;

/// Per viewport, the pass count (`RequestRepaintInfo::current_cumulative_pass_nr`) at the latest immediate repaint
/// request, from any caller: egui itself, app code, another thread.
#[derive(Default)]
struct Requested(Vec<(ViewportId, u64)>);

/// Records every immediate repaint request of `ctx`, through egui's repaint observer.
pub(crate) fn observe(ctx: &Context) {
    let requested = Service::<Requested>::of(ctx);
    ctx.set_repaint_observer(move |info: RequestRepaintInfo| {
        if !info.delay.is_zero() {
            return;
        }
        requested.with(|requested| match requested.0.iter_mut().find(|(viewport, _)| *viewport == info.viewport_id) {
            Some((_, nr)) => *nr = (*nr).max(info.current_cumulative_pass_nr),
            None => requested.0.push((info.viewport_id, info.current_cumulative_pass_nr)),
        });
    });
}

/// Whether an immediate repaint of `viewport` was asked for and has not run. A request made during a pass counts the
/// passes completed before it, so a later pass has run once two more completed; one made between passes is reported a
/// pass longer than needed, never shorter.
pub(crate) fn pending_now(ctx: &Context, viewport: ViewportId) -> bool {
    let asked = Service::<Requested>::run(ctx, |requested| requested.0.iter().find(|(id, _)| *id == viewport).map(|(_, nr)| *nr));
    asked.is_some_and(|nr| ctx.cumulative_pass_nr_for(viewport) < nr + 2)
}

/// Asks for one more pass after this one.
pub(crate) fn request(ctx: &Context) {
    ctx.request_repaint();
}

/// Asks for a pass of `viewport` at once.
pub(crate) fn request_of(ctx: &Context, viewport: egui::ViewportId) {
    ctx.request_repaint_of(viewport);
}

/// Asks for one pass of `viewport` after `seconds`; the earliest request of a frame wins.
pub(crate) fn request_after(ctx: &Context, viewport: egui::ViewportId, seconds: f64) {
    ctx.request_repaint_after_for(std::time::Duration::from_secs_f64(seconds.max(0.0)), viewport);
}

/// The animation time that makes egui's own animations jump; zero turns motion off.
pub(crate) fn animation_time(ctx: &Context) -> f32 {
    ctx.global_style().animation_time
}
