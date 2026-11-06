//! The brush preset card and the card view of a preset library ([`BrushPresetCards`]).

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use craft_theme::TypeRole;
use craft_ui::direction;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::presets::view::grid_room;
use craft_ui::presets::{CellShape, PresetBrowser, PresetOutput, PresetReorder, PresetRow, PresetState, PresetView};
use craft_ui::style::brush::BrushPanelStyle;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use egui::text::{LayoutJob, TextWrapping};
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Painter, Rect, Ui, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

use super::BrushPickerSettings;
use super::labels::BrushCardLabels;
use super::style::BrushCardStyle;

/// One part of a brush preset card, as the picker's gear menu names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrushCardPart {
    /// The preset's name ("Brush Name").
    Name,
    /// The stroke preview ("Brush Stroke").
    Stroke,
    /// The tip thumbnail with its size under it ("Brush Tip").
    Tip,
}

/// Which parts a card shows. At least one is always on: [`set`](Self::set) refuses to clear the
/// last one and [`normalised`](Self::normalised) turns all three on when none is. All on by
/// default.
///
/// - Every part: tip top left with its size under it, stroke top right, the name across the
///   bottom (a 2 × 2 grid whose bottom row is merged).
/// - Tip and name: the name moves into the stroke's cell; fixed `card_tip_name` cells.
/// - Tip alone: fixed `card_tip_only` cells.
/// - Tip off: no tip cell; the stroke (or name) takes the width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushCardParts {
    /// The name ("Brush Name").
    pub name: bool,
    /// The stroke preview ("Brush Stroke").
    pub stroke: bool,
    /// The tip thumbnail ("Brush Tip").
    pub tip: bool,
}

impl Default for BrushCardParts {
    fn default() -> Self {
        Self::ALL
    }
}

impl BrushCardParts {
    /// Every part on.
    pub const ALL: BrushCardParts = BrushCardParts { name: true, stroke: true, tip: true };

    /// Whether `part` shows.
    pub fn get(self, part: BrushCardPart) -> bool {
        match part {
            BrushCardPart::Name => self.name,
            BrushCardPart::Stroke => self.stroke,
            BrushCardPart::Tip => self.tip,
        }
    }

    /// `part` turned `on`, unless that would clear the last part (the card would show no brush):
    /// then unchanged.
    pub fn set(self, part: BrushCardPart, on: bool) -> Self {
        // pc: crates/ui-egui/src/tools/brush_picker.rs:390-396 (clicking the last checked box does nothing)
        let mut next = self;
        match part {
            BrushCardPart::Name => next.name = on,
            BrushCardPart::Stroke => next.stroke = on,
            BrushCardPart::Tip => next.tip = on,
        }
        if next.name || next.stroke || next.tip { next } else { self }
    }

    /// These parts, or all three when none is on (a loaded setting, a host's own value).
    pub fn normalised(self) -> Self {
        // pc: crates/ui-egui/src/tools/brush_picker.rs:151-153 (`restore`: none on gives all three)
        if self.name || self.stroke || self.tip { self } else { Self::ALL }
    }

    /// The tip alone: the small fixed cells.
    pub fn tip_only(self) -> bool {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:482 `tip_only` ((name, stroke, tip) == (false, false, true))
        !self.name && !self.stroke && self.tip
    }

    /// Tip and name, stroke off: the name sits in the stroke's cell, in fixed cells.
    pub fn tip_and_name(self) -> bool {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:487 `tip_and_name` ((true, false, true))
        self.name && !self.stroke && self.tip
    }
}

/// How small the tip thumbnail of a preset `diameter` pixels across is drawn, relative to its
/// cell: `(diameter / 40).clamp(0.3, 1.0)`, so small tips read smaller and 40 px and up fill it.
/// The cards and the Brush Tip Shape grid of tips both use it. A diameter that isn't a finite
/// number gives 1.
pub fn tip_thumb_scale(diameter: f64) -> f32 {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:255 `thumb_scale` ((size / 40).clamp(0.3, 1.0))
    if diameter.is_finite() { (diameter / 40.0).clamp(0.3, 1.0) as f32 } else { 1.0 }
}

/// Where a card's parts go: pure geometry, from the card's cell, its parts, the picker's scale
/// and the tokens. Rects are in points, inside `card`; none overlap.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushCardLayout {
    /// The tile: the cell inset by `style.inset × scale` (PhotoCraft 4) at both ends (at most
    /// half the cell), full height. The whole cell stays the drop target.
    pub card: Rect,
    /// The tip's cell (thumbnail and number band), when the tip shows.
    pub tip_cell: Option<Rect>,
    /// The tip thumbnail, centred in what the number band leaves, at
    /// `min(style.tip_base × scale, that room's side) × tip_thumb_scale(diameter)`; on a
    /// compact card the tip cell's shorter side less 2, centred.
    pub tip: Option<Rect>,
    /// The size number's band at the bottom of the tip cell, 1 pt off its bottom; `None` on a
    /// compact card or without the tip.
    pub number: Option<Rect>,
    /// The stroke preview: from `style.stroke_inset × scale` (8) past the tip cell,
    /// `clamp(cell width − 2 × inset, style.stroke_width[0] × scale, style.stroke_width[1] ×
    /// scale)` (24, 260) wide and at most the cell, `style.stroke_height × max(scale, 0.6)`
    /// (36) high, centred vertically in the top row.
    pub stroke: Option<Rect>,
    /// The name: the merged bottom row inset `style.name_inset × scale` (6) at both ends, or
    /// the stroke's cell when only the tip and the name show (`card_stroke_inset × scale` at the
    /// start, `card_name_inset × scale` at the end).
    pub name: Option<Rect>,
}

impl BrushCardLayout {
    /// Heights and padding stop shrinking at this scale; widths keep shrinking.
    pub const MIN_HEIGHT_SCALE: f32 = 0.60;
    /// At or below this scale the tip drops its size number and takes its cell whole.
    pub const COMPACT_SCALE: f32 = 0.70;

    /// The layout of a card in `cell` (from [`cell_size`](Self::cell_size)). `diameter` is the
    /// preset's tip size in pixels (`None`: drawn at full scale), `number_height` the height of the
    /// laid-out size number (ignored when compact).
    ///
    /// Padding `style.pad × scale` across and `× max(scale, 0.6)` down; the name row
    /// `style.name_height × max(scale, 0.6)`; the tip cell `min(style.tip_width ×
    /// scale, half the top row)` wide, or the whole top row on a tip-only card.
    pub fn of(cell: Rect, parts: BrushCardParts, scale: f32, diameter: Option<f64>, number_height: f32, style: &BrushCardStyle) -> Self {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:524-635 `preset_card` (pad, name_h, top_h, tip, stroke and name cells)
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:504 `card_tip_rect` (number band, 1 pt gap, tip side)
        let scale = sane_scale(scale);
        let scale_h = scale.max(Self::MIN_HEIGHT_SCALE);
        let compact = Self::compact(scale);
        let name_in_top = parts.tip_and_name();
        let inset = (style.inset * scale).min(cell.width() * 0.5);
        let card = cell.shrink2(vec2(inset, 0.0));
        let inner = card.shrink2(vec2(style.pad * scale, style.pad * scale_h));
        let name_h = if parts.name && !name_in_top { style.name_height * scale_h } else { 0.0 };
        let top_h = if parts.tip_only() {
            (style.tip_only.y - style.pad * 2.0) * scale_h
        } else if name_in_top {
            (style.tip_name.y - style.pad * 2.0) * scale_h
        } else if parts.tip || parts.stroke {
            (style.max.y - style.pad * 2.0 - style.name_height) * scale_h
        } else {
            0.0
        };
        let top = Rect::from_min_size(inner.left_top(), vec2(inner.width().max(0.0), top_h));
        let mut x = top.left();
        let (mut tip_cell, mut tip, mut number) = (None, None, None);
        if parts.tip {
            let width = if parts.tip_only() { top.width() } else { (style.tip_width * scale).min(top.width() * 0.5) };
            let tip_rect = Rect::from_min_size(top.left_top(), vec2(width, top_h));
            tip_cell = Some(tip_rect);
            if compact {
                tip = Some(Rect::from_center_size(
                    tip_rect.center(),
                    Vec2::splat((tip_rect.width().min(tip_rect.height()) - style.tip_compact_inset).max(1.0)),
                ));
            } else {
                let band = number_height.max(0.0);
                let bottom = tip_rect.bottom() - style.number_gap;
                number = Some(Rect::from_min_max(
                    pos2(tip_rect.left(), (bottom - band).max(tip_rect.top())),
                    pos2(tip_rect.right(), bottom.max(tip_rect.top())),
                ));
                let room = Rect::from_min_max(tip_rect.min, pos2(tip_rect.right(), (tip_rect.bottom() - band - style.tip_gap).max(tip_rect.top())));
                let fit = (style.tip_base * scale).min(room.width().min(room.height())).max(0.0);
                let side = fit * diameter.map_or(1.0, tip_thumb_scale);
                tip = Some(Rect::from_center_size(room.center(), Vec2::splat(if side.is_finite() { side.max(1.0) } else { 1.0 })));
            }
            x = tip_rect.right();
        }
        let (mut stroke, mut name) = (None, None);
        if parts.stroke || name_in_top {
            let room = Rect::from_min_max(pos2(x, top.top()), top.right_bottom());
            let start = style.stroke_inset * scale;
            if parts.stroke {
                let reach = if room.width().is_finite() { room.width().max(1.0) } else { 1.0 };
                let width =
                    (reach - 2.0 * start).clamp(style.stroke_width[0] * scale, style.stroke_width[1] * scale).min((reach - start).max(1.0)).max(1.0);
                let height = style.stroke_height * scale_h;
                stroke = Some(Rect::from_min_size(pos2(room.left() + start, room.center().y - height / 2.0), vec2(width, height)));
            } else {
                name = Some(Rect::from_min_max(
                    pos2(room.left() + start, room.top()),
                    pos2((room.right() - style.name_inset * scale).max(room.left() + start), room.bottom()),
                ));
            }
        }
        if name_h > 0.0 {
            let row = Rect::from_min_size(pos2(inner.left(), top.bottom()), vec2(inner.width().max(0.0), name_h));
            let edge = style.name_inset * scale;
            name = Some(Rect::from_min_max(pos2(row.left() + edge, row.top()), pos2((row.right() - edge).max(row.left() + edge), row.bottom())));
        }
        Self { card, tip_cell, tip, number, stroke, name }
    }

    /// The size of one card's cell at `scale` for `parts`, the cards sharing a row `width` wide
    /// (the browser's grid room). Tip only: `style.tip_only × scale` (height floored at 0.6);
    /// tip and name: `style.tip_name × scale` likewise; otherwise `card_max` width ×
    /// scale (at most `width`) by the padding, the top row and the name row, where the top row
    /// is `card_max` height − 2 × pad − name height, and absent when neither tip nor stroke
    /// shows.
    ///
    /// Cards keep this width and the spare room stays at the row's end (lead call 19); PhotoCraft
    /// stretched them to share the room.
    pub fn cell_size(parts: BrushCardParts, scale: f32, width: f32, style: &BrushCardStyle) -> Vec2 {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:541-556 (heights per part combination)
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:472 `card_columns` (replaced: fixed widths, as preset_panels.rs:487-491)
        let scale = sane_scale(scale);
        let scale_h = scale.max(Self::MIN_HEIGHT_SCALE);
        if parts.tip_only() {
            return vec2(style.tip_only.x * scale, style.tip_only.y * scale_h);
        }
        if parts.tip_and_name() {
            return vec2(style.tip_name.x * scale, style.tip_name.y * scale_h);
        }
        let name_h = if parts.name { style.name_height * scale_h } else { 0.0 };
        let top_h = if parts.tip || parts.stroke { (style.max.y - style.pad * 2.0 - style.name_height) * scale_h } else { 0.0 };
        let width = if width.is_finite() { width.max(1.0) } else { style.max.x * scale };
        vec2((style.max.x * scale).min(width), style.pad * scale_h * 2.0 + top_h + name_h)
    }

    /// Whether a card at `scale` is compact: at or below [`COMPACT_SCALE`](Self::COMPACT_SCALE).
    pub fn compact(scale: f32) -> bool {
        scale <= Self::COMPACT_SCALE
    }
}

/// A scale that is a finite number in the picker's range, else 1.
fn sane_scale(scale: f32) -> f32 {
    if scale.is_finite() { scale.clamp(*BrushPickerSettings::SCALE_RANGE.start(), *BrushPickerSettings::SCALE_RANGE.end()) } else { 1.0 }
}

/// `rect` mirrored left to right inside `cell`.
fn mirrored(rect: Rect, cell: Rect) -> Rect {
    Rect::from_min_max(pos2(cell.left() + cell.right() - rect.right(), rect.top()), pos2(cell.left() + cell.right() - rect.left(), rect.bottom()))
}

/// The card view's state, kept by the host: the browser's view (search, open groups) and what
/// the cards show. [`settings`](Self::settings) and [`restore`](Self::restore) save and load the
/// part that persists.
#[derive(Clone, Debug)]
pub struct BrushCardsState<K> {
    /// The preset browser's state. Its `thumb` is the card width, which
    /// [`BrushPresetCards`] sets from `scale` each pass.
    pub browser: PresetState<K>,
    /// What each card shows.
    pub parts: BrushCardParts,
    /// The card scale, in [`BrushPickerSettings::SCALE_RANGE`](super::BrushPickerSettings::SCALE_RANGE).
    pub scale: f32,
}

impl<K> Default for BrushCardsState<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> BrushCardsState<K> {
    /// Every part on, scale 1, an empty search.
    pub fn new() -> Self {
        Self { browser: PresetState::new(), parts: BrushCardParts::ALL, scale: 1.0 }
    }
}

type PaintPart<'a, K> = Box<dyn FnMut(&Painter, Rect, &K) + 'a>;
type DiameterOf<'a, K> = Box<dyn Fn(&K) -> Option<f64> + 'a>;

/// A preset library as brush cards: a [`PresetBrowser`](craft_ui::presets::PresetBrowser) whose
/// thumbnail is the card, so selection, search, groups, folders, rename, delete, reorder and keys
/// come from the browser. The app paints the tip and stroke images (from its engine); the card
/// draws its tile, the size number and the name.
///
/// - **Cells.** Grid view only. The cell shape follows the parts ([`BrushCardLayout::cell_size`]);
///   the width is `card_max` width × [`BrushCardsState::scale`], with the browser's
///   the grid gap between cells.
/// - **Look.** The tile on `card_fill`, the hover fill `state.hover`, the selected fill
///   `state.selected`; the selected ring is the browser's (`preset.ring.*`). Name in `text.muted`
///   (`label` role), size number in `text.faint` (`caption` role); the text keeps its size at
///   every scale.
/// - **Pointer.** A click selects (`Select`); a double-click activates (`Activate`); drag
///   reorders with [`reorder`](Self::reorder); right-click opens the browser's menu (rename,
///   delete). Hover shows the name as a tooltip.
/// - **AccessKit.** The browser's `Tree` of `TreeItem`s; build rows with [`row`](Self::row) so
///   each card is named "{name}, {size} px".
#[must_use = "show the cards with `show`"]
pub struct BrushPresetCards<'a, K> {
    id_salt: IdSalt,
    state: &'a mut BrushCardsState<K>,
    rows: &'a [PresetRow<K>],
    selected: Option<&'a K>,
    height: Option<f32>,
    reorder: Option<PresetReorder<'a, K>>,
    start_rename: Option<K>,
    labels: Option<BrushCardLabels>,
    tip: Option<PaintPart<'a, K>>,
    stroke: Option<PaintPart<'a, K>>,
    diameter_of: Option<DiameterOf<'a, K>>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> BrushPresetCards<'a, K> {
    /// Cards for `rows` (groups and presets, as for a `PresetBrowser`), with the view in `state`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut BrushCardsState<K>, rows: &'a [PresetRow<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            rows,
            selected: None,
            height: None,
            reorder: None,
            start_rename: None,
            labels: None,
            tip: None,
            stroke: None,
            diameter_of: None,
        }
    }

    /// A preset row named for AccessKit as "{name}, {size} px" ([`BrushCardLabels::preset_size`],
    /// the size rounded to a whole pixel in `numbers`, the installed number format
    /// (`LocaleNumbers::of`)); the label stays `name`, which a rename starts from.
    pub fn row(key: K, name: impl Into<String>, depth: usize, diameter: f64, labels: &BrushCardLabels, numbers: &LocaleNumbers) -> PresetRow<K> {
        // new: the browser reads `accessible_name` from the host's rows.
        let name = name.into();
        let size = numbers.clone().ungrouped().format(if diameter.is_finite() { diameter.round() } else { 0.0 }, 0..=0);
        let accessible = craft_i18n::fmt(&labels.preset_size, &[("name", &name), ("size", &size)]);
        PresetRow::item(key, name, depth).accessible_name(accessible)
    }

    /// The sizes and colors of a card from `tokens`.
    pub fn style(tokens: &UiTokens) -> BrushCardStyle {
        let (b, c, size, space) = (BrushPanelStyle::new(tokens), &tokens.color, &tokens.size, &tokens.space);
        BrushCardStyle {
            pad: space.gap.tight,
            inset: space.gap.tight,
            tip_width: size.thumb.lg,
            tip_base: b.card_tip_base,
            name_height: size.icon.lg,
            name_inset: space.gap.inline,
            tip_compact_inset: space.gap.tight,
            number_gap: b.card_number_gap,
            tip_gap: space.gap.tight,
            stroke_inset: space.gap.stack,
            stroke_width: [b.card_stroke_min, b.card_stroke_max],
            stroke_height: size.thumb.md,
            max: Vec2::from(b.card_max),
            tip_only: Vec2::from(b.card_tip_only),
            tip_name: Vec2::from(b.card_tip_name),
            radius: CornerRadius::same(tokens.shape.radius.sm.round() as u8),
            fill: b.card_fill.to_egui(),
            hover: c.state.hover.to_egui(),
            selected: c.state.selected.to_egui(),
            name: c.text.muted.to_egui(),
            number: c.text.faint.to_egui(),
        }
    }

    /// The current preset: drawn selected.
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// A fixed height: the list keeps it however many groups are open (the picker's list fills
    /// the picker). Unset: the browser's own height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Lets presets drag to reorder. A tip-only grid drops between cells, the others between
    /// rows, as the browser's grid does.
    pub fn reorder(mut self, reorder: PresetReorder<'a, K>) -> Self {
        self.reorder = Some(reorder);
        self
    }

    /// Opens the rename field on `key` this pass (the picker's Rename Brush…).
    pub fn start_rename(mut self, key: K) -> Self {
        self.start_rename = Some(key);
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushCardLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Paints a preset's tip image into the rect (white on dark themes, as the app's engine renders
    /// it). Without it the tip cell stays empty.
    pub fn tip(mut self, paint: impl FnMut(&Painter, Rect, &K) + 'a) -> Self {
        self.tip = Some(Box::new(paint));
        self
    }

    /// Paints a preset's stroke preview into the rect. Without it the stroke cell stays empty.
    pub fn stroke(mut self, paint: impl FnMut(&Painter, Rect, &K) + 'a) -> Self {
        self.stroke = Some(Box::new(paint));
        self
    }

    /// A preset's tip diameter in pixels: the size number and the tip's
    /// [`tip_thumb_scale`]. `None` (or no closure): no number, full-scale tip.
    pub fn diameter_of(mut self, diameter: impl Fn(&K) -> Option<f64> + 'a) -> Self {
        self.diameter_of = Some(Box::new(diameter));
        self
    }

    /// Shows the cards and returns the browser's output: `Select`, `Activate`, `Rename`, `Delete`,
    /// `Reorder` and the others, in order.
    pub fn show(self, ui: &mut Ui) -> PresetOutput<K> {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:524-635 `preset_card` (fill = card colour × 0.90, current and hover fills, parts)
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:493 `darker` (in gamma space, opaque; now the `card_fill` token)
        let Self { id_salt, state, rows, selected, height, reorder, start_rename, labels, mut tip, mut stroke, diameter_of } = self;
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = labels.unwrap_or_else(|| BrushCardLabels::of(ui));
        let numbers = LocaleNumbers::of(ui).as_ref().clone().ungrouped();
        let rtl = direction::is_rtl(ui);
        let parts = state.parts.normalised();
        let scale = sane_scale(state.scale);
        let cell = BrushCardLayout::cell_size(parts, scale, grid_room(ui, false), &style);
        state.browser.thumb = cell.x;

        let mut browser = PresetBrowser::new((&id_salt, "browser"), labels.presets.as_str(), &mut state.browser, rows)
            .selected(selected)
            .cell_shape(CellShape::Ratio(cell.x / cell.y.max(1.0)))
            .thumb_range(cell.x..=cell.x)
            .view_lock(PresetView::Grid);
        if let Some(height) = height {
            browser = browser.min_height(height).max_height(height);
        }
        if let Some(reorder) = reorder {
            browser = browser.reorder(reorder);
        }
        if let Some(key) = start_rename {
            browser = browser.start_rename(key);
        }
        let names: HashMap<&K, &str> = rows.iter().filter(|r| !r.is_group()).map(|r| (&r.key, r.label.as_str())).collect();
        let name_format = role_format(&tokens, TypeRole::Label, style.name);
        let number_format = role_format(&tokens, TypeRole::Caption, style.number);
        browser.show(ui, |painter, rect, info| {
            let key = info.key;
            let diameter = diameter_of.as_ref().and_then(|f| f(key));
            let text = |text: String, format: &egui::TextFormat, width: f32| {
                let mut job = LayoutJob::single_section(text, format.clone());
                job.wrap = TextWrapping::truncate_at_width(width.max(1.0));
                painter.layout_job(job)
            };
            let number = diameter.map(|d| text(numbers.format(d.round(), 0..=0), &number_format, rect.width()));
            let layout = BrushCardLayout::of(rect, parts, scale, diameter, number.as_ref().map_or(0.0, |g| g.size().y), &style);
            let at = |r: Rect| if rtl { mirrored(r, rect) } else { r };
            let tile = at(layout.card);
            painter.rect_filled(tile, style.radius, style.fill);
            if info.selected {
                painter.rect_filled(tile, style.radius, style.selected);
            } else if info.hovered {
                painter.rect_filled(tile, style.radius, style.hover);
            }
            if let (Some(paint), Some(tip_rect)) = (tip.as_mut(), layout.tip) {
                paint(painter, at(tip_rect), key);
            }
            if let (Some(galley), Some(band)) = (number, layout.number) {
                let band = at(band);
                painter.galley(pos2(band.center().x - galley.size().x / 2.0, band.bottom() - galley.size().y), galley, Color32::PLACEHOLDER);
            }
            if let (Some(paint), Some(stroke_rect)) = (stroke.as_mut(), layout.stroke) {
                paint(painter, at(stroke_rect), key);
            }
            if let Some(name_rect) = layout.name {
                let name_rect = at(name_rect);
                let label = names.get(key).copied().unwrap_or("");
                let galley = text(label.to_owned(), &name_format, name_rect.width());
                let x = if rtl { name_rect.right() - galley.size().x } else { name_rect.left() };
                painter.galley(pos2(x, name_rect.center().y - galley.size().y / 2.0), galley, Color32::PLACEHOLDER);
            }
        })
    }
}
