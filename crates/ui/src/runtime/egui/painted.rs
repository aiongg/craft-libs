//! The full text of each elided run painted in the current pass.

use super::memory::Service;
use egui::Context;
use std::collections::HashMap;

/// The full text of each elided run painted in the current pass, by the text its galley shows. A path cut by
/// `Wrap::ElidePath` is a whole galley to egui, so what reads the painted shapes asks here instead.
#[derive(Default)]
struct Elisions {
    pass: u64,
    full: HashMap<String, String>,
}

/// Records that the run whose galley shows `shown` was painted with `full` as its uncut text.
pub(crate) fn note_elided(ctx: &Context, shown: &str, full: &str) {
    let pass = ctx.cumulative_pass_nr();
    Service::<Elisions>::of(ctx).with(|log| {
        if log.pass != pass {
            log.pass = pass;
            log.full.clear();
        }
        log.full.insert(shown.to_owned(), full.to_owned());
    });
}

/// The full text of the painted run whose galley shows `shown`, when its layout cut text.
pub(crate) fn full_text_of_painted(ctx: &Context, shown: &str) -> Option<String> {
    Service::<Elisions>::of(ctx).with(|log| log.full.get(shown).cloned())
}
