//! [`EffectSettings`]: the selected effect layer's filter select and parameters (task WS-FG2).

use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::dialog::ParamRowsResponse;

use super::FilterGalleryLabels;
use super::catalog::{FilterCategory, GalleryFilter};
use super::session::{EffectChange, EffectLayer};

/// What [`EffectSettings::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EffectSettingsResponse {
    /// The rows' response (`changed`, a colour or path request).
    pub rows: Option<ParamRowsResponse>,
    /// A filter picked in the select, or the fields changed.
    pub change: Option<EffectChange>,
    pub dragging: bool,
}

/// The right column's settings: the Filter select (every filter, grouped by category) and the
/// selected effect layer's parameters through `ParamRows` (lead call 38). A colour row asks the
/// app through `ParamRowsResponse::color`.
#[expect(dead_code, reason = "skeleton: `show` (task WS-FG2) reads the fields")]
pub struct EffectSettings<'a> {
    id_salt: IdSalt,
    index: usize,
    effect: &'a EffectLayer,
    filter: Option<&'a GalleryFilter>,
    categories: &'a [FilterCategory],
    labels: &'a FilterGalleryLabels,
}

impl<'a> EffectSettings<'a> {
    /// The settings of `effect` (at `index` in the stack), whose filter is `filter` (`None`: a
    /// filter the app no longer has, shown with no rows).
    pub fn new(
        id_salt: impl AsIdSalt,
        index: usize,
        effect: &'a EffectLayer,
        filter: Option<&'a GalleryFilter>,
        categories: &'a [FilterCategory],
        labels: &'a FilterGalleryLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), index, effect, filter, categories, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn show(self, ui: &mut Ui) -> EffectSettingsResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:774-831 (the select, the parameter rows).
        todo!()
    }
}
