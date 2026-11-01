#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`ResetDot`]: a dot beside a dialog field that shows the field differs from its default and
//! resets it on a click.

use egui::{AsIdSalt, IdSalt, Response, Ui};

crate::words! {
    /// The dot's words, English by default (VectorCraft's).
    #[non_exhaustive]
    pub struct ResetDotLabels {
        /// The dot's accessible name.
        reset: "resetDot.reset" = "Reset to default";
        /// Its tooltip: `{default}` the default value as the field shows it.
        reset_to: "resetDot.resetTo" = "Reset to default: {default}";
    }
}

/// A reset dot, after VectorCraft's 3D and Revolve dialogs.
///
/// ```ignore
/// ui.horizontal(|ui| {
///     Select::new("cap", "Cap", &mut cap).option(..).show(ui);
///     if ResetDot::new("cap", cap != Cap::On, "On").show(ui).clicked() { cap = Cap::On }
/// });
/// ```
///
/// - **Look.** A 24-point square (`reset_dot.size`); while the field differs from its default,
///   a filled dot of radius 3 in `accent.solid` (`text.disabled` in a disabled `Ui`), else a
///   1-point ring of radius 3 in `text.disabled`. Hover fills the square with `state.hover` at
///   3-point corners.
/// - **Use.** A click (or Space and Enter with the focus) on a changed dot reports
///   `clicked()`; the host puts the default back. A dot at its default does nothing.
/// - **Keys.** A changed dot is a Tab stop after its field; a dot at its default isn't.
/// - **AccessKit.** A `Button` named [`ResetDotLabels::reset`], disabled at the default; its
///   tooltip and description [`ResetDotLabels::reset_to`].
pub struct ResetDot {
    id_salt: IdSalt,
    changed: bool,
    default: String,
    labels: Option<ResetDotLabels>,
}

impl ResetDot {
    /// A dot for a field that is `changed` from its default, which the tooltip names as
    /// `default` (as the field shows it, translated).
    pub fn new(id_salt: impl AsIdSalt, changed: bool, default: impl Into<String>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), changed, default: default.into(), labels: None }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: ResetDotLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // vectorcraft: crates/ui-egui/src/dialogs/revolve.rs:573 reset_dot, :564 dot
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
