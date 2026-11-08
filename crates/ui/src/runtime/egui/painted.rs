//! The full text of each elided run painted in the current pass, for the test fixture's paint reads: builds without
//! the `testing` feature record nothing.

#[cfg(feature = "testing")]
use super::memory::Service;
use egui::{Context, Galley};
#[cfg(feature = "testing")]
use std::collections::HashMap;
use std::sync::Arc;

/// The full text of each elided run painted in the current pass, by galley identity. A galley that egui cut (`Wrap::Elide`)
/// keeps the full text in its job and one cut by `Wrap::ElidePath` shows only the cut text, so neither the job's text nor
/// the galley's own flag tells a painted cut run from a whole one that shares its text (a tooltip under the elided
/// title). The log holds the galley, so its address names it for the whole pass.
#[cfg(feature = "testing")]
#[derive(Default)]
struct Elisions {
    pass: u64,
    full: HashMap<usize, (Arc<Galley>, String)>,
}

#[cfg(feature = "testing")]
fn address(galley: &Arc<Galley>) -> usize {
    Arc::as_ptr(galley) as usize
}

/// Records that the painted `galley` is a run cut from `full`.
#[cfg(feature = "testing")]
pub(crate) fn note_elided(ctx: &Context, galley: &Arc<Galley>, full: &str) {
    let pass = ctx.cumulative_pass_nr();
    Service::<Elisions>::of(ctx).with(|log| {
        if log.pass != pass {
            log.pass = pass;
            log.full.clear();
        }
        log.full.insert(address(galley), (galley.clone(), full.to_owned()));
    });
}

#[cfg(not(feature = "testing"))]
pub(crate) fn note_elided(_: &Context, _: &Arc<Galley>, _: &str) {}

/// The uncut text of the painted `galley`, when its layout cut text.
#[cfg(feature = "testing")]
pub(crate) fn full_text_of_painted(ctx: &Context, galley: &Arc<Galley>) -> Option<String> {
    Service::<Elisions>::of(ctx).with(|log| log.full.get(&address(galley)).map(|(_, full)| full.clone()))
}
