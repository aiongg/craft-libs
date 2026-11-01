//! The brush preset card and the card view of a preset library ([`BrushPresetCards`]).
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::presets::{PresetOutput, PresetReorder, PresetRow, PresetState};
use egui::{AsIdSalt, IdSalt, Painter, Rect, Ui, Vec2};
use serde::{Deserialize, Serialize};

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
/// - Tip and name: the name moves into the stroke's cell; fixed `brush.card_tip_name` cells.
/// - Tip alone: fixed `brush.card_tip_only` cells.
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
        todo!()
    }

    /// These parts, or all three when none is on (a loaded setting, a host's own value).
    pub fn normalised(self) -> Self {
        // pc: crates/ui-egui/src/tools/brush_picker.rs:151-153 (`restore`: none on gives all three)
        todo!()
    }

    /// The tip alone: the small fixed cells.
    pub fn tip_only(self) -> bool {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:482 `tip_only` ((name, stroke, tip) == (false, false, true))
        todo!()
    }

    /// Tip and name, stroke off: the name sits in the stroke's cell, in fixed cells.
    pub fn tip_and_name(self) -> bool {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:487 `tip_and_name` ((true, false, true))
        todo!()
    }
}

/// How small the tip thumbnail of a preset `diameter` pixels across is drawn, relative to its
/// cell: `(diameter / 40).clamp(0.3, 1.0)`, so small tips read smaller and 40 px and up fill it.
/// The cards and the Brush Tip Shape grid of tips both use it. A diameter that isn't a finite
/// number gives 1.
pub fn tip_thumb_scale(diameter: f64) -> f32 {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:255 `thumb_scale` ((size / 40).clamp(0.3, 1.0))
    todo!()
}

/// Where a card's parts go: pure geometry, from the card's cell, its parts, the picker's scale
/// and the tokens. Rects are in points, inside `card`; none overlap.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushCardLayout {
    /// The tile: the cell inset by `brush.card_inset × scale` (PhotoCraft 4) at both ends (at most
    /// half the cell), full height. The whole cell stays the drop target.
    pub card: Rect,
    /// The tip's cell (thumbnail and number band), when the tip shows.
    pub tip_cell: Option<Rect>,
    /// The tip thumbnail, centred in what the number band leaves, at
    /// `min(brush.card_tip_base × scale, that room's side) × tip_thumb_scale(diameter)`; on a
    /// compact card the tip cell's shorter side less 2, centred.
    pub tip: Option<Rect>,
    /// The size number's band at the bottom of the tip cell, 1 pt off its bottom; `None` on a
    /// compact card or without the tip.
    pub number: Option<Rect>,
    /// The stroke preview: from `brush.card_stroke_inset × scale` (8) past the tip cell,
    /// `clamp(cell width − 2 × inset, brush.card_stroke_min × scale, brush.card_stroke_max ×
    /// scale)` (24, 260) wide and at most the cell, `brush.card_stroke_height × max(scale, 0.6)`
    /// (36) high, centred vertically in the top row.
    pub stroke: Option<Rect>,
    /// The name: the merged bottom row inset `brush.card_name_inset × scale` (6) at both ends, or
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
    /// Padding `brush.card_pad × scale` across and `× max(scale, 0.6)` down; the name row
    /// `brush.card_name_height × max(scale, 0.6)`; the tip cell `min(brush.card_tip_width ×
    /// scale, half the top row)` wide, or the whole top row on a tip-only card.
    pub fn of(cell: Rect, parts: BrushCardParts, scale: f32, diameter: Option<f64>, number_height: f32, style: &BrushCardStyle) -> Self {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:524-635 `preset_card` (pad, name_h, top_h, tip, stroke and name cells)
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:504 `card_tip_rect` (number band, 1 pt gap, tip side)
        todo!()
    }

    /// The size of one card's cell at `scale` for `parts`, the cards sharing a row `width` wide
    /// (the browser's grid room). Tip only: `brush.card_tip_only × scale` (height floored at 0.6);
    /// tip and name: `brush.card_tip_name × scale` likewise; otherwise `brush.card_max` width ×
    /// scale (at most `width`) by the padding, the top row and the name row, where the top row
    /// is `card_max` height − 2 × pad − name height, and absent when neither tip nor stroke
    /// shows.
    ///
    /// Cards keep this width and the spare room stays at the row's end (lead call 19); PhotoCraft
    /// stretched them to share the room.
    pub fn cell_size(parts: BrushCardParts, scale: f32, width: f32, style: &BrushCardStyle) -> Vec2 {
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:541-556 (heights per part combination)
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:472 `card_columns` (replaced: fixed widths, as preset_panels.rs:487-491)
        todo!()
    }

    /// Whether a card at `scale` is compact: at or below [`COMPACT_SCALE`](Self::COMPACT_SCALE).
    pub fn compact(scale: f32) -> bool {
        scale <= Self::COMPACT_SCALE
    }
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
///   the width is `brush.card_max` width × [`BrushCardsState::scale`], with the browser's
///   `preset.grid_gap` between cells.
/// - **Look.** The tile on `brush.card_fill`, the hover fill `list.hover`, the selected fill
///   `list.selected`; the selected ring is the browser's (`preset.ring.*`). Name in `text.muted`
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
    /// the size rounded to a whole pixel in the installed number format); the label stays `name`,
    /// which a rename starts from.
    pub fn row(key: K, name: impl Into<String>, depth: usize, diameter: f64, labels: &BrushCardLabels) -> PresetRow<K> {
        // new: the browser reads `accessible_name` from the host's rows.
        todo!()
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
        // pc: crates/ui-egui/src/panels/brushes_tab.rs:493 `darker` (in gamma space, opaque; now the `brush.card_fill` token)
        todo!()
    }
}
