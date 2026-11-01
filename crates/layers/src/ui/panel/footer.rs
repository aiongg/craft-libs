//! The footer: `LayersFooterButton`, `LayersFooterAction`, press menus and footer drops.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::ListDrag;
use egui::Ui;

use super::context::PanelContext;
use super::events::LayersEvent;
use crate::ui::tree::{LayerRowKey, LayerTree};

/// A footer button that takes drops of rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayersFooterButton {
    /// Delete: layers, effects, Smart Filters and mask thumbnails.
    Delete,
    /// Create a new layer: duplicates the dropped layers.
    NewLayer,
    /// Create a new group: groups the dropped layers.
    NewGroup,
}

/// A click on a footer button, with what its modifiers ask (PhotoCraft, Photoshop).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayersFooterAction {
    /// Link layers (enabled by [`LayersFooterState::link_enabled`](crate::ui::tree::LayersFooterState::link_enabled)).
    Link,
    /// Create a new layer above the active one; `ask` (Alt) opens the New Layer dialog first;
    /// `below` (Cmd/Ctrl) puts it below.
    NewLayer {
        /// Alt held: ask first.
        ask: bool,
        /// Cmd/Ctrl held: below the active layer.
        below: bool,
    },
    /// Create a new group; `from_selection` when Shift is held or two or more layers are
    /// selected (Group Layers); `ask` (Alt) opens the New Group dialog; `below` (Cmd/Ctrl) adds an
    /// empty group below.
    NewGroup {
        /// Group the selected layers.
        from_selection: bool,
        /// Alt held: ask first.
        ask: bool,
        /// Cmd/Ctrl held: an empty group below.
        below: bool,
    },
    /// Delete what the button names (the app's `delete_label`); `confirm` is false with Alt
    /// (Photoshop deletes without asking).
    Delete {
        /// Ask before deleting.
        confirm: bool,
    },
    /// Add a mask (the button never replaces one); `invert` (Alt): Hide All or Hide Selection.
    AddMask {
        /// Alt held.
        invert: bool,
    },
    /// A pick in the fill or adjustment menu: the app's command.
    Adjustment(String),
    /// A pick in the layer style menu: the app's command.
    Style(String),
}

/// Draws the footer with [`PanelFooter`](craft_ui::composites::PanelFooter) and returns its
/// events in order.
///
/// - **Buttons**, from the reading start: Link, Layer Style (`fx`, a menu button), Mask, Fill or
///   Adjustment (a menu button), New Group, New Layer, Delete (`PanelFooter` puts its first
///   button at the reading end, so they are added Delete first). Tooltips and names: the
///   `LayersLabels` footer words, the app's `mask_label` and `delete_label`; Link greys without
///   `link_enabled`, Mask without a `mask_label`, Delete without `delete_enabled`.
/// - **Clicks** report [`LayersEvent::Footer`] with the modifiers read at the click: New Layer
///   `ask` = Alt, `below` = Cmd/Ctrl; New Group `from_selection` = Shift or two or more selected
///   layers (and an active layer), `ask` = Alt, `below` = Cmd/Ctrl; Delete `confirm` = !Alt;
///   Mask `invert` = Alt.
/// - **Menus** open on the press ([`PressMenu`](craft_ui::menu::PressMenu)) with the app's
///   `adjustment_menu` and `style_menu`; a release over an item picks it: `Adjustment(command)`,
///   `Style(command)`.
/// - **Drops**: right after Delete, New Layer and New Group,
///   `PanelFooter::drop_target::<ListDrag<LayerRowKey<Id>>>` with [`accepts_footer_drop`];
///   a drop reports [`LayersEvent::FooterDrop`] with the payload's items. The list's drag of
///   the same rows ends as `Cancel(Dismissed)` in the same pass.
/// - Keyboard: each button is a Tab stop; the menu buttons open with Space and Enter.
pub(super) fn show_footer<T: LayerTree>(ui: &mut Ui, cx: &PanelContext<'_, T>) -> Vec<LayersEvent<T::Id>> {
    // pc: crates/ui-egui/src/panels/layers.rs:371-503 (buttons, modifiers, menus, drops);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:40-48 `dropped_on`.
    todo!()
}

/// Whether `button` takes a drop of `drag`: Delete takes every row kind and a mask; New Layer
/// and New Group take layer rows only (no effects, filters or masks).
pub(super) fn accepts_footer_drop<Id>(button: LayersFooterButton, drag: &ListDrag<LayerRowKey<Id>>) -> bool {
    // pc: crates/ui-egui/src/panels/layers.rs:345-357 (which drags the footer reads);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:32-62 (`footer_drop`, `fx_trash_drop`).
    todo!()
}
