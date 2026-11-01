//! [`ResetDot`]: a dot beside a dialog field that shows the field differs from its default and
//! resets it on a click.

use egui::accesskit::Role;
use egui::{AsIdSalt, CornerRadius, IdSalt, Response, Sense, Ui, vec2};

use super::TooltipExt as _;
use crate::localized::Labels as _;
use crate::primitives::Interactive;
use crate::theme::{ToEgui, Tokens, TokensOf as _};

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
        let tokens = Tokens::of(ui);
        let labels = self.labels.unwrap_or_else(|| ResetDotLabels::of(ui));
        let enabled = self.changed && ui.is_enabled();
        let response = Interactive::salted(self.id_salt, Role::Button, labels.reset.as_str())
            .sense(Sense::click())
            .enabled(enabled)
            .state_layer(false)
            .corner_radius(CornerRadius::same(CORNER))
            .show(ui, vec2(SIZE, SIZE));
        if ui.is_rect_visible(response.rect) {
            let center = response.rect.center();
            let disabled = tokens.color.text.disabled.to_egui();
            if enabled && response.hovered() {
                ui.painter().rect_filled(response.rect, CornerRadius::same(CORNER), tokens.color.state.hover.to_egui());
            }
            if self.changed {
                let fill = if ui.is_enabled() { tokens.color.accent.solid.to_egui() } else { disabled };
                ui.painter().circle_filled(center, RADIUS, fill);
            } else {
                ui.painter().circle_stroke(center, RADIUS, egui::Stroke::new(1.0, disabled));
            }
        }
        if self.changed { response.tooltip(labels.reset_to.replace("{default}", &self.default)) } else { response }
    }
}

/// The dot's square (`reset_dot.size` once the tokens land), its dot and its hover corners.
const SIZE: f32 = 24.0;
const RADIUS: f32 = 3.0;
const CORNER: u8 = 3;
