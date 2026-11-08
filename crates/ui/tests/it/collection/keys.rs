//! The collection's keys: Space actions, ragged-grid moves, hidden items, digits against type-ahead, the header
//! flag, the lead's source and the first sort order.

use super::kit::{Count, Dismissed, ROW_H, Row, Words};
use super::table::Table;
use crate::contract::kit::PlainStyle;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, ItemLayout, ItemSource, NavShape, ScrollPos, Selection, SelectionChange};
use craft_ui::contract::{
    Collection, CollectionEvent, CollectionProps, CollectionResponse, ColumnEvent, ColumnKind, ColumnSpec, Common, Component, ContainerProps, Cx,
    Focus as _, LeadSource, Props as _, Role, SelectMode, ShowCollection as _, SpaceAction, ToggleColumn,
};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, input, pointer, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 240.0);
const COL: f32 = 100.0;
const NAME: ColumnKey = ColumnKey::new("name");
const SIZE: ColumnKey = ColumnKey::new("size");
const MUTE: ColumnKey = ColumnKey::new("mute");
const TILE: Vec2 = Vec2::new(30.0, ROW_H);

/// What a collection reported and where its items were built.
#[derive(Default)]
struct Seen {
    id: Cell<Option<Id>>,
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    rows: RefCell<Vec<(usize, Rect)>>,
    digits: RefCell<Vec<u8>>,
}

impl Seen {
    fn take(&self, out: CollectionResponse<usize, Dismissed>) {
        self.id.set(Some(out.response.id));
        self.events.borrow_mut().extend(out.events);
    }

    fn note(&self, key: usize, rect: Rect) {
        let mut rows = self.rows.borrow_mut();
        rows.retain(|(held, _)| *held != key);
        rows.push((key, rect));
    }

    fn row(&self, key: usize) -> Rect {
        self.rows.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the row was built")
    }

    fn leads(&self) -> Vec<(usize, LeadSource)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::LeadMoved { key, by, .. } = event { Some((*key, *by)) } else { None })
            .collect()
    }

    fn lead_keys(&self) -> Vec<usize> {
        self.leads().into_iter().map(|(key, _)| key).collect()
    }

    fn toggles(&self) -> Vec<(usize, bool)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| {
                if let CollectionEvent::Toggle(edit) = event {
                    edit.value()
                        .filter(|_| matches!(edit, craft_behavior::EditEvent::Set { .. } | craft_behavior::EditEvent::Commit { .. }))
                        .map(|toggle| (toggle.key, toggle.on))
                } else {
                    None
                }
            })
            .collect()
    }

    fn activations(&self) -> Vec<usize> {
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::Activate(key) = event { Some(*key) } else { None }).collect()
    }

    fn sorts(&self) -> Vec<(ColumnKey, bool)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                CollectionEvent::Column(ColumnEvent::Sort { key, descending, .. }) => Some((key.clone(), *descending)),
                _ => None,
            })
            .collect()
    }

    fn id(&self) -> Id {
        self.id.get().expect("the collection ran")
    }
}

fn press(frames: &mut Frames, key: Key, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![input::key_press(key, Modifiers::NONE)], &mut *show);
    frames.frame(vec![craft_ui::testing::focus::key_release(key, Modifiers::NONE)], &mut *show);
}

fn focus(frames: &mut Frames, seen: &Seen, show: &mut impl FnMut(&mut Ui)) {
    request(&frames.ctx, FocusTarget::Id(seen.id()));
    frames.idle(2, &mut *show);
}

/// Rows with a toggle cell in the column `MUTE`.
struct Switches {
    on: RefCell<Vec<bool>>,
}

impl DisplayOrder<usize> for Switches {
    fn len(&self) -> usize {
        self.on.borrow().len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.len()).then_some(*key)
    }
}

impl ItemSource<usize> for Switches {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == MUTE && self.on.borrow()[at]
    }
}

fn show_table(ui: &mut Ui, props: CollectionProps, source: &dyn ItemSource<usize>, seen: &Seen) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        seen.take(Table::new(props).id_salt("table").show_items(ui, source, &mut |key, cx| {
            seen.note(*key, cx.rect);
            Row::new()
        }));
    });
}

/// Space flips the lead row's toggle cell when the props say so, in one edit; Enter still activates.
#[trial]
fn space_toggles_the_lead_rows_cell_and_enter_activates() {
    let (seen, source) = (Seen::default(), Switches { on: RefCell::new(vec![false; 3]) });
    let props = || {
        CollectionProps::new().select(SelectMode::Single).space(SpaceAction::ToggleCell).columns(vec![
            ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
            ColumnSpec::new(MUTE, "Mute", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
        ])
    };
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        for (key, on) in seen.toggles() {
            source.on.borrow_mut()[key] = on;
        }
        show_table(ui, props(), &source, &seen);
    };
    frames.idle(3, &mut show);
    focus(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    press(&mut frames, Key::Space, &mut show);
    assert_eq!(seen.toggles(), vec![(0, true)], "one Toggle edit on the lead row");
    assert!(seen.activations().is_empty(), "Space does not activate");
    press(&mut frames, Key::Enter, &mut show);
    assert_eq!(seen.activations(), vec![0]);
    assert_eq!(seen.toggles().len(), 1, "Enter toggles nothing");
}

/// A collection that lays its items in rows of different lengths, the last items hidden as a closed group's are.
#[derive(Clone)]
struct Ragged {
    rows: Vec<usize>,
    hidden_from: usize,
}

impl Ragged {
    /// The row and column of item `at`.
    fn place(&self, at: usize) -> (usize, usize) {
        let mut start = 0;
        for (row, len) in self.rows.iter().enumerate() {
            if at < start + len {
                return (row, at - start);
            }
            start += len;
        }
        (self.rows.len(), 0)
    }

    fn count(&self) -> usize {
        self.rows.iter().sum()
    }
}

impl ItemLayout for Ragged {
    fn content_size(&self) -> ScrollPos {
        let widest = self.rows.iter().copied().max().unwrap_or(0);
        ScrollPos { x: f64::from(widest as f32 * TILE.x), y: f64::from(self.rows.len() as f32 * TILE.y) }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        let view = Rect::from_min_size(Pos2::ZERO, view);
        out.extend((0..self.count()).filter(|&at| self.shown(at) && self.rect(at, offset).intersects(view)));
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        if !self.shown(at) {
            return Rect::NOTHING;
        }
        let (row, column) = self.place(at);
        Rect::from_min_size(Pos2::new(column as f32 * TILE.x - offset.x as f32, row as f32 * TILE.y - offset.y as f32), TILE)
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        (0..self.count()).find(|&at| self.shown(at) && self.rect(at, offset).contains(point))
    }

    fn nav(&self) -> NavShape {
        NavShape::RaggedGrid
    }

    fn row_len(&self, row: usize) -> Option<usize> {
        self.rows.get(row).copied()
    }

    fn shown(&self, at: usize) -> bool {
        at < self.hidden_from
    }
}

/// A collection over `Ragged`, handing the digits it receives to the trial.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
struct Probe<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    layout: Ragged,
    digits: &'a RefCell<Vec<u8>>,
}

impl<'a> Probe<'a> {
    fn new(items: CollectionProps, layout: Ragged, digits: &'a RefCell<Vec<u8>>) -> Self {
        Probe { common: Common::new(), props: ContainerProps::new().sizing(craft_ui::contract::Sizing::Fill), items, layout, digits }
    }
}

impl Component for Probe<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: ProbeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        while let Some(digit) = cx.intents().take_digit() {
            self.digits.borrow_mut().push(digit);
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Probe<'_> {
    type Key = usize;
    type ItemLayout = Ragged;
    type Selection = Selection<usize>;
    type Item<'r> = Row;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, _: &dyn ItemSource<usize>, _: f32, _: &StyleCx<'_>) -> Ragged {
        self.layout.clone()
    }
}

fn show_probe(ui: &mut Ui, props: CollectionProps, layout: &Ragged, source: &dyn ItemSource<usize>, seen: &Seen) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        seen.take(Probe::new(props, layout.clone(), &seen.digits).id_salt("probe").show_items(ui, source, &mut |key, cx| {
            seen.note(*key, cx.rect);
            Row::new()
        }));
    });
}

/// Down in a ragged grid goes to the item below, which needs the layout's row lengths.
#[trial]
fn down_in_a_ragged_grid_moves_to_the_item_below() {
    let (seen, layout) = (Seen::default(), Ragged { rows: vec![3, 2, 3], hidden_from: 8 });
    let source = Count(8);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_probe(ui, CollectionProps::new().select(SelectMode::Single), &layout, &source, &seen);
    frames.idle(3, &mut show);
    focus(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    press(&mut frames, Key::ArrowRight, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    assert_eq!(seen.lead_keys(), vec![0, 1, 4], "the second item of the first row goes to the second item of the second");
}

/// End lands on the last item that shows, and no item of a closed group is built with an unplaced rect.
#[trial]
fn end_skips_the_items_of_a_closed_group() {
    let (seen, layout) = (Seen::default(), Ragged { rows: vec![3, 2, 3], hidden_from: 5 });
    let source = Count(8);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_probe(ui, CollectionProps::new().select(SelectMode::Single), &layout, &source, &seen);
    frames.idle(3, &mut show);
    focus(&mut frames, &seen, &mut show);
    press(&mut frames, Key::End, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    assert_eq!(seen.lead_keys(), vec![4], "End takes the last shown item, and Down past it stays");
    let rows = seen.rows.borrow();
    assert!(rows.iter().all(|(key, rect)| *key < 5 && rect.is_finite()), "only shown items are built: {rows:?}");
}

/// A collection that takes digits receives them, and the other typed text still moves the lead by type-ahead.
#[trial]
fn digit_keys_leave_the_letters_to_type_ahead() {
    let (seen, layout) = (Seen::default(), Ragged { rows: vec![3], hidden_from: 3 });
    let words = Words::flat(&["apple", "banana", "cherry"]);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_probe(ui, CollectionProps::new().select(SelectMode::Single).digit_keys(true), &layout, &words, &seen);
    frames.idle(3, &mut show);
    focus(&mut frames, &seen, &mut show);
    frames.frame(vec![input::text("3")], &mut show);
    assert_eq!(*seen.digits.borrow(), vec![3]);
    assert!(seen.lead_keys().is_empty(), "a digit is not type-ahead");
    frames.frame(vec![input::text("c")], &mut show);
    assert_eq!(seen.lead_keys(), vec![2]);
    assert_eq!(*seen.digits.borrow(), vec![3], "a letter is not a digit");
}

/// Without the header flag the columns lay the rows out and no header row is built.
#[trial]
fn header_off_builds_no_header_row() {
    let columns = || vec![ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)).sortable()];
    let headers = |on: bool| {
        let (seen, source) = (Seen::default(), Count(3));
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_table(ui, CollectionProps::new().columns(columns()).header(on), &source, &seen);
        frames.idle(3, &mut show);
        frames.tree().with_role(Role::COLUMN_HEADER).len()
    };
    assert_eq!(headers(true), 1);
    assert_eq!(headers(false), 0);
}

/// The lead's events say whether keys or the pointer moved it.
#[trial]
fn the_lead_reports_what_moved_it() {
    let (seen, source) = (Seen::default(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, CollectionProps::new().select(SelectMode::Single).lead_follows_hover(true), &source, &seen);
    frames.idle(3, &mut show);
    focus(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    assert_eq!(seen.leads(), vec![(0, LeadSource::Keys)]);
    let (first, second) = (seen.row(1).center(), seen.row(2).center());
    frames.frame(vec![pointer::moved(first)], &mut show);
    frames.frame(vec![pointer::moved(second)], &mut show);
    assert_eq!(seen.leads().last(), Some(&(2, LeadSource::Pointer)));
    assert!(seen.events.borrow().iter().any(|event| matches!(event, CollectionEvent::Selection(SelectionChange::Set(_)))));
}

/// With `arrows_select` off the arrows move the lead and select nothing; Space then selects the lead.
#[trial]
fn arrows_move_the_lead_without_selecting_when_the_props_say_so() {
    let (seen, source) = (Seen::default(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, CollectionProps::new().select(SelectMode::Single).arrows_select(false), &source, &seen);
    frames.idle(3, &mut show);
    focus(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    press(&mut frames, Key::ArrowDown, &mut show);
    assert_eq!(seen.leads(), vec![(0, LeadSource::Keys), (1, LeadSource::Keys)]);
    let selections = |seen: &Seen| seen.events.borrow().iter().filter(|event| matches!(event, CollectionEvent::Selection(_))).count();
    assert_eq!(selections(&seen), 0, "the arrows only moved the lead");
    press(&mut frames, Key::Space, &mut show);
    assert_eq!(selections(&seen), 1, "Space selects the lead");
}

/// The first press on a column that sorts descending first asks for descending; the next one reverses it.
#[trial]
fn the_first_click_on_a_first_descending_column_sorts_descending() {
    let (seen, source) = (Seen::default(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let sort = seen.sorts().last().map(|(key, descending)| craft_ui::contract::ColumnSort::new(key.clone(), *descending));
        let columns = vec![ColumnSpec::new(SIZE, "Size", ColumnWidth::Fixed(COL)).sortable().first_descending()];
        show_table(ui, CollectionProps::new().columns(columns).sort(sort), &source, &seen);
    };
    frames.idle(3, &mut show);
    let header = Pos2::new(ORIGIN.x + COL / 2.0, (ORIGIN.y + seen.row(0).min.y) / 2.0);
    pointer::click(&mut frames, header, &mut show);
    frames.idle(2, &mut show);
    pointer::click(&mut frames, header, &mut show);
    assert_eq!(seen.sorts(), vec![(SIZE, true), (SIZE, false)]);
}
