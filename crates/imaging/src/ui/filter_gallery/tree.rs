//! [`FilterTree`]: the middle column's category folders with thumbnails (task WS-FG2).

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{AsIdSalt, Id, IdSalt, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use craft_theme::{Stroke as Line, TypeRole};
use craft_ui::behavior::GridNav;
use craft_ui::canvas::textures::{Priority, ThumbFit, ThumbPaint, ThumbRequest, ThumbSource, Thumbnails};
use craft_ui::composites::SectionHeader;
use craft_ui::primitives::{Content, Corner, Gap, Interactive, Text};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};

use super::FilterGalleryLabels;
use super::catalog::{FilterCategory, GalleryThumb};

/// Thumbnails in a row of a folder.
const COLUMNS: usize = 3;

/// What [`FilterTree::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterTreeResponse {
    pub response: Response,
    /// The filter whose thumbnail was clicked (or picked with Enter or Space).
    pub picked: Option<String>,
}

/// The middle column: one collapsible folder per category (its open state in egui memory; the
/// folder of a newly selected filter opens), each a grid of thumbnails, three a row, at the size
/// `filter_gallery.thumb_width` × `.thumb_height`, with the filter's name under it in the caption
/// role; the selected filter's thumbnail has a 2 pt accent ring (`Stroke::W2`), the others a
/// 1 pt `line.divider` outline. Thumbnails come from a `Thumbnails<GalleryThumb>` cache over the
/// app's `ThumbSource` (the placeholder while loading).
///
/// Keyboard: each folder header is a Tab stop; the thumbnails of an open folder are one grid
/// (`GridNav::roving_stop`: arrows move, Enter or Space picks) with one Tab stop. AccessKit: a
/// folder header is a disclosure triangle named by the category, a thumbnail a button named by
/// its filter, `selected` when current; the folders sit in a group named by the column's word.
#[must_use = "show the tree with `show`"]
pub struct FilterTree<'a> {
    id_salt: IdSalt,
    categories: &'a [FilterCategory],
    labels_of: &'a dyn Fn(&str) -> String,
    selected: &'a str,
    thumbs: &'a Thumbnails<GalleryThumb>,
    source: &'a dyn ThumbSource<GalleryThumb>,
    labels: &'a FilterGalleryLabels,
}

impl<'a> FilterTree<'a> {
    /// The folders of `categories`; `labels_of` gives a filter key's translated name and `source`
    /// renders the pictures `thumbs` asks for.
    pub fn new(
        id_salt: impl AsIdSalt,
        categories: &'a [FilterCategory],
        labels_of: &'a dyn Fn(&str) -> String,
        selected: &'a str,
        thumbs: &'a Thumbnails<GalleryThumb>,
        source: &'a dyn ThumbSource<GalleryThumb>,
        labels: &'a FilterGalleryLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), categories, labels_of, selected, thumbs, source, labels }
    }

    pub fn show(self, ui: &mut Ui) -> FilterTreeResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:702-754 (folders, thumbnails 80 × 56, three
        // a row, names under them), :616 `update_thumbs`.
        let Self { id_salt, categories, labels_of, selected, thumbs, source, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let mut picked = None;
        let group = UiBuilder::new().id_salt(id).accessibility_role(Role::Group).accessibility_label(labels.filters.as_str());
        let scope = ui.scope_builder(group, |ui| {
            // The folder of a filter that has just become the selected one opens.
            let seen_id = id.with("selected");
            let seen: Option<String> = ui.data(|d| d.get_temp(seen_id));
            let newly_selected = seen.as_deref() != Some(selected);
            if newly_selected {
                ui.data_mut(|d| d.insert_temp(seen_id, selected.to_owned()));
            }
            for category in categories {
                let open_id = id.with(("open", category.key.as_str()));
                let mut open = ui.data_mut(|d| d.get_persisted::<bool>(open_id));
                if newly_selected && category.filters.iter().any(|key| key == selected) {
                    open = Some(true);
                }
                let mut open = open.unwrap_or(false);
                SectionHeader::new(open_id, category.label.as_str()).open(&mut open).show(ui, |ui| {
                    let folder = Folder { id: id.with(("folder", category.key.as_str())), category, labels_of, selected, thumbs, source };
                    if let Some(key) = folder.show(ui) {
                        picked = Some(key);
                    }
                });
                ui.data_mut(|d| d.insert_persisted(open_id, open));
            }
        });
        FilterTreeResponse { response: scope.response, picked }
    }
}

/// One open folder: its thumbnails as a grid of rows of [`COLUMNS`].
struct Folder<'a> {
    id: Id,
    category: &'a FilterCategory,
    labels_of: &'a dyn Fn(&str) -> String,
    selected: &'a str,
    thumbs: &'a Thumbnails<GalleryThumb>,
    source: &'a dyn ThumbSource<GalleryThumb>,
}

impl Folder<'_> {
    /// Draws the thumbnails; returns the filter picked.
    fn show(&self, ui: &mut Ui) -> Option<String> {
        let tokens = Tokens::of(ui);
        let sizes = tokens.component.filter_gallery;
        let gap = Gap::Row.points(&tokens);
        let ppp = ui.pixels_per_point();
        let name_format = Text::format(&tokens, TypeRole::Caption, Content::Normal);
        let name_height = ui.painter().layout_no_wrap("Ag".to_owned(), name_format.font_id.clone(), name_format.color).size().y;
        let cell = vec2(sizes.thumb_width, sizes.thumb_height + gap / 2.0 + name_height);
        let keys = &self.category.filters;
        let rows = keys.len().div_ceil(COLUMNS);
        let width = COLUMNS as f32 * cell.x + (COLUMNS - 1) as f32 * gap;
        let (_, area) = ui.allocate_space(vec2(width, rows as f32 * cell.y + rows.saturating_sub(1) as f32 * gap));
        let ids: Vec<Id> = keys.iter().map(|key| Interactive::id_in(ui, (self.id, key.as_str()))).collect();
        let current = keys.iter().position(|key| key == self.selected);
        let nav = GridNav::wrapped(keys.len(), COLUMNS).rtl(craft_ui::direction::is_rtl(ui));
        let stop = GridNav::roving_move_in(ui, &ids, &nav, current, true).0;
        let request = ThumbRequest::for_cell(sizes.thumb_width, ppp);
        let outline = Stroke::new(Line::W1.width(ppp), tokens.color.line.divider.to_egui());
        let ring = Stroke::new(Line::W2.width(ppp), Content::Accent.color(&tokens).to_egui());
        let mut picked = None;
        for (n, key) in keys.iter().enumerate() {
            let (row, column) = (n / COLUMNS, n % COLUMNS);
            let origin = area.min + vec2(column as f32 * (cell.x + gap), row as f32 * (cell.y + gap));
            let cell_rect = Rect::from_min_size(origin, cell);
            let thumb = Rect::from_min_size(origin, vec2(sizes.thumb_width, sizes.thumb_height));
            let on = *key == self.selected;
            let name = (self.labels_of)(key);
            let mut element =
                Interactive::with_id(ids[n], Role::Button, name.as_str()).corner(Corner::Control).selected(on).position_in_set(n).state_layer(false);
            if n != stop {
                element = element.sense(Sense::CLICK);
            }
            let response = element.interact_at(ui, cell_rect).finish(ui);
            if ui.is_rect_visible(cell_rect) {
                let painter = ui.painter();
                let state = self.thumbs.state(ui.ctx(), self.source, &GalleryThumb { filter: key.clone() }, &request, Priority::VISIBLE);
                self.thumbs.paint(painter, thumb, &state, &ThumbPaint::default().fit(ThumbFit::Cover).checker(false));
                painter.rect_stroke(thumb, 0.0, if on { ring } else { outline }, StrokeKind::Outside);
                let mut job = LayoutJob::single_section(name.clone(), name_format.clone());
                job.wrap = TextWrapping::truncate_at_width(cell.x);
                let galley = painter.layout_job(job);
                let color = if on { Content::Normal } else { Content::Muted }.color(&tokens).to_egui();
                painter.galley(pos2(origin.x, thumb.bottom() + gap / 2.0), galley, color);
            }
            if response.gained_focus() {
                ui.scroll_to_rect(cell_rect, None);
            }
            if response.clicked() {
                picked = Some(key.clone());
            }
        }
        picked
    }
}
