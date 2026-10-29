//! Conformance: an app's real appearance setup compared with the reference that craft-ui builds
//! for the same inputs.
//!
//! The app's setup code runs on a fresh headless [`Context`] for a [`Case`]; the reference runs
//! on another, through [`Base::resolve`] and [`install`]. Both are read the same way (the
//! tokens, their egui values, the two installed styles, egui's theme preference and the
//! direction) and compared key by key. A difference fails the app's test with a [`Report`]: the
//! app selected the wrong theme, size or text size, changed the `Style` after `install`, set
//! egui's theme preference, zoom factor, `override_font_id` or `override_text_style` itself, or
//! installed a font stack that differs from the reference's. The reference's own font stack is
//! checked against what craft-ui builds for the case's interface font (a `fonts` setup that
//! ignores the font fails on its own).
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
//! The closures under test are the app's production path from the selection to `install` and the
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
use crate::theme::{Base, FontFamilies, install};

/// A style or context key prefix the app may differ on, with the reason.
#[derive(Clone, Debug)]
struct Allowance {
    prefix: String,
    reason: String,
}

impl Allowance {
    /// Whether `key` is the allowed key or lies under it.
    fn covers(&self, key: &str) -> bool {
        key.strip_prefix(&self.prefix).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
    }
}

type Apply<'a> = Box<dyn Fn(&Context, &Case) + 'a>;
type Fonts<'a> = Box<dyn Fn(&Context, InterfaceFont) + 'a>;

/// The comparison of an app's appearance setup with the reference.
pub struct Conformance<'a> {
    apply: Apply<'a>,
    fonts: Fonts<'a>,
    frames: usize,
    allowed: Vec<Allowance>,
    faces_embedded: bool,
}

impl<'a> Conformance<'a> {
    /// `apply` is the app's real appearance setup, called once on a fresh `Context` per case.
    /// `fonts` is the app's real font-stack setup for an interface font; the reference calls it
    /// too, so the faces and `fit_interact_size` match. The reference's [`FontFamilies`] are
    /// checked against the interface font of the case.
    pub fn new(apply: impl Fn(&Context, &Case) + 'a, fonts: impl Fn(&Context, InterfaceFont) + 'a) -> Self {
        Self { apply: Box::new(apply), fonts: Box::new(fonts), frames: 2, allowed: Vec::new(), faces_embedded: false }
    }

    /// Frames run on each side before it is read, for setup that finishes on a later pass.
    pub fn frames(mut self, frames: usize) -> Self {
        self.frames = frames;
        self
    }

    /// Lets the `style.` or `context.` key `key_prefix` and the keys under it
    /// (`key_prefix` followed by `.`) differ: `style.spacing.scroll` covers
    /// `style.spacing.scroll.bar_width` but not `style.spacing.scroll_other`. The list is the
    /// record of every place the app departs from the reference. An allowance that matches no
    /// difference fails the check.
    ///
    /// Panics on an empty reason, a key outside `style.` and `context.` (tokens and the
    /// converted values come from the theme and cannot differ), a bare section (`style.`,
    /// `context.`) or a key that ends in `.`.
    #[allow(clippy::panic)] // a test assertion
    pub fn allow(mut self, key_prefix: &str, reason: &str) -> Self {
        if reason.trim().is_empty() {
            panic!("conformance: the allowance for `{key_prefix}` needs a reason");
        }
        let Some(key) = key_prefix.strip_prefix("style.").or_else(|| key_prefix.strip_prefix("context.")) else {
            panic!("conformance: `{key_prefix}` cannot be allowed; only `style.` and `context.` keys can differ");
        };
        if key.is_empty() || key.ends_with('.') {
            panic!("conformance: `{key_prefix}` is not a key; name a key such as `style.visuals.striped`, without a trailing `.`");
        }
        self.allowed.push(Allowance { prefix: key_prefix.to_owned(), reason: reason.to_owned() });
        self
    }

    /// Whether the app's build embeds craft-fonts faces (its `build.rs` embedded some, so its
    /// `CRAFT_FONTS` list is not empty). Such an app installs a font stack, and a case where
    /// `fonts` installs none differs ("no font stack installed"). Default `false`: an app
    /// without embedded faces installs no stack and has nothing to differ. craft-ui cannot see
    /// this from its own build, so the app says it.
    pub fn faces_embedded(mut self, embedded: bool) -> Self {
        self.faces_embedded = embedded;
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
        // The system face resolves in the background; a stack installed before it resolves has
        // not got it yet. Waiting here gives both sides the face from their first pass on.
        if case.font() == InterfaceFont::System {
            let _ = crate::fonts::system_ui_face();
        }
        let app_ctx = Context::default();
        (self.apply)(&app_ctx, case);
        let app = Side::read(&app_ctx, self.frames);

        let reference_ctx = Context::default();
        (self.fonts)(&reference_ctx, case.font());
        direction::apply(&reference_ctx, case.dir());
        let base = Base::resolve(case.selection(), case.system_appearance(), case.app_defaults(), case.app_id(), &mut case.lookup());
        install(&reference_ctx, &base);
        let reference = Side::read(&reference_ctx, self.frames);
        let fonts = font_families_diffs(&reference_ctx, case.font(), self.faces_embedded);

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
        sections.push(Section { title: "fonts".to_owned(), diffs: fonts });

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
                match self.allowed.iter().position(|a| a.covers(&key)) {
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

/// Where the families `fonts` installed differ from the ones craft-ui builds for `font`:
/// `reference` is what the interface font implies, `app` what the stack installed. Only the
/// optical size follows from the interface font; the families a stack offers are the app's. An
/// app that installs no stack differs when its build embeds faces (`faces_embedded`), and has
/// nothing to differ when it embeds none.
fn font_families_diffs(ctx: &Context, font: InterfaceFont, faces_embedded: bool) -> Vec<Diff> {
    if crate::fonts::installed_families(ctx).is_none() {
        if !faces_embedded {
            return Vec::new();
        }
        return vec![Diff {
            key: "families.installed".to_owned(),
            reference: Some("a font stack".to_owned()),
            app: Some("no font stack installed".to_owned()),
        }];
    }
    let expected = font == InterfaceFont::System && crate::fonts::system_ui_face().is_some();
    let installed = FontFamilies::installed(ctx).optical_size;
    if installed == expected {
        return Vec::new();
    }
    vec![Diff { key: "families.optical_size".to_owned(), reference: Some(expected.to_string()), app: Some(installed.to_string()) }]
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
    if differs("fonts") {
        hints.push(
            "the `fonts` setup does not follow the case's interface font: the system interface face is expected exactly when the case \
             asks for `InterfaceFont::System` and the OS has one (`UiFonts::interface_font`). `no font stack installed` means the build embeds faces (`faces_embedded`) and `fonts` installed none."
                .to_owned(),
        );
    }
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
