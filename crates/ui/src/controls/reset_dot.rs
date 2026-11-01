//! [`ResetDot`]: a dot beside a dialog field that shows the field differs from its default and
//! resets it on a click.

use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Response, Sense, Ui, vec2};

use super::TooltipExt as _;
use crate::localized::Labels as _;
use crate::primitives::Interactive;
use crate::theme::{ToEgui, Tokens, TokensOf as _, corner_radius};

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
/// - **Look.** A square of `reset_dot.size`; while the field differs from its default, a filled
///   dot of radius `reset_dot.dot` in `accent.solid` (`text.disabled` in a disabled `Ui`), else a
///   1-point ring of that radius in `text.disabled`. The state layer fills the square (hover and
///   press, under the focus ring) with `reset_dot.radius` corners.
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
        let tokens = Tokens::of(ui);
        let labels = self.labels.unwrap_or_else(|| ResetDotLabels::of(ui));
        let dot = &tokens.component.reset_dot;
        let corner = corner_radius(dot.radius);
        let enabled = self.changed && ui.is_enabled();
        let response = Interactive::salted(self.id_salt, Role::Button, labels.reset.as_str())
            .sense(Sense::click())
            .enabled(enabled)
            .corner_radius(corner)
            .show(ui, vec2(dot.size, dot.size));
        if ui.is_rect_visible(response.rect) {
            let center = response.rect.center();
            let disabled = tokens.color.text.disabled.to_egui();
            if self.changed {
                let fill = if ui.is_enabled() { tokens.color.accent.solid.to_egui() } else { disabled };
                ui.painter().circle_filled(center, dot.dot, fill);
            } else {
                ui.painter().circle_stroke(center, dot.dot, egui::Stroke::new(1.0, disabled));
            }
        }
        if self.changed { response.tooltip(craft_i18n::fmt(&labels.reset_to, &[("default", &self.default)])) } else { response }
    }
}
