//! [`GlyphsPanel`] (F14.05): Photoshop's Glyphs panel. A family and style picker, a category
//! list (Entire Font, Punctuation, Currency, …), a grid of glyph cells drawn by the app's text
//! engine, a zoom slider, and the recently used glyphs in a row at the top. Double-click or Enter
//! inserts the glyph into the text being edited.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use craft_ui::behavior::GridKey;
use craft_ui::behavior::GridNav;
use craft_ui::controls::{Select, Slider};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Corner, Interactive, Surface, SurfaceKind};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{
    AsIdSalt, ColorImage, EventFilter, Id, IdSalt, Key, Modifiers, Rect, ScrollArea, TextureHandle, TextureId, TextureOptions, Ui, pos2, vec2,
};
use serde::{Deserialize, Serialize};

craft_ui::words! {
    /// The panel's words, English by default.
    #[non_exhaustive]
    pub struct GlyphsLabels {
        show: "glyphs.show" = "Show:";
        entire_font: "glyphs.entireFont" = "Entire Font";
        recent: "glyphs.recent" = "Recently Used";
        style: "glyphs.style" = "Style";
        zoom: "glyphs.zoom" = "Zoom";
        no_text: "glyphs.noText" = "Edit text to insert glyphs";
        font: "glyphs.font" = "Font";
        category: "glyphs.category" = "Category";
        count: "glyphs.count" = "{n} glyphs";
        selected: "glyphs.selected" = "U+{code}  ·  {n} glyphs";
        cell: "glyphs.cell" = "{char}, U+{code}";
        punctuation: "glyphs.category.punctuation" = "Punctuation";
        alternates: "glyphs.category.alternates" = "Alternates for Selection";
        numbers: "glyphs.category.numbers" = "Numbers";
        currency: "glyphs.category.currency" = "Currency";
        symbols: "glyphs.category.symbols" = "Symbols";
        math_symbols: "glyphs.category.mathSymbols" = "Math Symbols";
        arrows: "glyphs.category.arrows" = "Arrows";
        latin: "glyphs.category.latin" = "Latin";
        greek: "glyphs.category.greek" = "Greek";
        cyrillic: "glyphs.category.cyrillic" = "Cyrillic";
        letterlike: "glyphs.category.letterlike" = "Letterlike Symbols";
        scripts: "glyphs.category.scripts" = "Superscripts and Subscripts";
        ornaments: "glyphs.category.ornaments" = "Ornaments";
        search: "glyphs.search" = "Search fonts";
        clear: "glyphs.clear" = "Clear";
    }
}

/// The glyph categories, Photoshop's list in order. These English names are the stable ids stored
/// in the settings and passed to [`GlyphSource::in_category`]; the panel shows them through
/// [`GlyphsLabels::category_name`].
pub const CATEGORIES: &[&str] = &[
    "Entire Font",
    "Recently Used",
    "Punctuation",
    "Alternates for Selection",
    "Numbers",
    "Currency",
    "Symbols",
    "Math Symbols",
    "Arrows",
    "Latin",
    "Greek",
    "Cyrillic",
    "Letterlike Symbols",
    "Superscripts and Subscripts",
    "Ornaments",
];

impl GlyphsLabels {
    /// The shown name of a [`CATEGORIES`] id (the id itself for any other).
    pub fn category_name<'a>(&'a self, id: &'a str) -> &'a str {
        match id {
            "Entire Font" => &self.entire_font,
            "Recently Used" => &self.recent,
            "Punctuation" => &self.punctuation,
            "Alternates for Selection" => &self.alternates,
            "Numbers" => &self.numbers,
            "Currency" => &self.currency,
            "Symbols" => &self.symbols,
            "Math Symbols" => &self.math_symbols,
            "Arrows" => &self.arrows,
            "Latin" => &self.latin,
            "Greek" => &self.greek,
            "Cyrillic" => &self.cyrillic,
            "Letterlike Symbols" => &self.letterlike,
            "Superscripts and Subscripts" => &self.scripts,
            "Ornaments" => &self.ornaments,
            other => other,
        }
    }
}

/// Where glyph cells come from: the app's text engine draws a glyph of a face at a pixel size and
/// lists the characters a face has. Apps plug in their own (PhotoCraft's `photocraft_text`).
pub trait GlyphSource: Send + Sync {
    /// Every character the face (family, style) has a glyph for, in code point order.
    fn charmap(&self, family: &str, style: &str) -> Vec<char>;
    /// The glyph of `c` in the face, `px` pixels tall, as an image for a texture; `None` when the
    /// face lacks it. The panel tints the image with the text colour, so a glyph is white with its
    /// coverage in the alpha channel.
    fn glyph(&self, family: &str, style: &str, c: char, px: u32) -> Option<ColorImage>;
    /// Whether `c` is in `category` (one of [`CATEGORIES`]).
    fn in_category(&self, c: char, category: &str) -> bool;
}

/// The panel's remembered state (`[glyphs_panel]`): category, zoom and the recent glyphs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlyphsPanelSettings {
    /// The chosen category, one of [`CATEGORIES`].
    pub category: String,
    /// Cell size in points, 24..=64 (PhotoCraft).
    pub zoom: f32,
    /// Most recent first, at most 25 (Photoshop).
    pub recent: Vec<String>,
}

impl craft_host::Section for GlyphsPanelSettings {
    const KEY: &'static str = "glyphs_panel";
}

/// What the panel reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphsPanelResponse {
    /// A glyph to insert at the caret (double-click, Enter on the focused cell).
    pub insert: Option<String>,
    /// The family or style changed in the panel's own pickers.
    pub font_changed: bool,
}

/// The Glyphs panel body. The family comes from the text selection while editing (the panel
/// follows the caret), else from the panel's own picker.
///
/// - Keys: the grid is one Tab stop; arrows move the focused cell, Page Up/Down a screen, Home/End
///   to the ends, Enter inserts. Hovering a cell shows its character and code point as a tooltip.
/// - A cell is drawn from [`GlyphSource::glyph`] at the zoom size, cached per (face, char, px).
///   The grid shows only the current category, and only the cells in view are drawn; a cell the
///   source returns no image for stays empty.
/// - Without text being edited the cells still show, Enter and double-click do nothing and the
///   footer says so (`no_text`).
/// - The font and style lists come from the app ([`families`](GlyphsPanel::families),
///   [`styles`](GlyphsPanel::styles)); without a list the name is shown as text.
pub struct GlyphsPanel<'a> {
    salt: IdSalt,
    source: &'a dyn GlyphSource,
    family: &'a mut String,
    style: &'a mut String,
    settings: &'a mut GlyphsPanelSettings,
    families: &'a [String],
    styles: &'a [String],
    editing: bool,
    labels: Option<GlyphsLabels>,
}

/// How many recent glyphs are kept (Photoshop).
const RECENT_LIMIT: usize = 25;
/// The smallest grid cell, in points.
const CELL_MIN: f32 = 24.0;
/// The largest grid cell, in points.
const CELL_MAX: f32 = 64.0;
/// The room round a glyph in its cell, as a fraction of the cell.
const GLYPH_INSET_RATIO: f32 = 0.1;
/// The grid's height when the host gives the panel no bound.
const UNBOUNDED_GRID_HEIGHT: f32 = 260.0;

type Textures = HashMap<char, Option<TextureHandle>>;

/// Textures of one face at one pixel size.
#[derive(Default)]
struct GlyphCache {
    key: (String, String, u32),
    textures: Textures,
}

/// The characters of a face in one category.
#[derive(Default)]
struct CharsCache {
    key: (String, String, String),
    chars: Arc<Vec<char>>,
}

/// Where the keyboard lead is in the grid and what was in view last frame.
#[derive(Clone, Copy, Default)]
struct GridState {
    lead: usize,
    first_visible: usize,
    last_visible: usize,
    offset: f32,
}

fn shared<T: Default + Send + Sync + 'static>(ctx: &egui::Context, id: Id) -> Arc<Mutex<T>> {
    ctx.data_mut(|d| d.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(T::default()))).clone())
}

fn texture(
    ctx: &egui::Context,
    cache: &Mutex<GlyphCache>,
    source: &dyn GlyphSource,
    family: &str,
    style: &str,
    c: char,
    px: u32,
) -> Option<TextureId> {
    let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    let key = (family.to_owned(), style.to_owned(), px);
    if cache.key != key || cache.textures.len() > 4096 {
        cache.key = key;
        cache.textures.clear();
    }
    let handle = cache.textures.entry(c).or_insert_with(|| {
        source.glyph(family, style, c, px).map(|image| ctx.load_texture(format!("glyph-{}-{px}", c as u32), image, TextureOptions::LINEAR))
    });
    handle.as_ref().map(TextureHandle::id)
}

fn code(c: char) -> String {
    format!("{:04X}", c as u32)
}

impl<'a> GlyphsPanel<'a> {
    /// `id_salt` names the panel to egui; give each panel on a screen its own.
    pub fn new(
        id_salt: impl AsIdSalt,
        source: &'a dyn GlyphSource,
        family: &'a mut String,
        style: &'a mut String,
        settings: &'a mut GlyphsPanelSettings,
    ) -> Self {
        GlyphsPanel { salt: IdSalt::new(id_salt), source, family, style, settings, families: &[], styles: &[], editing: false, labels: None }
    }

    /// Whether text is being edited (inserting is possible).
    pub fn editing(mut self, editing: bool) -> Self {
        self.editing = editing;
        self
    }

    /// The families the font list offers.
    pub fn families(mut self, families: &'a [String]) -> Self {
        self.families = families;
        self
    }

    /// The styles of the family, for the style list.
    pub fn styles(mut self, styles: &'a [String]) -> Self {
        self.styles = styles;
        self
    }

    /// Use `labels` instead of the English words or the installed translation.
    pub fn labels(mut self, labels: GlyphsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draw the panel body in `ui`.
    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:627 `glyphs_panel` (pickers, category
    // list, grid, zoom) and `:578 glyph_texture`, `:595 charmap`; recents through `:608 insert_glyph`.
    pub fn show(self, ui: &mut Ui) -> GlyphsPanelResponse {
        let GlyphsPanel { salt, source, family, style, settings, families, styles, editing, labels } = self;
        let labels = labels.unwrap_or_else(|| GlyphsLabels::of(ui));
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let mut response = GlyphsPanelResponse::default();
        let panel = ui.scope_id().with(("glyphs", salt));

        // Font and style.
        let (family_before, style_before) = (family.clone(), style.clone());
        ui.horizontal(|ui| {
            if families.is_empty() {
                ui.label(family.as_str());
            } else {
                let mut select =
                    Select::new(panel.with("family"), labels.font.clone(), &mut *family).searchable(labels.search.clone(), labels.clear.clone());
                for name in families {
                    select = select.option(name.clone(), name.clone());
                }
                let _ = select.show(ui);
            }
            if styles.is_empty() {
                ui.label(style.as_str());
            } else {
                let mut select = Select::new(panel.with("style"), labels.style.clone(), &mut *style);
                for name in styles {
                    select = select.option(name.clone(), name.clone());
                }
                let _ = select.show(ui);
            }
        });
        response.font_changed = *family != family_before || *style != style_before;

        // Show: the category.
        if settings.category.is_empty() {
            settings.category = CATEGORIES[0].to_owned();
        }
        ui.horizontal(|ui| {
            ui.label(labels.show.clone());
            let mut select = Select::new(panel.with("category"), labels.category.clone(), &mut settings.category);
            for name in CATEGORIES {
                select = select.option((*name).to_owned(), labels.category_name(name));
            }
            let _ = select.show(ui);
        });

        let recent_cell = tokens.size.control.sm;
        let recent_pad = tokens.stroke.thin;
        let recent_pitch = recent_cell + 2.0 * tokens.stroke.thin;
        let recent_glyph_inset = recent_cell * GLYPH_INSET_RATIO;
        let size = if settings.zoom.is_finite() && settings.zoom > 0.0 { settings.zoom.clamp(CELL_MIN, CELL_MAX) } else { tokens.size.control.md };
        let glyph_inset = size * GLYPH_INSET_RATIO;
        let ppp = ctx.pixels_per_point();
        let ink = Content::Normal.color(&tokens).to_egui();
        let mut activated: Option<String> = None;

        // Recently used.
        ui.label(labels.recent.clone());
        let recent_cache = shared::<GlyphCache>(&ctx, panel.with("recent-textures"));
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), recent_pitch), egui::Sense::hover());
        Surface::paint(ui.painter(), strip, SurfaceKind::Sunken, &tokens);
        let rtl = direction::is_rtl(ui);
        let recent_px = (recent_cell * ppp).round().max(8.0) as u32;
        for (i, glyph) in settings.recent.iter().enumerate() {
            let Some(c) = glyph.chars().next() else { continue };
            let offset = recent_pad + i as f32 * recent_pitch;
            if offset + recent_cell > strip.width() {
                break;
            }
            let x = if rtl { strip.right() - offset - recent_cell } else { strip.left() + offset };
            let cell = Rect::from_min_size(pos2(x, strip.top() + recent_pad), vec2(recent_cell, recent_cell));
            let name = labels.cell.replace("{char}", &c.to_string()).replace("{code}", &code(c));
            let sensed = Interactive::with_id(panel.with(("recent", i)), Role::Button, name.clone())
                .surface(SurfaceKind::Raised)
                .corner(Corner::Control)
                .interact_at(ui, cell);
            let cell_response = sensed.finish(ui).on_hover_text(name);
            if let Some(id) = texture(&ctx, &recent_cache, source, family, style, c, recent_px) {
                paint_glyph(ui, id, cell.shrink(recent_glyph_inset), ink);
            }
            if cell_response.clicked() {
                activated = Some(glyph.clone());
            }
        }
        ui.add_space(ui.spacing().item_spacing.y);

        // The grid.
        let chars = category_chars(&ctx, panel, source, family, style, &settings.category, &settings.recent);
        // What the panel has left above the zoom row; a host with no bound gets a fixed height.
        let grid_height = if ui.available_height().is_finite() {
            (ui.available_height() - tokens.size.control.md - ui.spacing().item_spacing.y).max(size)
        } else {
            UNBOUNDED_GRID_HEIGHT
        };
        let width = (ui.available_width() - ui.spacing().scroll.bar_width - ui.spacing().scroll.bar_inner_margin).max(size);
        let columns = GridNav::columns(width, 0.0, 0.0, size);
        let rows = chars.len().div_ceil(columns);
        let state_id = panel.with("grid-state");
        let mut state: GridState = ctx.data(|d| d.get_temp(state_id)).unwrap_or_default();
        state.lead = state.lead.min(chars.len().saturating_sub(1));
        let cell_id = |i: usize| panel.with(("cell", i));
        // The Tab stop is the lead, or the first cell in view when the lead was scrolled away.
        let in_view = |i: usize| (state.first_visible..state.last_visible).contains(&i);
        let mut stop = if in_view(state.lead) || chars.is_empty() { state.lead } else { state.first_visible.min(chars.len().saturating_sub(1)) };
        let mut scroll_to: Option<usize> = None;
        if !chars.is_empty() && ui.memory(|m| m.has_focus(cell_id(stop))) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(cell_id(stop), EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() })
            });
            let page = ((grid_height / size).floor() as usize).saturating_sub(1);
            let nav = GridNav::wrapped(chars.len(), columns).rtl(rtl).page(page);
            let mut lead = stop;
            ui.input_mut(|i| {
                for key in [Key::ArrowRight, Key::ArrowLeft, Key::ArrowDown, Key::ArrowUp, Key::Home, Key::End, Key::PageUp, Key::PageDown] {
                    if i.consume_key(Modifiers::NONE, key)
                        && let Some(grid_key) = GridKey::from_key(key)
                    {
                        lead = nav.step(lead, grid_key, None).index;
                    }
                }
                if i.consume_key(Modifiers::NONE, Key::Enter) {
                    activated = Some(chars[stop].to_string());
                }
            });
            if lead != stop {
                ui.memory_mut(|m| m.request_focus(cell_id(lead)));
                scroll_to = Some(lead);
            }
            state.lead = lead;
        } else {
            state.lead = stop;
        }
        // The cell the focus moved to is the Tab stop this frame, so it can take the focus.
        stop = state.lead;

        let grid_cache = shared::<GlyphCache>(&ctx, panel.with("grid-textures"));
        let px = ((size - 2.0 * glyph_inset) * ppp).round().max(8.0) as u32;
        let mut area = ScrollArea::vertical().id_salt(panel.with("scroll")).max_height(grid_height).auto_shrink([false, false]);
        if let Some(lead) = scroll_to {
            let top = (lead / columns) as f32 * size;
            if top < state.offset {
                area = area.vertical_scroll_offset(top);
            } else if top + size > state.offset + grid_height {
                area = area.vertical_scroll_offset(top + size - grid_height);
            }
        }
        let mut clicked: Option<usize> = None;
        let output = area.show_rows(ui, size, rows, |ui, range| {
            state.first_visible = (range.start * columns).min(chars.len());
            state.last_visible = (range.end * columns).min(chars.len());
            for row in range {
                let (strip, _) = ui.allocate_exact_size(vec2(width, size), egui::Sense::hover());
                for column in 0..columns {
                    let index = row * columns + column;
                    let Some(&c) = chars.get(index) else { break };
                    let x = if rtl { strip.right() - (column + 1) as f32 * size } else { strip.left() + column as f32 * size };
                    let cell = Rect::from_min_size(pos2(x, strip.top()), vec2(size, size));
                    let name = labels.cell.replace("{char}", &c.to_string()).replace("{code}", &code(c));
                    let mut element = Interactive::with_id(cell_id(index), Role::Button, name.clone())
                        .surface(SurfaceKind::Raised)
                        .selected(index == stop)
                        .position_in_set(index);
                    if index != stop {
                        element = element.sense(egui::Sense::CLICK);
                    }
                    let cell_response = element.interact_at(ui, cell).finish(ui).on_hover_text(name);
                    ui.ctx().accesskit_node_builder(cell_response.id, |node| {
                        node.set_size_of_set(chars.len());
                        if index != stop {
                            node.add_action(egui::accesskit::Action::Focus);
                        }
                    });
                    if ui.is_rect_visible(cell)
                        && let Some(id) = texture(ui.ctx(), &grid_cache, source, family, style, c, px)
                    {
                        paint_glyph(ui, id, cell.shrink(glyph_inset), ink);
                    }
                    if cell_response.double_clicked() {
                        activated = Some(c.to_string());
                        clicked = Some(index);
                    } else if cell_response.clicked() {
                        clicked = Some(index);
                    }
                }
            }
        });
        state.offset = output.state.clamped_offset().y;
        if let Some(index) = clicked {
            state.lead = index;
            ui.memory_mut(|m| m.request_focus(cell_id(index)));
        }
        ctx.data_mut(|d| d.insert_temp(state_id, state));

        // Footer: the selection, the count and the zoom.
        ui.horizontal(|ui| {
            let info = match chars.get(state.lead) {
                Some(&c) => labels.selected.replace("{code}", &code(c)).replace("{n}", &chars.len().to_string()),
                None => labels.count.replace("{n}", &chars.len().to_string()),
            };
            ui.label(info);
            let zoom = Slider::new(panel.with("zoom"), labels.zoom.clone(), size, CELL_MIN..=CELL_MAX).step(1.0).show(ui);
            if let Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = zoom.event {
                settings.zoom = value as f32;
            }
        });
        if !editing {
            ui.label(labels.no_text.clone());
        }

        if let Some(glyph) = activated
            && editing
        {
            settings.recent.retain(|g| *g != glyph);
            settings.recent.insert(0, glyph.clone());
            settings.recent.truncate(RECENT_LIMIT);
            response.insert = Some(glyph);
        }
        response
    }
}

fn paint_glyph(ui: &Ui, id: TextureId, rect: Rect, tint: egui::Color32) {
    ui.painter().add(egui::Shape::image(id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint));
}

/// The characters of the face in `category`, cached until the face or the category changes.
fn category_chars(
    ctx: &egui::Context,
    panel: Id,
    source: &dyn GlyphSource,
    family: &str,
    style: &str,
    category: &str,
    recent: &[String],
) -> Arc<Vec<char>> {
    let all = |ctx: &egui::Context| -> Arc<Vec<char>> {
        let cache = shared::<CharsCache>(ctx, panel.with("chars"));
        let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (family.to_owned(), style.to_owned(), category.to_owned());
        if cache.key != key {
            let charmap = source.charmap(family, style);
            let chars = if category == CATEGORIES[0] { charmap } else { charmap.into_iter().filter(|c| source.in_category(*c, category)).collect() };
            *cache = CharsCache { key, chars: Arc::new(chars) };
        }
        cache.chars.clone()
    };
    if category == "Recently Used" {
        let mut seen = Vec::new();
        for c in recent.iter().filter_map(|g| g.chars().next()) {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
        return Arc::new(seen);
    }
    all(ctx)
}
