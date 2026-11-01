//! [`EffectParameters`]: the selected effect layer's filter select and parameters (task WS-FG2).

use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Ui, UiBuilder};

use craft_theme::TypeRole;
use craft_ui::controls::Select;
use craft_ui::dialog::{ParamRows, ParamRowsResponse};
use craft_ui::primitives::{Content, Gap, Text};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::FilterGalleryLabels;
use super::catalog::{FilterCategory, GalleryFilter};
use super::session::{EffectChange, EffectLayer};

/// What [`EffectParameters::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EffectParametersResponse {
    /// The rows' response (`changed`, a colour or path request).
    pub rows: Option<ParamRowsResponse>,
    /// A filter picked in the select, or the fields changed.
    pub change: Option<EffectChange>,
    pub dragging: bool,
}

/// The right column's parameters: the Filter select (every filter, grouped by category) and the
/// selected effect layer's parameters through `ParamRows` (lead call 38). A colour row asks the
/// app through `ParamRowsResponse::color`. A filter that uses the foreground and background
/// colours says so under its rows.
///
/// AccessKit: the select is a combo box named "Filter"; the rows are the form's.
#[must_use = "show the parameters with `show`"]
pub struct EffectParameters<'a> {
    id_salt: IdSalt,
    index: usize,
    effect: &'a EffectLayer,
    filter: Option<&'a GalleryFilter>,
    categories: &'a [FilterCategory],
    labels_of: &'a dyn Fn(&str) -> String,
    labels: &'a FilterGalleryLabels,
}

impl<'a> EffectParameters<'a> {
    /// The settings of `effect` (at `index` in the stack), whose filter is `filter` (`None`: a
    /// filter the app no longer has, shown with no rows). `labels_of` gives a filter key's
    /// translated name for the select.
    pub fn new(
        id_salt: impl AsIdSalt,
        index: usize,
        effect: &'a EffectLayer,
        filter: Option<&'a GalleryFilter>,
        categories: &'a [FilterCategory],
        labels_of: &'a dyn Fn(&str) -> String,
        labels: &'a FilterGalleryLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), index, effect, filter, categories, labels_of, labels }
    }

    pub fn show(self, ui: &mut Ui) -> EffectParametersResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:774-831 (the select, the parameter rows).
        let Self { id_salt, index, effect, filter, categories, labels_of, labels } = self;
        let id = ui.make_persistent_id(id_salt).with(index);
        let gap = Gap::Row.points(&Tokens::of(ui));
        let mut change = None;

        let mut key = effect.filter.clone();
        let mut select = Select::new(id.with("filter"), labels.filter.as_str(), &mut key).width(ui.available_width());
        for category in categories {
            select = select.group(category.label.as_str());
            for filter_key in &category.filters {
                select = select.option(filter_key.clone(), labels_of(filter_key));
            }
        }
        let edit = select.show(ui);
        if let Some(picked) = edit.committed().filter(|picked| **picked != effect.filter) {
            change = Some(EffectChange::SetFilter { index, filter: picked.clone() });
        }
        ui.add_space(gap);

        let mut rows = None;
        if let Some(filter) = filter {
            let mut fields = effect.fields.clone();
            let group = UiBuilder::new().id_salt(id.with("rows")).accessibility_role(Role::Group).accessibility_label(labels.settings.as_str());
            let shown = ui.scope_builder(group, |ui| ParamRows::new(id.with("params"), &filter.params).show(ui, &mut fields)).inner;
            if shown.changed && change.is_none() {
                change = Some(EffectChange::Fields { index, fields });
            }
            rows = Some(shown);
            if filter.uses_colors {
                ui.add_space(gap);
                ui.add(Text::new(labels.uses_colors.as_str()).role(TypeRole::Caption).content(Content::Faint).max_width(ui.available_width()));
            }
        }
        EffectParametersResponse { rows, change, dragging: ui.input(|i| i.pointer.any_down()) }
    }
}
