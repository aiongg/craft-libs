//! The Brush Preset picker ([`BrushPresetPicker`]): a popover with the brush's Size and Hardness,
//! a search row, the gear menu and the preset cards.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::edit::EditEvent;
use craft_ui::popover::{CloseReason, PopoverAnchor};
use craft_ui::presets::{PresetEvent, PresetReorder, PresetRow};
use egui::{AsIdSalt, Id, IdSalt, Painter, Rect, Ui};

use super::card::BrushCardsState;
use super::labels::{BrushCardLabels, BrushPickerLabels};
use super::settings::BrushPickerSettings;

/// The brush values the picker's head edits, given by the app each pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BrushPickerValues {
    /// The diameter in pixels: the Size field and its log slider.
    pub diameter: f64,
    /// The hardness, 0 to 1, shown in percent; `None` for a tip without one (a sampled tip):
    /// then no Hardness slider shows.
    pub hardness: Option<f64>,
}

/// What the picker asks the app to do beyond the size and hardness edits.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum BrushPick<K> {
    /// A card list event: `Select` (a click: pick, keep open), `Activate` (a double-click or
    /// Enter: pick, and the picker closes), `Rename`, `Delete` (also from the gear),
    /// `Reorder`.
    List(PresetEvent<K>),
    /// Brush Settings…: open the Brush Settings panel.
    OpenSettings,
    /// New Brush Preset (the button or the gear): open the app's New Brush Preset dialog.
    NewPreset,
    /// Import Brushes… (the gear, shown only with [`BrushPresetPicker::import`]).
    Import,
}

impl<K> BrushPick<K> {
    /// Whether this pick closed the picker: `Activate`, and every pick that opens another panel
    /// or dialog (Brush Settings, New Brush Preset, Import).
    pub fn closes(&self) -> bool {
        // pc: crates/ui-egui/src/tools/brush_picker.rs:73 `Pick::closes` (Choose, OpenSettings, Import)
        // pc: crates/ui-egui/src/tools/brush_picker.rs:164-167 (NewPreset closes the picker before the dialog)
        todo!()
    }
}

/// The picker's state, kept by the host while the app runs: the card view (search, open groups,
/// parts, scale). The popover keeps its own dragged size in egui memory.
#[derive(Clone, Debug)]
pub struct BrushPickerState<K> {
    /// The cards' state; [`BrushCardsState::settings`] is what persists.
    pub cards: BrushCardsState<K>,
    /// A rename the gear asked for, started on the cards in the next pass the picker shows.
    rename: Option<K>,
}

impl<K> Default for BrushPickerState<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> BrushPickerState<K> {
    /// Every card part on, scale 1, an empty search.
    pub fn new() -> Self {
        Self { cards: BrushCardsState::new(), rename: None }
    }
}

type PaintPart<'a, K> = Box<dyn FnMut(&Painter, Rect, &K) + 'a>;
type DiameterOf<'a, K> = Box<dyn Fn(&K) -> Option<f64> + 'a>;

/// Photoshop's Brush Preset picker: a resizable [`Popover`](craft_ui::popover::Popover) opened
/// from the options-bar [`BrushPresetChip`](super::BrushPresetChip) or at the pointer (the app
/// decides: a right-click with a painting tool).
///
/// ```text
/// Size                       [ 30 px]
/// ───────────────●───────────────────     log slider, 1 to max_diameter
/// Hardness ────────────●──────── 80 %     round tips only
/// ───────────────────────────────────
/// (search) [Search Brushes ]  [S] [N] [G]  Brush Settings, New Brush Preset, gear
/// ┌ cards ───────────────────────────┐   BrushPresetCards
/// └──────────────────────────────────┘
/// ──────●──────────────────────── grip    card scale
/// ```
///
/// - **Size.** `brush.picker_width` × `brush.picker_height` (300 × 420) until the user drags the
///   grip; at least `brush.picker_min_*` (230 × 170); remembered between opens and kept 40 pt
///   inside the window (the popover's own rules).
/// - **Head.** A block at most `brush.picker_head_max` (350) wide at the leading edge: the
///   [`BrushSizeField`](crate::ui::brush_settings::BrushSizeField) (Size, 1 to
///   [`max_diameter`](Self::max_diameter), log slider), and the Hardness `SliderRow` (0 to
///   100 %) when [`BrushPickerValues::hardness`] is set. They edit the brush live and report
///   [`BrushPresetPickerOutput::diameter`] and [`hardness`](BrushPresetPickerOutput::hardness)
///   as edit gestures; a drag on either closes the gear menu.
/// - **Search row.** A search icon, the "Search Brushes" field (the cards' browser filter: any
///   letter case, groups open while filtering), then at the trailing end the Brush Settings, New
///   Brush Preset and gear icon buttons.
/// - **Gear menu.** New Brush Preset…; Rename Brush… and Delete Brush (enabled with a current
///   preset: Rename opens the card's rename field, Delete reports the browser's `Delete`); the
///   Brush Name, Brush Stroke and Brush Tip boxes (at least one stays on); Import Brushes… with
///   [`import`](Self::import).
/// - **Cards.** [`BrushPresetCards`](super::BrushPresetCards) fill the height the head and
///   footer leave (at least `brush.picker_list_min`), so closing every group doesn't shrink the
///   picker. A click picks and keeps the picker open; a double-click picks and closes it.
/// - **Footer.** The card scale slider (0.15 to 2.0, rounded to 0.01) from the leading edge to
///   `brush.picker_grip_room` short of the grip.
/// - **Closing.** A press outside (not on the [`toggle`](Self::toggle) control, nor in a menu the
///   picker opened), Esc, Enter unless a card rename is open, a double-click on a card, and the
///   picks that open something else ([`BrushPick::closes`]). Edits made in the closing pass
///   (a typed size) are reported first.
/// - **AccessKit.** A non-modal `Dialog` named "Brush Preset picker"; the buttons are named
///   ("Brush Settings…", "New Brush Preset", "Brush Preset Options"); Size is a `SpinButton`
///   with its slider, Hardness a `Slider`, the cards a `Tree`.
/// - **RTL.** The popover, head, search row and footer mirror; the grip sits at the trailing
///   bottom corner.
#[must_use = "show the picker with `show`"]
pub struct BrushPresetPicker<'a, K> {
    id_salt: IdSalt,
    anchor: PopoverAnchor,
    state: &'a mut BrushPickerState<K>,
    rows: &'a [PresetRow<K>],
    current: Option<&'a K>,
    values: BrushPickerValues,
    max_diameter: f64,
    toggle: Option<Id>,
    import: bool,
    reorder: Option<PresetReorder<'a, K>>,
    labels: Option<BrushPickerLabels>,
    card_labels: Option<BrushCardLabels>,
    tip: Option<PaintPart<'a, K>>,
    stroke: Option<PaintPart<'a, K>>,
    diameter_of: Option<DiameterOf<'a, K>>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> BrushPresetPicker<'a, K> {
    /// A picker hanging from `anchor` over the presets in `rows` (build each preset row with
    /// [`BrushPresetCards::row`](super::BrushPresetCards::row)), its view in `state`.
    pub fn new(id_salt: impl AsIdSalt, anchor: PopoverAnchor, state: &'a mut BrushPickerState<K>, rows: &'a [PresetRow<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            anchor,
            state,
            rows,
            current: None,
            values: BrushPickerValues::default(),
            max_diameter: 5000.0,
            toggle: None,
            import: false,
            reorder: None,
            labels: None,
            card_labels: None,
            tip: None,
            stroke: None,
            diameter_of: None,
        }
    }

    /// The preset the brush was picked from: its card shows selected, and the gear's Rename and
    /// Delete act on it (disabled without one).
    pub fn current(mut self, current: Option<&'a K>) -> Self {
        self.current = current;
        self
    }

    /// The brush's diameter and hardness.
    pub fn values(mut self, values: BrushPickerValues) -> Self {
        self.values = values;
        self
    }

    /// The largest diameter the app's engine paints (PhotoCraft's `MAX_BRUSH_SIZE`); 5000 until
    /// set.
    pub fn max_diameter(mut self, max: f64) -> Self {
        self.max_diameter = max;
        self
    }

    /// The control whose click toggles the picker (the chip's `response.id`): a press on it is
    /// not outside.
    pub fn toggle(mut self, id: Id) -> Self {
        self.toggle = Some(id);
        self
    }

    /// Shows Import Brushes… in the gear menu.
    pub fn import(mut self, import: bool) -> Self {
        self.import = import;
        self
    }

    /// Lets cards drag to reorder.
    pub fn reorder(mut self, reorder: PresetReorder<'a, K>) -> Self {
        self.reorder = Some(reorder);
        self
    }

    /// The picker's words; the installed language's by default.
    pub fn labels(mut self, labels: BrushPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The cards' words; the installed language's by default.
    pub fn card_labels(mut self, labels: BrushCardLabels) -> Self {
        self.card_labels = Some(labels);
        self
    }

    /// Paints a preset's tip image (see [`BrushPresetCards::tip`](super::BrushPresetCards::tip)).
    pub fn tip(mut self, paint: impl FnMut(&Painter, Rect, &K) + 'a) -> Self {
        self.tip = Some(Box::new(paint));
        self
    }

    /// Paints a preset's stroke preview (see
    /// [`BrushPresetCards::stroke`](super::BrushPresetCards::stroke)).
    pub fn stroke(mut self, paint: impl FnMut(&Painter, Rect, &K) + 'a) -> Self {
        self.stroke = Some(Box::new(paint));
        self
    }

    /// A preset's tip diameter (see
    /// [`BrushPresetCards::diameter_of`](super::BrushPresetCards::diameter_of)).
    pub fn diameter_of(mut self, diameter: impl Fn(&K) -> Option<f64> + 'a) -> Self {
        self.diameter_of = Some(Box::new(diameter));
        self
    }

    /// Shows the picker while `open` is true; sets `open` to false when it closes. Returns this
    /// pass's edits and picks, in order (empty while closed).
    pub fn show(self, ui: &mut Ui, open: &mut bool) -> BrushPresetPickerOutput<K> {
        // pc: crates/ui-egui/src/canvas/paint_mouse.rs:186-236 `show_picker` (size memory, clamps, close rules; replaced by `Popover`)
        // pc: crates/ui-egui/src/tools/brush_picker.rs:212-286 `body` (head block, search row, list height, footer)
        // pc: crates/ui-egui/src/tools/brush_picker.rs:294-303 `scale_slider` (rounded to 0.01, grip room)
        // pc: crates/ui-egui/src/tools/brush_picker.rs:358-403 `gear_menu`
        todo!()
    }
}

/// What [`BrushPresetPicker::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the picker's edits and picks"]
pub struct BrushPresetPickerOutput<K> {
    /// The Size field or slider: the new diameter, whole pixels from 1 to the maximum. A drag is
    /// one gesture (`Begin`, `Update`, `Commit`); a typed value is a `Set`. The app sends it to its
    /// tool options host (`GestureHost<QuietMarks<_>>`): no history state.
    pub diameter: Option<EditEvent<f64>>,
    /// The Hardness slider: the new hardness, 0 to 1 in whole percent.
    pub hardness: Option<EditEvent<f64>>,
    /// The picks, in the order they happened.
    pub picks: Vec<BrushPick<K>>,
    /// The persisted view when the user changed it this pass (a part box, the end of a scale
    /// drag): the app saves it.
    pub settings: Option<BrushPickerSettings>,
    /// Why the picker closed this pass, if it did; `open` is then false.
    pub closed_by: Option<CloseReason>,
}
