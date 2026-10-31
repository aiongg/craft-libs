//! [`FontPicker`] (F14.01, gap 17): the font family field of the type tool, the Character panel,
//! the style panels and the Glyphs panel. A text field that filters the installed families as the
//! user types, with a list under it of family rows drawn in their own face, favourites (stars),
//! recent families at the top, class and feature filters above the list and a "similar fonts"
//! filter. Behaviour follows Photoshop's font menu; PhotoCraft's `family_picker_in` is the source.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError, Weak};

use craft_font_catalog::{Catalog, FaceInfo, LocalizedName};
use craft_host::{Host, Section};
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::controls::{Button, IconButton, TextField};
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ToggleColumn};
use craft_ui::localized::Labels as _;
use craft_ui::popover::{CloseReason, Popover, PopoverAnchor, PopoverClose};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::epaint::text::FontId;
use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use egui::{Align2, Context, Event, Id, Key, Modifiers, Rect, Response, Sense, Ui, pos2, vec2};
use serde::{Deserialize, Serialize};
use skrifa::MetadataProvider as _;
use skrifa::raw::{FontRef, TableProvider};

use crate::panel::faces::{FaceFonts, Owner, Shown};

craft_ui::words! {
    /// The picker's words, English by default.
    #[non_exhaustive]
    pub struct FontPickerLabels {
        search: "fontPicker.search" = "Search fonts";
        recent: "fontPicker.recent" = "Recent";
        favorites: "fontPicker.favorites" = "Show favorite fonts";
        similar: "fontPicker.similar" = "Show similar fonts";
        variable: "fontPicker.variable" = "Show variable fonts";
        classes: "fontPicker.classes" = "Filter by class";
        all_classes: "fontPicker.allClasses" = "All Classes";
        clear_all: "fontPicker.clearAll" = "Clear All";
        sample: "fontPicker.sample" = "Sample";
        no_match: "fontPicker.noMatch" = "No fonts match";
        star: "fontPicker.star" = "Add to favorites";
        unstar: "fontPicker.unstar" = "Remove from favorites";
        favorite: "fontPicker.favorite" = "Favorite {row}";
        list: "fontPicker.list" = "Fonts";
        hide_classes: "fontPicker.hideClasses" = "Hide Font Classes";
        show_classes: "fontPicker.showClasses" = "Show Font Classes";
        no_similar: "fontPicker.noSimilar" = "No similar fonts: the current font doesn't say what it looks like.";
        no_favorites: "fontPicker.noFavorites" = "No favorite fonts yet: click the star beside a font to add it.";
        no_match_filters: "fontPicker.noMatchFilters" = "No fonts match these filters.";
        serif: "fontPicker.class.serif" = "Serif";
        slab_serif: "fontPicker.class.slabSerif" = "Slab Serif";
        sans_serif: "fontPicker.class.sansSerif" = "Sans Serif";
        script: "fontPicker.class.script" = "Script";
        monospace: "fontPicker.class.monospace" = "Monospace";
        handwritten: "fontPicker.class.handwritten" = "Handwritten";
        blackletter: "fontPicker.class.blackletter" = "Blackletter";
        decorative: "fontPicker.class.decorative" = "Decorative";
    }
}

/// Photoshop's font classes, in its filter menu order (the class glyph at the row's right).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontClass {
    Serif,
    SlabSerif,
    SansSerif,
    Script,
    Monospace,
    Handwritten,
    Blackletter,
    Decorative,
}

impl FontClass {
    /// Every class, in menu order.
    pub const ALL: [FontClass; 8] = [
        FontClass::Serif,
        FontClass::SlabSerif,
        FontClass::SansSerif,
        FontClass::Script,
        FontClass::Monospace,
        FontClass::Handwritten,
        FontClass::Blackletter,
        FontClass::Decorative,
    ];

    /// The class of a family from its PANOSE and OS/2 data, else from its name; `None` when the
    /// face can't be read.
    // pc: crates/ui-egui/src/text/font_filters.rs:44 `traits` and crates/text/src/font_class.rs.
    pub fn of(catalog: &Catalog, family: &str) -> Option<FontClass> {
        let faces = catalog.styles(family);
        let face = regular_face(&faces)?;
        read_traits(catalog, face)?.class
    }
}

/// Photoshop's Font Preview Size (Type › Font Preview Size): how large the sample in each row is,
/// or none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontPreviewSize {
    None,
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
    Huge,
}

impl FontPreviewSize {
    /// The sample's point size, `None` for no sample.
    // pc: crates/ui-egui/src/text/font_preview.rs:40 `sample`.
    pub fn points(self) -> Option<f32> {
        match self {
            FontPreviewSize::None => None,
            FontPreviewSize::Small => Some(14.0),
            FontPreviewSize::Medium => Some(18.0),
            FontPreviewSize::Large => Some(24.0),
            FontPreviewSize::ExtraLarge => Some(32.0),
            FontPreviewSize::Huge => Some(40.0),
        }
    }
}

/// What narrows the list. Every filter off shows every installed family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontFilters {
    /// Only families of this class (`None`: All Classes).
    pub class: Option<FontClass>,
    /// Only starred families.
    pub favorites: bool,
    /// Only variable fonts.
    pub variable: bool,
    /// Families like the current one, most alike first ([`SIMILAR_COUNT`] of them).
    pub similar: bool,
}

impl FontFilters {
    /// Whether any filter narrows the list (Clear All is enabled).
    pub fn any(&self) -> bool {
        self.class.is_some() || self.favorites || self.variable || self.similar
    }

    /// Clear All: every filter off.
    pub fn clear(&mut self) {
        *self = FontFilters::default();
    }
}

/// How many families "similar fonts" shows.
pub const SIMILAR_COUNT: usize = 30;

/// The picker's remembered state: starred families and the most recent picks, one settings
/// section shared by every picker of the app (`[font_picker]`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontPickerSettings {
    /// Starred families, in star order.
    pub favorites: Vec<String>,
    /// Most recent first, at most [`FontPickerSettings::recent_limit`].
    pub recent: Vec<String>,
    /// How many recent families the list shows (Photoshop: Number of Recent Fonts to Display, 10).
    pub recent_limit: usize,
    pub preview: FontPreviewSize,
    /// Show family names in English rather than their own language (Type › Show Font Names in English).
    pub names_in_english: bool,
}

impl Default for FontPickerSettings {
    fn default() -> Self {
        FontPickerSettings {
            favorites: Vec::new(),
            recent: Vec::new(),
            recent_limit: 10,
            preview: FontPreviewSize::default(),
            names_in_english: false,
        }
    }
}

impl Section for FontPickerSettings {
    const KEY: &'static str = "font_picker";
}

impl FontPickerSettings {
    /// Put `family` first in the recents, dropping a duplicate and anything past the limit.
    // pc: crates/ui-egui/src/app.rs `note_recent_font`.
    pub fn note_recent(&mut self, family: &str) {
        self.recent.retain(|f| f != family);
        self.recent.insert(0, family.to_owned());
        self.recent.truncate(self.recent_limit);
    }

    /// Star or unstar `family`.
    pub fn set_favorite(&mut self, family: &str, on: bool) {
        let starred = self.favorites.iter().any(|f| f == family);
        if on && !starred {
            self.favorites.push(family.to_owned());
        } else if !on {
            self.favorites.retain(|f| f != family);
        }
    }
}

/// One row of the list, for apps that draw their own list from the picker's filtering
/// ([`FontPicker::rows`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyRow {
    /// The family as the catalog names it.
    pub family: String,
    /// The name shown: the family's name in the UI language unless `names_in_english`.
    pub shown: String,
    pub starred: bool,
    /// The family's class once it has been read: by a class filter, or by the list drawing the row.
    pub class: Option<FontClass>,
    pub variable: bool,
    /// The row is in the Recent group at the top.
    pub recent: bool,
}

/// What the user did this pass.
#[derive(Clone, Debug)]
pub struct FontPickerResponse {
    /// A family was picked: Enter on the typed name, a click or Enter on a row, or the wheel on the
    /// closed field. The caller applies it (and notes it as recent).
    pub picked: Option<String>,
    /// The family under the pointer or the keyboard row while the list is open, for a live preview
    /// of the selected text (Photoshop previews on hover). `None` restores the committed family.
    pub preview: Option<String>,
    /// The field's response, for `changed`, focus and layout.
    pub response: Response,
}

/// The font family picker. Built per pass over the catalog and the app's settings:
///
/// ```ignore
/// let mut settings = host.setting::<FontPickerSettings>();
/// let r = FontPicker::new("type-family", &catalog, &mut family).settings(&mut settings).width(220.0).show(ui);
/// if let Some(picked) = r.picked { settings.note_recent(&picked); host.set_setting(&settings); apply(picked); }
/// ```
///
/// What the user sees and does:
/// - A text field with a chevron. Typing filters the list to families whose name (in any
///   language) contains the text; the list opens on the first keystroke or on the chevron. The
///   current family is scrolled into view when the list opens.
/// - Rows: star, the family name in the UI face, a sample in the family's own face at the
///   preview size (the family name when the face can't draw "Sample"), the class glyph at the
///   right. Recent families form a group at the top, separated by a rule. A row under the pointer
///   or the keyboard row previews on the canvas.
/// - Above the list: the search field is the picker's own field; a row of filter toggles
///   (class list button, star, similar, variable) and Clear All, as Photoshop's font menu. The
///   class list is a collapsible column at the left.
/// - Keys: ↓/↑ move the keyboard row (stopping at the ends), Enter picks it (or the exact typed name), Esc
///   closes and restores the field text, Tab picks and moves on. While the IME composes, Enter,
///   Esc and the arrows belong to the composition; a later Enter picks. On the closed field the
///   wheel steps to the previous or next family in list order and picks it (Photoshop).
/// - Platform: the list is a `Popover` and keeps the stable id `<salt>.popup`; on the web the
///   sample images are rendered from served faces as they arrive.
pub struct FontPicker<'a> {
    salt: &'a str,
    catalog: &'a Arc<Catalog>,
    current: &'a mut String,
    settings: Option<&'a mut FontPickerSettings>,
    filters: Option<&'a mut FontFilters>,
    width: Option<f32>,
    list_height: f32,
    labels: Option<FontPickerLabels>,
    language: Option<String>,
    start_open: bool,
}

/// The default height of the open list, in points (PhotoCraft's `FONT_LIST_HEIGHT`).
pub const LIST_HEIGHT: f32 = 440.0;

impl<'a> FontPicker<'a> {
    /// A picker over `catalog` editing `current` (the family name). `salt` keeps its ids apart from
    /// other pickers in the same `Ui`.
    pub fn new(salt: &'a str, catalog: &'a Arc<Catalog>, current: &'a mut String) -> Self {
        FontPicker {
            salt,
            catalog,
            current,
            settings: None,
            filters: None,
            width: None,
            list_height: LIST_HEIGHT,
            labels: None,
            language: None,
            start_open: false,
        }
    }

    /// Stars, recents and the preview size; without it no stars or recents are shown.
    pub fn settings(mut self, settings: &'a mut FontPickerSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The filters, kept by the caller so they survive the list closing; without it the filter
    /// row is hidden.
    pub fn filters(mut self, filters: &'a mut FontFilters) -> Self {
        self.filters = Some(filters);
        self
    }

    /// The field's width; the list is at least as wide.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Start with the list open when the field has no state yet (the stories' open list).
    pub(crate) fn start_open(mut self, open: bool) -> Self {
        self.start_open = open;
        self
    }

    /// The open list's height.
    pub fn list_height(mut self, height: f32) -> Self {
        self.list_height = height;
        self
    }

    pub fn labels(mut self, labels: FontPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The interface language (`ja`, `pt-BR`) the family names are shown in; by default the
    /// Host's, English without one. A name the family has no record in stays the family name.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// The rows the list would show for `query` and the filters, in list order: recents first
    /// (when the query is empty and no filter is on), then the matching families collated in the
    /// UI language.
    // pc: crates/ui-egui/src/tools/type_tool/fonts.rs:69 `font_menu_rows` and
    // crates/ui-egui/src/text/font_filters.rs:65 `apply` (similar fonts ranked by trait distance).
    pub fn rows(&self, query: &str) -> Vec<FamilyRow> {
        self.list(query, true).0
    }

    /// The rows for `query`, narrowed by the filters and headed by the recents when `narrowed`.
    /// The flag is false when Show Similar Fonts is on and the current family has nothing to
    /// compare (its rows are then left as they were).
    fn list(&self, query: &str, narrowed: bool) -> (Vec<FamilyRow>, bool) {
        let infos = infos(self.catalog);
        let language = self.language.as_deref().unwrap_or("en");
        let settings = self.settings.as_deref();
        let filters = if narrowed { self.filters.as_deref().cloned().unwrap_or_default() } else { FontFilters::default() };
        let english = settings.is_some_and(|s| s.names_in_english);
        let query = query.to_lowercase();
        let mut rows: Vec<FamilyRow> = self
            .catalog
            .families()
            .iter()
            .filter_map(|family| {
                let info = infos.families.get(family)?;
                let shown = shown_name(info, family, language, english);
                let matches = query.is_empty()
                    || family.to_lowercase().contains(&query)
                    || shown.to_lowercase().contains(&query)
                    || info.localized.iter().any(|n| n.name.to_lowercase().contains(&query));
                matches.then(|| FamilyRow {
                    family: family.clone(),
                    starred: settings.is_some_and(|s| s.favorites.iter().any(|f| f == family)),
                    class: infos.known_class(family),
                    variable: info.variable,
                    recent: false,
                    shown,
                })
            })
            .collect();
        rows.sort_by_cached_key(|r| r.shown.to_lowercase());
        if filters.favorites {
            rows.retain(|r| r.starred);
        }
        if filters.variable {
            rows.retain(|r| r.variable);
        }
        if let Some(class) = filters.class {
            rows.retain_mut(|r| {
                r.class = infos.traits(self.catalog, &r.family).and_then(|t| t.class);
                r.class == Some(class)
            });
        }
        let mut comparable = true;
        if filters.similar {
            let reference = infos.traits(self.catalog, self.current).filter(|t| t.class.is_some() || t.panose.iter().any(|d| *d > 1));
            if let Some(reference) = reference {
                let mut scored: Vec<(f32, FamilyRow)> = rows
                    .drain(..)
                    .filter_map(|r| {
                        let traits = infos.traits(self.catalog, &r.family)?;
                        let distance = if r.family == *self.current { -1.0 } else { reference.difference(&traits) };
                        Some((distance, r))
                    })
                    .collect();
                scored.sort_by(|a, b| a.0.total_cmp(&b.0));
                rows = scored.into_iter().take(SIMILAR_COUNT).map(|(_, r)| r).collect();
            } else {
                comparable = false;
            }
        }
        if narrowed
            && query.is_empty()
            && !filters.any()
            && let Some(settings) = settings
        {
            let recent: Vec<FamilyRow> = settings
                .recent
                .iter()
                .take(settings.recent_limit)
                .filter_map(|f| rows.iter().find(|r| r.family == *f))
                .map(|r| FamilyRow { recent: true, ..r.clone() })
                .collect();
            rows.splice(0..0, recent);
        }
        (rows, comparable)
    }

    /// The family `notches` steps from the current one in list order (up, positive, is the
    /// previous), unfiltered and without recents so stepping doesn't depend on past picks.
    fn stepped_family(&self, notches: f32) -> Option<String> {
        if notches == 0.0 {
            return None;
        }
        let all = self.list("", false).0;
        let i = all.iter().position(|r| r.family == *self.current)?;
        let n = wheel_family_index(i, all.len(), notches)?;
        (all[n].family != *self.current).then(|| all[n].family.clone())
    }

    /// Draw the field and, when open, the list.
    // pc: crates/ui-egui/src/tools/type_tool/fonts.rs:388 `family_picker_in` (field, keys, IME
    // guard, popup, wheel) and `:843 wheel_family_index`; rows through `list_row`.
    pub fn show(mut self, ui: &mut Ui) -> FontPickerResponse {
        let ctx = ui.ctx().clone();
        let labels = self.labels.take().unwrap_or_else(|| FontPickerLabels::of(ui));
        let language = self.language.get_or_insert_with(|| ui_language(&ctx)).clone();
        let infos = infos(self.catalog);
        let english = self.settings.as_deref().is_some_and(|s| s.names_in_english);
        let shown_of = |family: &str| infos.families.get(family).map_or_else(|| family.to_owned(), |i| shown_name(i, family, &language, english));

        let field_id = ui.make_persistent_id(("font-picker", self.salt));
        let edit_id = field_id.with("edit");
        let enabled = ui.is_enabled();
        let focused = ui.memory(|m| m.has_focus(field_id));
        let mut edit: Edit =
            ctx.data(|d| d.get_temp::<Edit>(edit_id)).filter(|e| e.current == *self.current && (e.open || focused)).unwrap_or_else(|| Edit {
                text: shown_of(self.current),
                current: self.current.clone(),
                open: self.start_open,
                reveal: if self.start_open { 2 } else { 0 },
                ..Edit::default()
            });
        let was_open = edit.open;
        let live = enabled && (focused || was_open);
        if live {
            ui.input(|i| {
                for event in &i.events {
                    match event {
                        Event::Ime(egui::ImeEvent::Preedit { text, .. }) => edit.composing = !text.is_empty(),
                        Event::Ime(egui::ImeEvent::Commit(_)) => edit.composing = false,
                        _ => {}
                    }
                }
            });
        }
        // The arrows are the picker's before the text edit can move its caret with them; during a
        // composition they and Enter and Esc stay the input method's.
        let consume = |ui: &mut Ui, key| live && !edit.composing && ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        let down = consume(ui, Key::ArrowDown);
        let up = consume(ui, Key::ArrowUp);
        let tab = live && ui.input(|i| i.key_pressed(Key::Tab));

        let mut field = TextField::new(format!("{}.field", self.salt), labels.search.clone(), &edit.text)
            .id(field_id)
            .trailing_arrow(true)
            .keep_focus(true)
            .enabled(enabled);
        if let Some(width) = self.width {
            field = field.width(width);
        }
        let mut field = field.show(ui);
        let rect = field.response.rect;
        let arrow_rect = if craft_ui::direction::is_rtl(ui) {
            Rect::from_min_size(rect.min, vec2(rect.height(), rect.height()))
        } else {
            Rect::from_min_size(pos2(rect.right() - rect.height(), rect.top()), vec2(rect.height(), rect.height()))
        };
        let arrow = ui.interact(arrow_rect, field_id.with("arrow"), Sense::click());
        arrow.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, labels.list.clone()));
        icons::paint(ui, arrow_rect, Icon::ChevronDown, IconSize::Sm, ui.style().interact(&arrow).fg_stroke.color);

        let mut open = was_open;
        if let Some(text) = field.event.as_ref().and_then(EditEvent::value) {
            edit.text.clone_from(text);
            edit.query.clone_from(text);
            edit.row = None;
            open = true;
        }
        if field.response.clicked() && !was_open {
            open = true;
            edit.select = true;
        }
        if arrow.clicked() {
            open = !open;
            ctx.memory_mut(|m| m.request_focus(field_id));
        }
        if open && !was_open {
            edit.reveal = 2;
            edit.moved = false;
        }

        let (rows, comparable) = self.list(&edit.query, true);
        let current_row = rows.iter().position(|r| r.family == *self.current);
        let mut picked: Option<String> = None;
        if open && (down || up) && !rows.is_empty() {
            let last = rows.len() - 1;
            let next = match edit.row.or(current_row) {
                Some(i) if down => (i + 1).min(last),
                Some(i) => i.saturating_sub(1),
                None if down => 0,
                None => last,
            };
            edit.row = Some(next);
            edit.text.clone_from(&rows[next].shown);
            edit.select = true;
            edit.reveal = 1;
        }
        if open && field.submitted {
            picked = accept_row(&rows, &edit.text, edit.row).map(|i| rows[i].family.clone());
        }
        if open && tab && (edit.row.is_some() || !edit.query.is_empty()) {
            picked = accept_row(&rows, &edit.text, edit.row).map(|i| rows[i].family.clone());
        }
        if !open && enabled {
            let mut notches = if focused { f32::from(u8::from(up)) - f32::from(u8::from(down)) } else { 0.0 };
            if field.response.hovered() || arrow.hovered() {
                notches += take_wheel_notch(ui);
            }
            picked = picked.or_else(|| self.stepped_family(notches));
        }

        // The popup.
        let mut filters = self.filters.take();
        let preview_size = self.settings.as_deref().and_then(|s| s.preview.points());
        let favorites_empty = self.settings.as_deref().is_none_or(|s| s.favorites.is_empty());
        let hide_id = field_id.with("hide-classes");
        let hide_classes = ctx.data(|d| d.get_temp::<bool>(hide_id)).unwrap_or(false);
        let faces = face_fonts(&ctx);
        let reveal = (edit.reveal > 0).then(|| edit.row.or(current_row)).flatten();
        let list_width = rect.width().max(LIST_WIDTH);
        let list_height = self.list_height;
        let catalog = self.catalog;
        let popup = Popover::new(format!("{}.popup", self.salt), PopoverAnchor::Control(field.response.clone()))
            .close(PopoverClose::default().focus_left(false))
            .toggle(arrow.id)
            .label(labels.list.clone())
            .show(ui, &mut open, |ui, pop| {
                pop.initial_focus(field_id);
                let mut out = PopupOut::default();
                let mut hide = hide_classes;
                if let Some(f) = filters.as_deref_mut() {
                    filter_bar(ui, f, &mut hide, &labels);
                }
                let any = filters.as_deref().is_some_and(FontFilters::any);
                let similar = filters.as_deref().is_some_and(|f| f.similar);
                let favorites = filters.as_deref().is_some_and(|f| f.favorites);
                ui.horizontal_top(|ui| {
                    if let (Some(f), false) = (filters.as_deref_mut(), hide) {
                        class_column(ui, field_id, f, &labels);
                    }
                    ui.vertical(|ui| {
                        ui.set_width(list_width);
                        if similar && !comparable {
                            ui.label(&labels.no_similar);
                        }
                        if rows.is_empty() {
                            let message = if favorites && favorites_empty {
                                &labels.no_favorites
                            } else if any {
                                &labels.no_match_filters
                            } else {
                                &labels.no_match
                            };
                            ui.label(message);
                            return;
                        }
                        let lists = ListContext {
                            ui_id: field_id.with("list"),
                            rows: &rows,
                            infos: &infos,
                            catalog,
                            faces: &faces,
                            labels: &labels,
                            preview: preview_size,
                            selected: edit.row.or(current_row),
                            reveal,
                            height: list_height,
                        };
                        lists.show(ui, &mut out);
                    });
                });
                if hide != hide_classes {
                    ui.data_mut(|d| d.insert_temp(hide_id, hide));
                }
                if out.picked.is_some() {
                    pop.close();
                }
                out
            });
        self.filters = filters;
        let mut out = PopupOut::default();
        let mut closed_by = None;
        if let Some(response) = popup {
            closed_by = response.closed_by;
            out = response.inner;
        }
        for (family, on) in out.stars {
            if let Some(settings) = self.settings.as_deref_mut() {
                settings.set_favorite(&family, on);
            }
        }
        picked = picked.or(out.picked);
        let escaped = field.escaped || closed_by == Some(CloseReason::Escape);
        let hovered = out.hovered;
        if open && !edit.moved {
            if tab || ui.input(|i| i.pointer.any_pressed()) {
                edit.moved = true;
            } else {
                ctx.memory_mut(|m| m.request_focus(field_id));
            }
        }
        if edit.reveal > 0 && open {
            edit.reveal -= 1;
            ctx.request_repaint();
        }

        if let Some(family) = &picked {
            self.current.clone_from(family);
            edit.text = shown_of(family);
            edit.select = true;
            ctx.memory_mut(|m| m.request_focus(field_id));
            field.response.mark_changed();
        }
        if escaped {
            open = false;
        }
        if picked.is_some() || (was_open && !open) || escaped {
            if picked.is_none() {
                edit.text = shown_of(self.current);
            }
            edit.query.clear();
            edit.row = None;
            edit.composing = false;
            open = false;
        }
        if edit.select && !escaped {
            select_all(&ctx, field_id, &edit.text);
            edit.select = false;
            ctx.request_repaint();
        }
        let preview = if open { hovered.or_else(|| edit.row.and_then(|i| rows.get(i)).map(|r| r.family.clone())) } else { None };
        edit.current.clone_from(self.current);
        edit.open = open;
        if open || ui.memory(|m| m.has_focus(field_id)) {
            ctx.data_mut(|d| d.insert_temp(edit_id, edit));
        } else {
            ctx.data_mut(|d| d.remove::<Edit>(edit_id));
        }
        FontPickerResponse { picked, preview, response: field.response }
    }
}

/// The row of a list the keyboard lands on after typing `text`: the exact match, else the row that
/// was selected, else the first row. Pure, for the tests.
// pc: crates/ui-egui/src/tools/type_tool/fonts.rs:365 `accept_row`.
pub fn accept_row(rows: &[FamilyRow], text: &str, selected: Option<usize>) -> Option<usize> {
    let exact = text.to_lowercase();
    rows.iter()
        .position(|r| r.family.to_lowercase() == exact || r.shown.to_lowercase() == exact)
        .or(selected.filter(|i| *i < rows.len()))
        .or_else(|| (!rows.is_empty()).then_some(0))
}

/// The list's width at least (PhotoCraft's `FONT_LIST_WIDTH`), in points.
const LIST_WIDTH: f32 = 420.0;
/// The class list's width (PhotoCraft's `CLASS_COLUMN_WIDTH`), in points.
const CLASS_COLUMN_WIDTH: f32 = 118.0;
/// The room a row keeps for its class glyph.
const CLASS_GLYPH_WIDTH: f32 = 28.0;

/// What the field keeps between passes while it has the focus or the list is open.
#[derive(Clone, Default)]
struct Edit {
    /// The field's text: what was typed, or the name of the keyboard row.
    text: String,
    /// What was typed, which narrows the list.
    query: String,
    /// The keyboard row.
    row: Option<usize>,
    /// The current family when the state was made; a different one starts it afresh.
    current: String,
    open: bool,
    composing: bool,
    /// Select the whole text from the next pass, so typing replaces it.
    select: bool,
    /// Passes left in which the list scrolls the marked row into view (the popover's first pass
    /// only measures).
    reveal: u8,
    /// The user moved the focus into the popover (a press or Tab); until then the field keeps it,
    /// taking it back from the popover, which moves it to its first control as it opens.
    moved: bool,
}

#[derive(Default)]
struct PopupOut {
    picked: Option<String>,
    hovered: Option<String>,
    stars: Vec<(String, bool)>,
}

/// Everything the list of rows needs.
struct ListContext<'a> {
    ui_id: Id,
    rows: &'a [FamilyRow],
    infos: &'a Arc<Infos>,
    catalog: &'a Arc<Catalog>,
    faces: &'a Arc<Mutex<Faces>>,
    labels: &'a FontPickerLabels,
    preview: Option<f32>,
    selected: Option<usize>,
    reveal: Option<usize>,
    height: f32,
}

impl ListContext<'_> {
    fn show(&self, ui: &mut Ui, out: &mut PopupOut) {
        lock(self.faces).begin(ui.ctx());
        let tokens = Tokens::of(ui);
        let base = ListRow::<usize>::style(&tokens).row_height;
        let row_height = self.preview.map_or(base, |size| base.max((size * 4.0 / 3.0).ceil() + 4.0));
        let order: Vec<RowInfo<usize>> = (0..self.rows.len()).map(RowInfo::leaf).collect();
        let mut selection = self.selected.map(Selection::only).unwrap_or_default();
        let hovered = Cell::new(None);
        let mut list = List::new(self.ui_id, self.labels.list.clone(), order.as_slice())
            .mode(SelectionMode::Single)
            .selection(&mut selection)
            .row_height(row_height)
            .max_height(self.height);
        if let Some(key) = self.reveal {
            list = list.reveal(key);
        }
        let output = list.show(ui, |_, &key| list_row(self, key, &hovered));
        for event in output.events {
            match event {
                ListEvent::Clicked(key) | ListEvent::Activate(key) => out.picked = self.rows.get(key).map(|r| r.family.clone()),
                ListEvent::Row(RowEvent::Toggle(EditEvent::Set { value, .. } | EditEvent::Commit { value, changed: true, .. })) => {
                    if let Some(row) = self.rows.get(value.row) {
                        out.stars.push((row.family.clone(), value.on));
                    }
                }
                _ => {}
            }
        }
        out.hovered = hovered.take().and_then(|key| self.rows.get(key)).map(|r| r.family.clone());
    }
}

/// One list row: the star column, the name, the sample in the family's own face and the class
/// glyph at the right. A pointer over the row's visible part is reported through `hovered`.
// pc: crates/ui-egui/src/text/font_preview.rs:132 `paint` (sample images, cached per family and
// size) and crates/ui-egui/src/tools/type_tool/fonts.rs:600 (the row).
fn list_row<'a>(cx: &'a ListContext<'a>, key: usize, hovered: &'a Cell<Option<usize>>) -> ListRow<'a, usize> {
    let row = &cx.rows[key];
    let labels = cx.labels;
    let star = ToggleColumn::new(labels.favorite.clone(), Icon::StarFilled, Icon::Star, row.starred);
    let glyph = ListRow::new(key, row.shown.clone()).toggle(star).trailing_item(CLASS_GLYPH_WIDTH, move |ui| {
        if ui.rect_contains_pointer(ui.clip_rect()) {
            hovered.set(Some(key));
        }
        if let Some(class) = cx.infos.traits(cx.catalog, &row.family).and_then(|t| t.class) {
            ui.label(egui::RichText::new(class_glyph(class)).weak()).on_hover_text(class_label(class, labels));
        }
    });
    let Some(size) = cx.preview else { return glyph };
    glyph.trailing_item(size * SAMPLE_WIDTH_PER_POINT, move |ui| {
        let Some(info) = cx.infos.families.get(&row.family) else { return };
        let text = sample_text(ui.ctx(), cx.catalog, info, &labels.sample, &row.shown);
        paint_sample(ui, cx, &info.regular, &text, size);
    })
}

/// The sample's width per point of its size: PhotoCraft's 100 pt box at its 18 pt medium.
const SAMPLE_WIDTH_PER_POINT: f32 = 100.0 / 18.0;

/// The row of filter toggles above the list: the class list's collapse button, variable fonts,
/// favourites and similar fonts, and Clear All.
fn filter_bar(ui: &mut Ui, f: &mut FontFilters, hide_classes: &mut bool, labels: &FontPickerLabels) {
    ui.horizontal(|ui| {
        let (icon, label) = if *hide_classes { (Icon::ChevronsRight, &labels.show_classes) } else { (Icon::ChevronsLeft, &labels.hide_classes) };
        ui.add(IconButton::new(icon, label.clone()).toggle(hide_classes));
        ui.add(IconButton::new(Icon::FontVariable, labels.variable.clone()).toggle(&mut f.variable));
        let star = if f.favorites { Icon::StarFilled } else { Icon::Star };
        ui.add(IconButton::new(star, labels.favorites.clone()).toggle(&mut f.favorites));
        ui.add(IconButton::new(Icon::FontSimilar, labels.similar.clone()).toggle(&mut f.similar));
        if ui.add_enabled(f.any(), Button::new(labels.clear_all.clone())).clicked() {
            f.clear();
        }
    });
}

/// The class list at the left: All Classes and the eight classes, one selected.
fn class_column(ui: &mut Ui, field_id: Id, filters: &mut FontFilters, labels: &FontPickerLabels) {
    let names: Vec<&str> = std::iter::once(labels.all_classes.as_str()).chain(FontClass::ALL.map(|c| class_label(c, labels))).collect();
    let order: Vec<RowInfo<usize>> = (0..names.len()).map(RowInfo::leaf).collect();
    let selected = filters.class.and_then(|c| FontClass::ALL.iter().position(|a| *a == c)).map_or(0, |i| i + 1);
    let mut selection = Selection::only(selected);
    ui.allocate_ui(vec2(CLASS_COLUMN_WIDTH, 0.0), |ui| {
        ui.set_width(CLASS_COLUMN_WIDTH);
        let _ = List::new(field_id.with("classes"), labels.classes.clone(), order.as_slice())
            .mode(SelectionMode::Single)
            .allow_empty(false)
            .selection(&mut selection)
            .show(ui, |_, &key| ListRow::new(key, names[key]));
    });
    let chosen = selection.items.first().copied().unwrap_or(0);
    if chosen != selected {
        filters.class = chosen.checked_sub(1).and_then(|i| FontClass::ALL.get(i).copied());
    }
}

fn class_label(class: FontClass, labels: &FontPickerLabels) -> &str {
    match class {
        FontClass::Serif => &labels.serif,
        FontClass::SlabSerif => &labels.slab_serif,
        FontClass::SansSerif => &labels.sans_serif,
        FontClass::Script => &labels.script,
        FontClass::Monospace => &labels.monospace,
        FontClass::Handwritten => &labels.handwritten,
        FontClass::Blackletter => &labels.blackletter,
        FontClass::Decorative => &labels.decorative,
    }
}

/// The two letters that mark a class at the end of a row.
fn class_glyph(class: FontClass) -> &'static str {
    match class {
        FontClass::Serif => "Se",
        FontClass::SlabSerif => "Sl",
        FontClass::SansSerif => "Sa",
        FontClass::Script => "Sc",
        FontClass::Monospace => "Mo",
        FontClass::Handwritten => "Ha",
        FontClass::Blackletter => "Bl",
        FontClass::Decorative => "De",
    }
}

/// The row `notches` wheel notches from row `index` of `len` (up, positive, is the previous one),
/// stopping at the ends; none when the wheel did not move.
// pc: crates/ui-egui/src/tools/type_tool/fonts.rs:843 `wheel_family_index`.
fn wheel_family_index(index: usize, len: usize, notches: f32) -> Option<usize> {
    let steps = notches.abs().min(len as f32) as usize;
    if steps == 0 || len == 0 {
        return None;
    }
    let last = len - 1;
    Some(if notches < 0.0 { index.saturating_add(steps).min(last) } else { index.saturating_sub(steps) })
}

/// One notch in the direction of this pass's wheel events (up: 1, down: -1), taking them out of
/// the input so nothing under the field scrolls. A plain wheel only; Alt and Ctrl/Cmd zoom the view.
fn take_wheel_notch(ui: &mut Ui) -> f32 {
    let total: f32 = ui.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                Event::MouseWheel { delta, modifiers, .. } if !modifiers.alt && !modifiers.ctrl && !modifiers.command => Some(delta.y),
                _ => None,
            })
            .sum()
    });
    if total != 0.0 {
        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    }
    total.signum() * f32::from(u8::from(total != 0.0))
}

/// Selects all of `text` in the text edit `id` from the next pass.
fn select_all(ctx: &Context, id: Id, text: &str) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::select_all_str(text)));
    state.store(ctx, id);
}

/// The Host's interface language code, English without one.
fn ui_language(ctx: &Context) -> String {
    Host::try_of_ctx(ctx).and_then(|h| h.i18n().map(|i| i.code(h.lang()).to_owned())).unwrap_or_else(|| "en".to_owned())
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// Family data.

/// What the catalog says about a family, read once per catalog generation.
struct Info {
    localized: Vec<LocalizedName>,
    variable: bool,
    /// The upright face nearest weight 400: the one that stands for the family.
    regular: FaceInfo,
}

/// What a face's tables say about its family, as the class filter and similar fonts read them.
#[derive(Clone, Debug)]
struct Traits {
    class: Option<FontClass>,
    panose: [u8; 10],
    weight: u16,
}

impl Traits {
    /// How unlike `other` is to `self`: 0 for the same traits, larger the more they differ. The
    /// class matters most, then the PANOSE digits both fonts state, then the weight.
    // pc: crates/text/src/font_class.rs `difference`.
    fn difference(&self, other: &Traits) -> f32 {
        let mut d = 0.0;
        if self.class != other.class {
            d += 20.0;
        }
        // Digits 1..=9; 0 and 1 mean "any"/"no fit", so only stated digits count.
        for (x, y) in self.panose.iter().zip(other.panose.iter()).skip(1) {
            if *x > 1 && *y > 1 {
                d += f32::from(x.abs_diff(*y)).min(6.0);
            }
        }
        d + f32::from(self.weight.abs_diff(other.weight)) / 100.0
    }
}

struct Infos {
    families: HashMap<String, Info>,
    traits: Mutex<HashMap<String, Option<Traits>>>,
}

impl Infos {
    fn build(catalog: &Catalog) -> Infos {
        let mut groups: HashMap<String, Vec<FaceInfo>> = HashMap::new();
        for face in catalog.faces().iter() {
            groups.entry(face.family.clone()).or_default().push(face.clone());
        }
        let families = groups
            .into_iter()
            .filter_map(|(family, faces)| {
                let regular = regular_face(&faces)?.clone();
                let localized = faces.iter().flat_map(|f| f.localized_families.iter().cloned()).collect();
                Some((family, Info { localized, variable: faces.iter().any(|f| f.variable), regular }))
            })
            .collect();
        Infos { families, traits: Mutex::new(HashMap::new()) }
    }

    /// The family's traits, read from its regular face on first use.
    fn traits(&self, catalog: &Catalog, family: &str) -> Option<Traits> {
        if let Some(t) = lock(&self.traits).get(family) {
            return t.clone();
        }
        let traits = self.families.get(family).and_then(|i| read_traits(catalog, &i.regular));
        lock(&self.traits).insert(family.to_owned(), traits.clone());
        traits
    }

    /// The class if the family's traits were read already.
    fn known_class(&self, family: &str) -> Option<FontClass> {
        lock(&self.traits).get(family).and_then(|t| t.as_ref()).and_then(|t| t.class)
    }
}

type CacheEntry = (Weak<Catalog>, u64, Arc<Infos>);
static CACHE: Mutex<Vec<CacheEntry>> = Mutex::new(Vec::new());

/// The family data of `catalog`, rebuilt when its generation moves.
fn infos(catalog: &Arc<Catalog>) -> Arc<Infos> {
    let generation = catalog.generation();
    let same = |w: &Weak<Catalog>| std::ptr::eq(w.as_ptr(), Arc::as_ptr(catalog));
    let mut cache = lock(&CACHE);
    cache.retain(|(w, _, _)| w.strong_count() > 0);
    if let Some((_, _, infos)) = cache.iter().find(|(w, g, _)| *g == generation && same(w)) {
        return Arc::clone(infos);
    }
    cache.retain(|(w, _, _)| !same(w));
    let infos = Arc::new(Infos::build(catalog));
    cache.push((Arc::downgrade(catalog), generation, Arc::clone(&infos)));
    infos
}

/// The upright face nearest weight 400.
fn regular_face(faces: &[FaceInfo]) -> Option<&FaceInfo> {
    faces.iter().min_by_key(|f| (f.italic, (f.weight - 400.0).abs() as u32))
}

/// The name shown for a family: its record in the interface language, else the family name.
fn shown_name(info: &Info, family: &str, language: &str, english: bool) -> String {
    let primary = |tag: &str| tag.split(['-', '_']).next().unwrap_or(tag).to_ascii_lowercase();
    if english || primary(language) == "en" {
        return family.to_owned();
    }
    let exact = |n: &&LocalizedName| n.language.as_deref().is_some_and(|l| l.replace('_', "-").eq_ignore_ascii_case(&language.replace('_', "-")));
    let same_language = |n: &&LocalizedName| n.language.as_deref().is_some_and(|l| primary(l) == primary(language));
    info.localized.iter().find(exact).or_else(|| info.localized.iter().find(same_language)).map_or_else(|| family.to_owned(), |n| n.name.clone())
}

fn font_of<'a>(bytes: &'a [u8], face: &FaceInfo) -> Option<FontRef<'a>> {
    FontRef::from_index(bytes, face.index).ok()
}

/// A face's class, PANOSE and weight from its `OS/2` and `post` tables.
// pc: crates/text/src/font_class.rs `traits_of`.
fn read_traits(catalog: &Catalog, face: &FaceInfo) -> Option<Traits> {
    let data = catalog.load(face).ok()?;
    let font = font_of(&data.bytes, face)?;
    let os2 = font.os2().ok();
    let family_class = os2.as_ref().map_or(0, |t| t.s_family_class());
    let mut panose = [0u8; 10];
    if let Some(bytes) = os2.as_ref().map(|t| t.panose_10()) {
        for (slot, b) in panose.iter_mut().zip(bytes.iter()) {
            *slot = *b;
        }
    }
    let weight = os2.as_ref().map_or(400, |t| t.us_weight_class()).clamp(1, 1000);
    let fixed_pitch = font.post().ok().is_some_and(|p| p.is_fixed_pitch() != 0);
    Some(Traits { class: classify(&face.family, family_class, panose, fixed_pitch), panose, weight })
}

/// Name fragments (lowercase) that mark a blackletter face; the tables have no class for it.
const BLACKLETTER: [&str; 7] = ["fraktur", "blackletter", "black letter", "textura", "old english", "schwabacher", "rotunda"];

/// Classify from the OS/2 `sFamilyClass` (high byte: the IBM class), the PANOSE digits, the
/// `post` fixed-pitch flag and, last, the family name.
// pc: crates/text/src/font_class.rs `classify`.
fn classify(family: &str, family_class: i16, panose: [u8; 10], fixed_pitch: bool) -> Option<FontClass> {
    let name = family.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    // Whole words of the name, so "Encode Sans" isn't read as a "code" font.
    let words: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    // PANOSE Latin Text proportion 9 is monospaced.
    if fixed_pitch || (panose[0] == 2 && panose[3] == 9) {
        return Some(FontClass::Monospace);
    }
    if has(&BLACKLETTER) {
        return Some(FontClass::Blackletter);
    }
    // PANOSE family kind: 3 Latin Hand Written, 4 Latin Decoratives, 5 Latin Symbol.
    match panose[0] {
        3 => return Some(if family_class >> 8 == 10 { FontClass::Script } else { FontClass::Handwritten }),
        4 | 5 => return Some(FontClass::Decorative),
        _ => {}
    }
    match family_class >> 8 {
        // 4 is Clarendon: bracketed serifs (Georgia), not slabs.
        1..=4 | 7 => return Some(FontClass::Serif),
        5 => return Some(FontClass::SlabSerif),
        8 => return Some(FontClass::SansSerif),
        9 | 12 => return Some(FontClass::Decorative),
        10 => return Some(if has(&["hand", "marker", "brush", "pen", "chalk"]) { FontClass::Handwritten } else { FontClass::Script }),
        _ => {}
    }
    // PANOSE Latin Text serif style: 11-15 are sans (normal, obtuse, perpendicular, flared,
    // rounded); square (6) is a slab; the rest, the square coves (bracketed serifs, as Georgia's)
    // included, are serifs.
    if panose[0] == 2 {
        match panose[1] {
            11..=15 => return Some(FontClass::SansSerif),
            6 => return Some(FontClass::SlabSerif),
            2..=10 => return Some(FontClass::Serif),
            _ => {}
        }
    }
    // Last, the name.
    if words.iter().any(|w| matches!(*w, "mono" | "monospace" | "monospaced" | "code" | "courier") || w.starts_with("consol")) {
        Some(FontClass::Monospace)
    } else if has(&["slab"]) {
        Some(FontClass::SlabSerif)
    } else if has(&["sans", "grotesk", "grotesque", "gothic"]) {
        Some(FontClass::SansSerif)
    } else if has(&["serif", "roman", "garamond", "times", "bodoni", "baskerville", "caslon"]) {
        Some(FontClass::Serif)
    } else if has(&["hand", "marker", "brush", "chalk"]) {
        Some(FontClass::Handwritten)
    } else if has(&["script"]) {
        Some(FontClass::Script)
    } else {
        None
    }
}

// Samples.

/// The faces the samples are drawn in, as egui font families: the same loading budget and
/// eviction as the fonts panel's.
struct Faces {
    frame: u64,
    fonts: FaceFonts,
}

impl Faces {
    /// Starts the pass once, however many pickers draw in it.
    fn begin(&mut self, ctx: &Context) {
        let frame = ctx.cumulative_frame_nr();
        if self.frame != frame {
            self.frame = frame;
            self.fonts.begin_frame(ctx);
        }
    }
}

fn face_fonts(ctx: &Context) -> Arc<Mutex<Faces>> {
    let id = Id::unique("craft-font-picker-faces");
    ctx.data_mut(|d| d.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(Faces { frame: u64::MAX, fonts: FaceFonts::default() }))).clone())
}

/// Sample texts resolved per family, dropped when the catalog changes.
#[derive(Clone, Default)]
struct Samples {
    generation: u64,
    texts: HashMap<(String, String), String>,
}

/// What the row's sample says: `sample` when the family's face draws it, else its name, else the
/// first glyphs the face has (a symbol or non-Latin face).
// pc: crates/ui-egui/src/text/font_preview.rs `sample_texts`.
fn sample_text(ctx: &Context, catalog: &Catalog, info: &Info, sample: &str, shown: &str) -> String {
    let id = Id::unique("craft-font-picker-samples");
    let generation = catalog.generation();
    let key = (info.regular.family.clone(), sample.to_owned());
    let cached = ctx.data(|d| d.get_temp::<Samples>(id)).filter(|s| s.generation == generation).and_then(|s| s.texts.get(&key).cloned());
    if let Some(text) = cached {
        return text;
    }
    let text = (|| {
        let data = catalog.load(&info.regular).ok()?;
        let font = font_of(&data.bytes, &info.regular)?;
        let map = font.charmap();
        let drawn = |t: &str| t.chars().filter(|c| !c.is_whitespace()).all(|c| map.map(c).is_some());
        if drawn(sample) {
            return Some(sample.to_owned());
        }
        if drawn(shown) {
            return Some(shown.to_owned());
        }
        Some(map.mappings().filter_map(|(cp, _)| char::from_u32(cp)).filter(|c| !c.is_whitespace() && !c.is_control()).take(6).collect())
    })()
    .unwrap_or_default();
    ctx.data_mut(|d| {
        let samples = d.get_temp_mut_or_default::<Samples>(id);
        if samples.generation != generation {
            *samples = Samples { generation, texts: HashMap::new() };
        }
        samples.texts.insert(key, text.clone());
    });
    text
}

/// Draws `text` at `size` points in `face`, from the start of the cell; nothing until egui has
/// loaded the face.
fn paint_sample(ui: &Ui, cx: &ListContext<'_>, face: &FaceInfo, text: &str, size: f32) {
    let shown = lock(cx.faces).fonts.family(ui.ctx(), cx.catalog, Owner::Installed, face);
    let Shown::Ready(family) = shown else { return };
    let cell = ui.max_rect();
    let (align, at) =
        if craft_ui::direction::is_rtl(ui) { (Align2::RIGHT_CENTER, cell.right_center()) } else { (Align2::LEFT_CENTER, cell.left_center()) };
    ui.painter().with_clip_rect(cell).text(at, align, text, FontId::new(size, family), ui.visuals().text_color());
}
