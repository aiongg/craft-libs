//! Conformance: an app's real appearance setup compared with the reference that craft-ui builds
//! for the same inputs.
//!
//! The app's setup code runs on a fresh headless [`Context`] for a [`Case`]; the reference runs
//! on another, through [`Base::resolve`] and [`install`]. Both are read the same way (the
//! tokens, their egui values, the two installed styles, egui's theme preference and the
//! direction) and compared key by key. A difference fails the app's test with a [`Report`]: the
//! app selected the wrong theme, size or text size, changed the `Style` after `install`, set
//! egui's theme preference itself, or installed a font stack that differs from the reference's.
//!
//! ```no_run
//! use craft_ui::conformance::{Case, Conformance};
//!
//! # fn my_apply(_: &egui::Context, _: &Case) {}
//! # fn my_fonts(_: &egui::Context, _: craft_theme::InterfaceFont) {}
//! let check = Conformance::new(my_apply, my_fonts)
//!     .allow("style.spacing.scroll", "thin floating scroll bars");
//! check.assert_all(Case::matrix().into_iter().chain(Case::axes()));
//! ```
//!
//! The closure under test is the app's production path from the selection to `install` and the
//! font install, shared with `main`. Fields of `Style` that no token sets are not compared (see
//! [`style_for`](crate::theme::style_for)); an app that departs on a compared one lists it with
//! [`Conformance::allow`] and a reason.

mod case;
mod compare;
mod report;
#[cfg(test)]
mod tests;

use craft_theme::InterfaceFont;
use egui::Context;

pub use case::Case;
pub use report::Report;

use compare::{Diff, Side, diff_rows};
use report::{Details, Section};

use crate::direction;
use crate::theme::{Base, install};

/// A style or context key prefix the app may differ on, with the reason.
#[derive(Clone, Debug)]
struct Allowance {
    prefix: String,
    reason: String,
}

type Apply<'a> = Box<dyn Fn(&Context, &Case) + 'a>;
type Fonts<'a> = Box<dyn Fn(&Context, InterfaceFont) + 'a>;

/// The comparison of an app's appearance setup with the reference.
pub struct Conformance<'a> {
    apply: Apply<'a>,
    fonts: Fonts<'a>,
    frames: usize,
    allowed: Vec<Allowance>,
}

impl<'a> Conformance<'a> {
    /// `apply` is the app's real appearance setup, called once on a fresh `Context` per case.
    /// `fonts` is the app's real font-stack setup for an interface font; the reference calls it
    /// too, so faces, `fit_interact_size` and the font families match by construction.
    pub fn new(apply: impl Fn(&Context, &Case) + 'a, fonts: impl Fn(&Context, InterfaceFont) + 'a) -> Self {
        Self { apply: Box::new(apply), fonts: Box::new(fonts), frames: 2, allowed: Vec::new() }
    }

    /// Frames run on each side before it is read, for setup that finishes on a later pass.
    pub fn frames(mut self, frames: usize) -> Self {
        self.frames = frames;
        self
    }

    /// Lets the keys that start with `key_prefix` (`style.` or `context.` keys) differ. The
    /// list is the record of every place the app departs from the reference. An allowance that
    /// matches no difference fails the check.
    ///
    /// Panics on an empty reason or a key outside `style.` and `context.`: tokens and the
    /// converted values come from the theme and cannot differ.
    #[allow(clippy::panic)] // a test assertion
    pub fn allow(mut self, key_prefix: &str, reason: &str) -> Self {
        if reason.trim().is_empty() {
            panic!("conformance: the allowance for `{key_prefix}` needs a reason");
        }
        if !(key_prefix.starts_with("style.") || key_prefix.starts_with("context.")) {
            panic!("conformance: `{key_prefix}` cannot be allowed; only `style.` and `context.` keys can differ");
        }
        self.allowed.push(Allowance { prefix: key_prefix.to_owned(), reason: reason.to_owned() });
        self
    }

    /// Compares the app's appearance for `case` with the reference.
    pub fn check(&self, case: &Case) -> Result<(), Report> {
        let mut outcome = self.run(case);
        let unused = self.unused(&outcome.used);
        let failed = outcome.differs || !unused.is_empty();
        outcome.report.details_mut().unused = unused;
        if failed { Err(outcome.report) } else { Ok(()) }
    }

    /// [`check`](Self::check), failing the test with the report.
    #[track_caller]
    #[allow(clippy::panic)] // a test assertion
    pub fn assert(&self, case: &Case) {
        if let Err(report) = self.check(case) {
            panic!("{report}");
        }
    }

    /// Checks every case and fails once, listing each failed case and the allowances no case
    /// used.
    #[track_caller]
    #[allow(clippy::panic)] // a test assertion
    pub fn assert_all(&self, cases: impl IntoIterator<Item = Case>) {
        const ROWS: usize = 10;
        let mut used = vec![0; self.allowed.len()];
        let (mut total, mut failed) = (0, Vec::new());
        for case in cases {
            total += 1;
            let outcome = self.run(&case);
            for (sum, n) in used.iter_mut().zip(&outcome.used) {
                *sum += n;
            }
            if outcome.differs {
                failed.push(outcome.report);
            }
        }
        let unused = self.unused(&used);
        if failed.is_empty() && unused.is_empty() {
            return;
        }
        let mut message = String::new();
        for report in &failed {
            message.push_str(&report.excerpt(ROWS));
            message.push_str("\n\n");
        }
        for (prefix, reason) in &unused {
            message.push_str(&format!("unused allowance: {prefix}  {reason:?} (matches no difference in any case; remove it)\n"));
        }
        panic!("conformance: {} of {total} cases failed\n\n{message}", failed.len());
    }

    fn unused(&self, used: &[usize]) -> Vec<(String, String)> {
        self.allowed.iter().zip(used).filter(|(_, n)| **n == 0).map(|(a, _)| (a.prefix.clone(), a.reason.clone())).collect()
    }

    fn run(&self, case: &Case) -> Outcome {
        let app_ctx = Context::default();
        (self.apply)(&app_ctx, case);
        let app = Side::read(&app_ctx, self.frames);

        let reference_ctx = Context::default();
        (self.fonts)(&reference_ctx, case.font());
        direction::apply(&reference_ctx, case.dir());
        let base = Base::resolve(case.selection(), case.system_appearance(), case.app_defaults(), case.app_id(), &mut case.lookup());
        install(&reference_ctx, &base);
        let reference = Side::read(&reference_ctx, self.frames);

        let mut used = vec![0; self.allowed.len()];
        let mut sections = Vec::new();
        for (r, a) in reference.modes.iter().zip(&app.modes) {
            let mode = case::mode_text(r.mode);
            let style = self.filter("style", diff_rows(&r.style, &a.style), &mut used);
            sections.push(Section { title: format!("style ({mode})"), diffs: style });
            sections.push(Section { title: format!("tokens ({mode})"), diffs: diff_rows(&r.tokens, &a.tokens) });
            sections.push(Section { title: format!("ui ({mode})"), diffs: diff_rows(&r.ui, &a.ui) });
        }
        let context = self.filter("context", diff_rows(&reference.context, &app.context), &mut used);
        sections.push(Section { title: "context".to_owned(), diffs: context });

        let shown = reference.modes.iter().find(|m| m.mode == reference.active).map_or(reference.active, |m| m.shown);
        let mode = format!("mode {}, shown {}", case.mode_name(), case::mode_text(shown));
        let allowed = self.allowed.iter().zip(&used).filter(|(_, n)| **n > 0).map(|(a, n)| (a.prefix.clone(), a.reason.clone(), *n)).collect();
        let hints = hints(&sections);
        let differs = sections.iter().any(|s| !s.diffs.is_empty());
        Outcome { report: Report::new(Details { case: case.name(), mode, sections, allowed, unused: Vec::new(), hints }), differs, used }
    }

    /// The differences of `section` the allowances do not cover; counts the ones they cover.
    fn filter(&self, section: &str, diffs: Vec<Diff>, used: &mut [usize]) -> Vec<Diff> {
        diffs
            .into_iter()
            .filter(|d| {
                let key = format!("{section}.{}", d.key);
                match self.allowed.iter().position(|a| key.starts_with(&a.prefix)) {
                    Some(i) => {
                        used[i] += 1;
                        false
                    }
                    None => true,
                }
            })
            .collect()
    }
}

struct Outcome {
    report: Report,
    /// Whether any difference is left after the allowances.
    differs: bool,
    /// Differences each allowance covered, in the order of the allowances.
    used: Vec<usize>,
}

/// Likely causes, chosen by which sections differ.
fn hints(sections: &[Section]) -> Vec<String> {
    let differs = |prefix: &str| sections.iter().any(|s| s.title.starts_with(prefix) && !s.diffs.is_empty());
    let mut hints = Vec::new();
    if differs("tokens") {
        hints.push(
            "the tokens differ: the app resolved a different theme, size, contrast, text size, per-app override or default than the case. \
             Check the selection, system appearance, defaults and app id the setup receives."
                .to_owned(),
        );
    } else if differs("ui") {
        hints.push(
            "the tokens match but the converted values differ: the app's font stack differs from the `fonts` setup \
             (role fonts, font families, optical size)."
                .to_owned(),
        );
    } else if differs("style") {
        let only_interact_size =
            sections.iter().filter(|s| s.title.starts_with("style")).flat_map(|s| &s.diffs).all(|d| d.key.starts_with("spacing.interact_size"));
        if only_interact_size {
            hints.push(
                "only `spacing.interact_size` differs: the app's font stack differs from `fonts` (`Stack::fit_interact_size` \
                 adds to the interaction height)."
                    .to_owned(),
            );
        } else {
            hints.push(
                "tokens and ui match but style differs: the app changed the Style after `install` \
                 (search for `global_style_mut`, `set_visuals`, `set_style`, `style_mut`)."
                    .to_owned(),
            );
        }
    }
    let context_has = |key: &str| sections.iter().any(|s| s.title == "context" && s.diffs.iter().any(|d| d.key == key));
    if context_has("theme_preference") {
        hints.push("the app sets egui's theme preference itself (`set_theme`); `install` sets it from the selection's mode.".to_owned());
    }
    if context_has("direction") {
        hints.push("the app applied a different direction than the case's; call `direction::apply` with `case.dir()`.".to_owned());
    }
    hints
}
