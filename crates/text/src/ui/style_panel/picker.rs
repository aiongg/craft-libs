//! [`StyleChip`] (N3.04): a design tool's named style field (FigCraft's, after Figma's): a chip
//! showing the style applied to the selection (or "Mixed", or nothing), with a detach button,
//! opening a picker popover of the document's styles as a list (preview, name, meta) or a tile
//! grid, grouped by folder, with a create button.

use craft_theme::TypeRole;
use craft_ui::behavior::{GridNav, RowInfo, Selection, SelectionMode};
use craft_ui::controls::{Button, ButtonVariant, ControlSize, IconButton, SearchField, Segmented, TooltipExt as _};
use craft_ui::icons::{Icon, IconSize};
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::Labels as _;
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align2, Color32, Id, Key, Modifiers, Painter, Rect, Response, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2};

use crate::styles::{group_by_folder, style_leaf};

craft_ui::words! {
    /// The chip's and picker's words (FigCraft's strings).
    #[non_exhaustive]
    pub struct StylePickerLabels {
        mixed: "stylePicker.mixed" = "Mixed";
        detach: "stylePicker.detach" = "Detach style";
        apply_style: "stylePicker.apply" = "Apply style";
        create_style: "stylePicker.create" = "Create style";
        list_view: "stylePicker.listView" = "List view";
        grid_view: "stylePicker.gridView" = "Grid view";
        no_styles: "stylePicker.noStyles" = "No styles";
        edit_style: "stylePicker.edit" = "Edit style";
        search: "stylePicker.search" = "Search styles";
        clear_search: "stylePicker.clearSearch" = "Clear search";
        view: "stylePicker.view" = "View";
    }
}

/// What a style row shows before its name.
#[derive(Clone, Debug, PartialEq)]
pub enum StylePreview {
    /// A swatch (colour and fill styles).
    Color(Color32),
    /// A short sample drawn in the style ("Ag"), with the app's family and size already applied
    /// through a font id.
    Text {
        /// The sample text.
        sample: String,
        /// The font to draw it in.
        font: egui::FontId,
    },
    /// An icon from craft-ui's set (effect and grid styles).
    Icon(craft_ui::icons::Icon),
}

/// One style in the picker; its folder is the name's ([`crate::styles::style_folder`]).
#[derive(Clone, Debug, PartialEq)]
pub struct NamedStyleRow {
    /// The style's id.
    pub id: u64,
    /// The style's name; folders are split off it.
    pub name: String,
    /// The second line ("Inter 16/24", "#1473E6").
    pub meta: String,
    /// What the row shows before its name.
    pub preview: StylePreview,
}

/// How the picker lays styles out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StylePickerView {
    #[allow(missing_docs)]
    #[default]
    List,
    /// Tiles of previews (colour styles), the name in the tooltip.
    Grid,
}

/// What the chip did this pass.
#[derive(Debug)]
pub struct StyleChipResponse {
    /// A style picked in the popover: apply it.
    pub picked: Option<u64>,
    /// The detach button: keep the values, drop the link.
    pub detach: bool,
    /// The create button: a new style from the selection's values (named
    /// [`crate::styles::next_style_name`]).
    pub create: bool,
    /// Edit from a row's context menu.
    pub edit: Option<u64>,
    #[allow(missing_docs)]
    pub response: Response,
}

/// The chip, built per pass.
///
/// - Shows the applied style's preview and leaf name with a detach button, or "Mixed" for
///   several. With no style applied FigCraft draws no chip, and its section header's Styles button
///   opens the picker; this chip is that button (a palette icon button), and the app draws its
///   own value field beside it.
/// - A click (or Enter, Space) opens the picker popover under the chip (craft-ui's popover: focus
///   moves in, Esc closes and returns it): a header with the list/grid toggle (`Segmented`) and
///   create (`IconButton`), a `SearchField` that filters by name, then the styles by folder
///   (top-level first, folder captions as section rows), the applied one checked.
/// - List view: a craft-ui [`List`](craft_ui::list::List) of `ListRow`s (the preview in the
///   glyph slot, name with meta as the detail line; folders as `RowInfo::section` rows; one Tab
///   stop, ↑/↓, Enter applies, the context menu's Edit style). Grid view: preview tiles moved
///   through with craft-ui's [`GridNav`](craft_ui::behavior::GridNav), the name in the tooltip.
pub struct StyleChip<'a> {
    id_salt: egui::IdSalt,
    styles: &'a [NamedStyleRow],
    applied: Option<Option<u64>>,
    view: &'a mut StylePickerView,
    opened: bool,
    labels: Option<StylePickerLabels>,
}

/// A row of the picker's list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RowKey {
    Style(u64),
    /// A folder caption, by its position among the groups.
    Folder(usize),
}

/// What the open picker keeps between passes, in egui memory under the chip's id.
#[derive(Clone, Default)]
struct Browsing {
    query: String,
    selection: Selection<RowKey>,
}

/// The context menu's only command.
#[derive(Clone, Copy)]
struct EditCommand;

/// Paints a style's preview in `rect`: a swatch, the sample in its own font, or the icon.
fn paint_preview(ui: &Ui, painter: &Painter, rect: Rect, preview: &StylePreview) {
    let tokens = Tokens::of(ui);
    match preview {
        StylePreview::Color(color) => {
            painter.rect_filled(rect, tokens.radius.control, *color);
        }
        StylePreview::Text { sample, font } => {
            painter.with_clip_rect(rect).text(rect.center(), Align2::CENTER_CENTER, sample, font.clone(), tokens.color.text.normal.to_egui());
        }
        StylePreview::Icon(icon) => craft_ui::icons::paint(ui, rect, *icon, IconSize::Md, tokens.color.text.normal.to_egui()),
    }
}

/// The picker's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StylePickerStyle {
    /// The side of a style tile in the grid view (`size.control.sm`).
    pub tile_side: f32,
    /// The side of a style's preview square in the chip (`size.icon.sm`).
    pub preview_side: f32,
    /// The type role of a folder's caption in the grid view (FigCraft's 10 pt dim caption).
    pub folder_caption: TypeRole,
}

impl<'a> StyleChip<'a> {
    /// The picker's measures, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> StylePickerStyle {
        StylePickerStyle { tile_side: tokens.size.control.sm, preview_side: tokens.size.icon.sm, folder_caption: TypeRole::Caption }
    }

    /// `applied`: `Some(Some(id))` one style, `Some(None)` mixed, `None` no style.
    pub fn new(id_salt: impl egui::AsIdSalt, styles: &'a [NamedStyleRow], applied: Option<Option<u64>>, view: &'a mut StylePickerView) -> Self {
        StyleChip { id_salt: egui::IdSalt::new(id_salt), styles, applied, view, opened: false, labels: None }
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: StylePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Open the picker in this pass, as a click on the chip does (a story shows the picker open
    /// by passing `true` on its first pass). The popover closes by its own rules afterwards; the
    /// app passes `false` from the next pass on.
    pub fn opened(mut self, on: bool) -> Self {
        self.opened = on;
        self
    }

    // figcraft: crates/ui-egui/src/widgets/style_picker.rs:211 `chip`, :154 `Picker`, :341
    // `by_folder`.
    /// Paints the chip and its popover and returns what the user picked.
    pub fn show(self, ui: &mut Ui) -> StyleChipResponse {
        let StyleChip { id_salt, styles, applied, view, opened, labels } = self;
        let labels = labels.unwrap_or_else(|| StylePickerLabels::of_ctx(ui.ctx()));
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let open_id = id.with("open");
        let mut open = opened || ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let applied_style = applied.flatten().and_then(|wanted| styles.iter().find(|style| style.id == wanted));
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let style = Self::style(&tokens);
        let icon_side = style.preview_side;
        let mut out = StyleChipResponse {
            picked: None,
            detach: false,
            create: false,
            edit: None,
            response: ui.allocate_response(vec2(0.0, 0.0), Sense::hover()),
        };

        let row = ui.horizontal(|ui| match applied {
            None => ui.add(IconButton::new(Icon::Palette, labels.apply_style.as_str()).id_salt((id, "chip"))),
            Some(_) => {
                let name = applied_style.map_or(labels.mixed.as_str(), |style| style_leaf(&style.name));
                if let Some(style) = applied_style {
                    let (_, rect) = ui.allocate_space(vec2(icon_side, icon_side));
                    paint_preview(ui, ui.painter(), rect, &style.preview);
                }
                let width = ui.available_width() - side - ui.spacing().item_spacing.x;
                let chip = ui.add(Button::new(name).variant(ButtonVariant::Secondary).min_width(width).id_salt((id, "chip")));
                if ui.add(IconButton::new(Icon::Unlink, labels.detach.as_str()).id_salt((id, "detach"))).clicked() {
                    out.detach = true;
                }
                chip
            }
        });
        let (chip, width) = (row.inner, row.response.rect.width());
        if chip.clicked() {
            open = !open;
        }

        let popup = Popover::new((id, "popup"), PopoverAnchor::Control(chip.clone()))
            .size(PopoverSize::Fit)
            .close(PopoverClose::default())
            .toggle(chip.id)
            .label(labels.apply_style.as_str())
            .show(ui, &mut open, |ui, pop| {
                ui.set_width(width);
                let mut state = ui.data(|d| d.get_temp::<Browsing>(id.with("browsing"))).unwrap_or_else(|| Browsing {
                    query: String::new(),
                    selection: applied_style.map_or_else(Selection::new, |style| Selection::only(RowKey::Style(style.id))),
                });
                let mut result = Picked::default();
                show_picker(ui, id, &Pass { styles, applied_style, labels: &labels, style }, view, &mut state, &mut result, |field| {
                    pop.initial_focus(field)
                });
                ui.data_mut(|d| d.insert_temp(id.with("browsing"), state));
                if result.picked.is_some() || result.edit.is_some() {
                    pop.close();
                }
                result
            });
        match popup {
            Some(shown) => {
                out.picked = shown.inner.picked;
                out.create = shown.inner.create;
                out.edit = shown.inner.edit;
            }
            None => ui.data_mut(|d| {
                d.remove_temp::<Browsing>(id.with("browsing"));
            }),
        }
        ui.data_mut(|d| d.insert_temp(open_id, open));
        out.response = chip;
        out
    }
}

/// What the popover's content reports.
#[derive(Default)]
struct Picked {
    picked: Option<u64>,
    create: bool,
    edit: Option<u64>,
}

/// What the popover needs from the chip.
struct Pass<'a> {
    styles: &'a [NamedStyleRow],
    applied_style: Option<&'a NamedStyleRow>,
    labels: &'a StylePickerLabels,
    style: StylePickerStyle,
}

/// The popover's content: the header (view toggle and create), the search field, and the styles by folder.
fn show_picker(
    ui: &mut Ui,
    id: Id,
    pass: &Pass<'_>,
    view: &mut StylePickerView,
    state: &mut Browsing,
    result: &mut Picked,
    initial_focus: impl FnOnce(Id),
) {
    let Pass { styles, applied_style, labels, .. } = *pass;
    let ctx = ui.ctx().clone();
    let search_salt = (id, "search");
    let search_id = Interactive::id_in(ui, search_salt);
    initial_focus(search_id);

    ui.horizontal(|ui| {
        Segmented::new((id, "view"), labels.view.as_str(), view)
            .icon_item(StylePickerView::List, Icon::List, labels.list_view.as_str())
            .icon_item(StylePickerView::Grid, Icon::LayoutGrid, labels.grid_view.as_str())
            .show(ui);
        if ui.add(IconButton::new(Icon::Plus, labels.create_style.as_str()).id_salt((id, "create"))).clicked() {
            result.create = true;
        }
    });

    // ↓ in the search field moves into the styles before the field is drawn, so the key is not
    // the field's.
    let list_id = List::id_in(ui, (id, "list"));
    if ctx.memory(|m| m.has_focus(search_id)) && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
        match *view {
            StylePickerView::List => ctx.memory_mut(|m| m.request_focus(list_id)),
            StylePickerView::Grid => ui.data_mut(|d| {
                d.insert_temp(id.with("enter-grid"), true);
            }),
        }
    }
    let edit = SearchField::new(search_salt, labels.search.as_str(), &state.query, labels.clear_search.as_str())
        .placeholder(labels.search.as_str())
        .show(ui);
    if let Some(text) = edit.event.as_ref().and_then(|event| event.value()) {
        state.query.clone_from(text);
    }

    let query = state.query.trim().to_lowercase();
    let shown: Vec<&NamedStyleRow> = styles.iter().filter(|style| query.is_empty() || style.name.to_lowercase().contains(&query)).collect();
    let groups = group_by_folder(shown.iter().copied(), |style| style.name.as_str());
    if groups.is_empty() {
        ui.label(labels.no_styles.as_str());
        return;
    }
    match *view {
        StylePickerView::List => list_view(ui, id, pass, &groups, state, result),
        StylePickerView::Grid => grid_view(ui, id, pass, &groups, search_id, result),
    }
    let _ = applied_style;
}

/// The context menu of a style: Edit style.
fn edit_menu(ui: &Ui, labels: &StylePickerLabels, response: &Response, open: Option<OpenAt>) -> bool {
    let entries = [MenuEntry::from(MenuItem::new(labels.edit_style.as_str(), EditCommand))];
    ContextMenu::new(&entries).open_at(open).own_gestures(open.is_none()).show(ui, response).chosen.is_some()
}

fn list_view(ui: &mut Ui, id: Id, pass: &Pass<'_>, groups: &[(String, Vec<&NamedStyleRow>)], state: &mut Browsing, result: &mut Picked) {
    let Pass { labels, .. } = *pass;
    let mut infos = Vec::new();
    let mut rows: Vec<(RowKey, &str, Option<&NamedStyleRow>)> = Vec::new();
    for (at, (folder, members)) in groups.iter().enumerate() {
        if !folder.is_empty() {
            infos.push(RowInfo::section(RowKey::Folder(at)));
            rows.push((RowKey::Folder(at), folder.as_str(), None));
        }
        for style in members {
            infos.push(RowInfo::leaf(RowKey::Style(style.id)));
            rows.push((RowKey::Style(style.id), style_leaf(&style.name), Some(style)));
        }
    }
    let menu_id = id.with("menu-row");
    let glyph_tokens = Tokens::of(ui);
    let max_height = ui.available_height();
    let out = List::new((id, "list"), labels.apply_style.as_str(), infos.as_slice())
        .selection(&mut state.selection)
        .mode(SelectionMode::Single)
        .allow_empty(false)
        .max_height(max_height)
        .show(ui, |at, _| {
            let (key, label, style) = &rows[at];
            let mut row = ListRow::new(key.clone(), *label);
            if let Some(style) = style {
                row = row.detail(style.meta.as_str());
                row = match &style.preview {
                    StylePreview::Icon(icon) => row.icon(*icon),
                    preview => {
                        let preview = preview.clone();
                        let tokens = glyph_tokens.clone();
                        row.glyph(move |painter, rect| paint_glyph(&tokens, painter, rect, &preview))
                    }
                };
            }
            row
        });
    let mut menu = None;
    for event in out.events {
        match event {
            ListEvent::Clicked(RowKey::Style(style)) | ListEvent::Activate(RowKey::Style(style)) => result.picked = Some(style),
            ListEvent::ContextMenu(request) => {
                if let Some(RowKey::Style(style)) = request.row {
                    ui.data_mut(|d| d.insert_temp(menu_id, style));
                    menu = Some(request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer));
                }
            }
            _ => {}
        }
    }
    if edit_menu(ui, labels, &out.response, menu)
        && let Some(style) = ui.data(|d| d.get_temp::<u64>(menu_id))
    {
        result.edit = Some(style);
    }
}

/// A glyph-slot preview, painted without a `Ui`.
fn paint_glyph(tokens: &craft_ui::theme::UiTokens, painter: &Painter, rect: Rect, preview: &StylePreview) {
    match preview {
        StylePreview::Color(color) => {
            painter.rect_filled(rect, tokens.radius.control, *color);
        }
        StylePreview::Text { sample, font } => {
            painter.text(rect.center(), Align2::CENTER_CENTER, sample, font.clone(), tokens.color.text.normal.to_egui());
        }
        StylePreview::Icon(_) => {}
    }
}

fn grid_view(ui: &mut Ui, id: Id, pass: &Pass<'_>, groups: &[(String, Vec<&NamedStyleRow>)], search_id: Id, result: &mut Picked) {
    let Pass { applied_style, labels, style, .. } = *pass;
    let side = style.tile_side;
    let ctx = ui.ctx().clone();
    let tokens = Tokens::of(ui);
    let gap = tokens.space.gap.inline;
    let columns = GridNav::columns(ui.available_width() - ui.spacing().scroll.allocated_width(), 0.0, gap, side);
    let total: usize = groups.iter().map(|(_, members)| members.len()).sum();
    let ids: Vec<Id> = (0..total).map(|at| id.with(("cell", at))).collect();
    let lines = |members: usize| (0..members.div_ceil(columns)).map(move |row| ((row + 1) * columns).min(members) - row * columns);
    let nav = GridNav::ragged(groups.iter().flat_map(|(_, members)| lines(members.len()))).rtl(craft_ui::direction::is_rtl(ui));
    let focused = ids.iter().position(|cell| ctx.memory(|m| m.has_focus(*cell)));
    let applied_at = applied_style.and_then(|style| groups.iter().flat_map(|(_, members)| members.iter()).position(|member| member.id == style.id));
    let mut stop = focused.or(applied_at).unwrap_or(0);
    let mut moved = false;
    if let Some(from) = focused {
        if nav.row_of(from) == 0 && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
            ctx.memory_mut(|m| m.request_focus(search_id));
        } else {
            (stop, moved) = GridNav::roving_move_in(ui, &ids, &nav, applied_at, true);
        }
    } else if ui.data_mut(|d| d.remove_temp::<bool>(id.with("enter-grid"))).is_some() {
        stop = 0;
        moved = true;
        ctx.memory_mut(|m| m.request_focus(ids[0]));
    } else {
        (stop, moved) = GridNav::roving_move_in(ui, &ids, &nav, applied_at, true);
    }

    let grid = ui.new_child(
        UiBuilder::new()
            .id_salt((id, "grid"))
            .max_rect(ui.available_rect_before_wrap())
            .accessibility_role(Role::Grid)
            .accessibility_label(labels.grid_view.as_str()),
    );
    let mut grid = grid;
    let muted = tokens.color.text.muted.to_egui();
    ScrollArea::vertical().id_salt((id, "scroll")).max_height(grid.available_height()).auto_shrink([false, true]).show(&mut grid, |ui| {
        ui.spacing_mut().item_spacing.y = gap;
        let forward = craft_ui::direction::forward(ui);
        let mut at = 0;
        for (folder, members) in groups {
            if !folder.is_empty() {
                ui.label(LayoutJob::single_section(folder.clone(), role_format(&tokens, style.folder_caption, muted)));
            }
            for row in members.chunks(columns) {
                let (_, strip) = ui.allocate_space(vec2(columns as f32 * (side + gap) - gap, side));
                for (column, style) in row.iter().enumerate() {
                    let x = craft_ui::direction::start_x(ui, strip) + forward * column as f32 * (side + gap);
                    let rect = Rect::from_min_max(pos2(x.min(x + forward * side), strip.top()), pos2(x.max(x + forward * side), strip.bottom()));
                    let mut cell =
                        Interactive::with_id(ids[at], Role::GridCell, style.name.as_str()).selected(applied_style.is_some_and(|a| a.id == style.id));
                    if at != stop {
                        cell = cell.sense(Sense::CLICK);
                    }
                    let response = cell.interact_at(ui, rect).finish(ui);
                    if ui.is_rect_visible(rect) {
                        paint_preview(ui, &ui.painter().with_clip_rect(rect), rect, &style.preview);
                    }
                    if moved && at == stop {
                        response.scroll_to_me(None);
                    }
                    if response.clicked() {
                        result.picked = Some(style.id);
                    }
                    if edit_menu(ui, labels, &response, None) {
                        result.edit = Some(style.id);
                    }
                    response.tooltip(style.name.as_str());
                    at += 1;
                }
            }
        }
    });
    ui.allocate_rect(grid.min_rect(), Sense::hover());
}
