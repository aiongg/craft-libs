//! The Layers panel for Craft apps, over the app's own layer tree.
//!
//! The app implements `ui::tree::LayerTree`, a read-only view of its layers, and shows it with
//! `ui::panel::LayersPanel`, which reports what the user did as events in the order they
//! happened. The app applies them with its own commands and undo.
#![warn(missing_docs)]

// The derives' output names the UI library as `craft_ui`: here craft-ui-panels, which re-exports the contract's and
// the widgets' modules beside its own.
extern crate craft_ui_panels as craft_ui;

pub mod ui;

/// The stories and trials this crate registers, and where a host test finds the parts of a layer row that have no
/// access node of their own: a test finds the row by the layer's name and passes the row's node.
#[cfg(feature = "screen-testing")]
pub mod testing {
    use crate::ui::tree::{LayerInfo, LayerPart};
    use craft_ui::contract::Id;
    use craft_ui::list::{ThumbTarget, TrailingPart};
    use craft_ui::testing::Harness;
    use craft_ui_rt::{Rect, Renderer};

    #[cfg(feature = "suite")]

    craft_ui::registry!();

    /// The rect of the thumbnail that shows `part` of the layer `info`, in the row `row`, as the row's kept hit
    /// shapes give it; `None` when the row shows no such thumbnail. `row` is the layer's row, or for
    /// [`LayerPart::FilterMask`] the layer's Smart Filters row, whose thumbnail the filter mask is.
    pub fn thumb_rect<S, R: Renderer>(harness: &Harness<S, R>, row: Id, info: &LayerInfo, part: LayerPart) -> Option<Rect> {
        let thumb = if part == LayerPart::FilterMask { ThumbTarget::First } else { info.thumb(part)? };
        harness.part_rect(row, thumb.part())
    }

    /// The rect of the `fx` badge of the layer row `row`, as the row's kept hit shapes give it: the badge is a drag
    /// handle, named in the row's description; `None` when the layer has no effects.
    pub fn fx_badge_rect<S, R: Renderer>(harness: &Harness<S, R>, row: Id) -> Option<Rect> {
        harness.part_rect(row, TrailingPart::new(crate::ui::panel::FX_BADGE).part())
    }
}
