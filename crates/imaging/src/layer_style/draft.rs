//! The Layer Style dialog's draft: the style being edited plus the dialog's own view state, and
//! the rules every edit follows.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-7)")]

use super::model::{EffectId, EffectKind, LayerStyle, LayerStylePage, StyleParams, StyleValue};
use super::shadow::ShadowDrag;

/// What the Layer Style dialog edits: the [`LayerStyle`] and the dialog's view state (the page,
/// the Blend If channel shown, the Preview check, a shadow drag in progress).
///
/// The app owns the draft for as long as the dialog is open. The dialog reports a changed copy
/// (`EditEvent<LayerStyleDraft>`); the app keeps it, previews it while
/// [`preview`](Self::preview) is on, applies [`style`](Self::style) as one history step on OK and
/// drops it on Cancel. The view state never reaches the document, and
/// [`render_hash`](Self::render_hash) ignores it, so a page change re-renders nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerStyleDraft {
    pub style: LayerStyle,
    /// The page shown.
    pub page: LayerStylePage,
    /// The Blend If entry shown on the Blending Options page, an index into
    /// `style.blending.blend_if`.
    pub blend_if_channel: usize,
    /// The Preview check box: whether the host shows the draft on the canvas. On by default.
    pub preview: bool,
    /// The shadow drag in progress, from its press to its release.
    pub shadow_drag: Option<ShadowDrag>,
}

impl Default for LayerStyleDraft {
    fn default() -> Self {
        LayerStyleDraft::new(LayerStyle::default())
    }
}

impl LayerStyleDraft {
    /// A draft of `style` showing its first instance's page, or Blending Options when it has
    /// none, with Preview on.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:529-571 `initial_fields` (no page asked: the
    // first instance).
    pub fn new(style: LayerStyle) -> LayerStyleDraft {
        let page = style.effects.first().map_or(LayerStylePage::Blending, |e| LayerStylePage::Effect(e.id));
        LayerStyleDraft { style, page, blend_if_channel: 0, preview: true, shadow_drag: None }
    }

    /// Opens on `kind`'s page (Layer › Layer Style › Drop Shadow…): its first instance, turned on,
    /// or a new instance when the layer has none.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:549-567 (`select` names a kind).
    pub fn open_on(&mut self, kind: EffectKind) {
        todo!()
    }

    /// The selected instance's kind; `None` on Styles and Blending Options.
    pub fn selected_kind(&self) -> Option<EffectKind> {
        match self.page {
            LayerStylePage::Effect(id) => self.style.effect(id).map(|e| e.kind),
            _ => None,
        }
    }

    /// Adds an instance of `kind`, turned on, with `defaults` (the kind's spec defaults) and,
    /// when its `useGlobalLight` is on, `angle` set to the shared light angle. The id is one past
    /// the highest `fxN` in the style (the lowest free number if that overflows). It goes right
    /// after the last instance of its kind (at the end when there is none), so the render order
    /// keeps the layer's effect order, and its page is selected.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:449-460 `next_id`, :463-473 `add_instance`,
    // :82-88 `fresh_params`.
    pub fn add_instance(&mut self, kind: EffectKind, defaults: &StyleParams) -> EffectId {
        todo!()
    }

    /// Removes instance `id`. When its page was selected, the selection moves to the first
    /// remaining instance of the same kind, else to the Styles page.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:476-491 `remove_instance` (falls back to
    // `STYLES_PAGE`).
    pub fn remove_instance(&mut self, id: EffectId) {
        todo!()
    }

    /// The check box of instance `id`: turns it on or off and selects its page (selecting a page
    /// alone never turns an effect on, #2288).
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:927-933.
    pub fn toggle(&mut self, id: EffectId) {
        todo!()
    }

    /// Sets parameter `key` of instance `id`. The shared light angle: when `key` is `angle` and
    /// the instance's `useGlobalLight` is on, or `key` is `useGlobalLight` turned on, the angle
    /// (the new one, or the shared one) becomes `style.global_light` and every instance with
    /// `useGlobalLight` on, disabled ones included, takes it as its `angle`.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:415-446 `set_param`.
    pub fn set_param(&mut self, id: EffectId, key: &str, value: StyleValue) {
        todo!()
    }

    /// Replaces the instances with a style preset's (`(kind, on, params)` in order), numbered
    /// `fx1`, `fx2`, … The app sets the preset's blend mode and opacities on
    /// `style.blending`; the light angle stays and the page stays on Styles, where presets
    /// are picked.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:752-775 `apply_style_preset`.
    pub fn apply_preset(&mut self, effects: Vec<(EffectKind, bool, StyleParams)>) {
        todo!()
    }

    /// Whether a drag on the image moves the selected effect now: a Drop Shadow or Inner Shadow
    /// page is selected.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:498-501 `drags_shadow`.
    pub fn drags_shadow(&self) -> bool {
        self.selected_kind().is_some_and(EffectKind::drags_shadow)
    }

    /// A drag on the image with a shadow page selected. `start` is the press (it records the
    /// shadow's angle, 120 when missing, and distance, 0 when missing); `total` is the pointer's
    /// movement since, in document pixels. The new angle and distance ([`ShadowDrag::moved`]) go
    /// through [`set_param`](Self::set_param), so with Use Global Light the shared angle moves
    /// too. Returns whether the style changed; does nothing on other pages. The host calls it on
    /// every move (an `Update` of its outer gesture) and clears
    /// [`shadow_drag`](Self::shadow_drag) on release.
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:508-527 `drag_shadow`.
    pub fn drag_shadow(&mut self, start: bool, total: [f64; 2]) -> bool {
        todo!()
    }

    /// The style's [`render_hash`](LayerStyle::render_hash); the page, the Blend If channel,
    /// the Preview check and the drag don't count.
    pub fn render_hash(&self) -> u64 {
        self.style.render_hash()
    }
}
