//! Trailing items, modifiers on activations, column context requests, thumbnail drags and footer picks.

use crate::collection::kit::{Count, Dismissed, Row};
use crate::collection::table::Table;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, IconId, ItemSource};
use craft_keymap::Mods;
use craft_ui::contract::{CollectionEvent, CollectionProps, ColumnKind, ColumnSpec, Props as _, SelectMode, ShowCollection as _, ToggleColumn};
use craft_ui::list::{List, ListRow, RowEvent, ThumbSize, ThumbTarget, TrailingFace, TrailingPart};
use craft_ui::runtime::drag;
use craft_ui::sections::{FooterAction, FooterBar};
use craft_ui::testing::internals::place_row;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Frames, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PART: TrailingPart = TrailingPart::new(7);

struct Two;

impl DisplayOrder<usize> for Two {
    fn len(&self) -> usize {
        2
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Two {}

/// What a list of two rows reported: the rows' events and how many selection changes.
#[derive(Default)]
struct Seen {
    events: RefCell<Vec<RowEvent>>,
    selections: Cell<usize>,
}

/// A list of two rows made by `make`, shown in a fixture.
fn rig(make: impl Fn(usize) -> ListRow<'static> + 'static) -> (Fixture, Rc<Seen>) {
    let seen = Rc::new(Seen::default());
    let sink = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = List::new("Items").select(SelectMode::Single).show_items(ui, &Two, &mut |key, _| make(*key));
        for event in out.events {
            match event {
                CollectionEvent::Item(_, event) => sink.events.borrow_mut().push(event),
                CollectionEvent::Selection(_) => sink.selections.set(sink.selections.get() + 1),
                _ => {}
            }
        }
    });
    fixture.settle();
    (fixture, seen)
}

fn with_trailing(key: usize) -> ListRow<'static> {
    ListRow::new(format!("Row {key}")).trailing_icon(IconId::new("lock"), "Locked").trailing_part(PART, TrailingFace::disclosure(false), "Effects")
}

/// A trailing icon and a trailing part are nodes named by the caller.
#[trial]
fn a_trailing_icon_is_a_named_node() {
    let (fixture, _) = rig(with_trailing);
    assert!(fixture.by_label("Locked").is_some());
    assert!(fixture.by_label("Effects").is_some());
}

/// Required items stay in a narrow row while an optional text drops, and the label keeps its least width.
#[trial]
fn required_trailing_items_stay_and_optional_text_drops_first() {
    let row = || {
        ListRow::new("Layer")
            .trailing_icon(IconId::new("lock"), "Locked")
            .trailing_part(PART, TrailingFace::disclosure(true), "Effects")
            .optional("Multiply")
    };
    let wide = place_row(&row(), 400.0, false);
    assert!(wide.items.iter().all(Option::is_some), "everything fits in a wide row");
    let narrow = place_row(&row(), 130.0, false);
    assert!(narrow.items[0].is_some() && narrow.items[1].is_some(), "the icon and the part stay");
    assert!(narrow.items[2].is_none(), "the optional text dropped");
    assert!(narrow.label.width() >= narrow.min_label);
}

/// A press on a trailing part reports it with the modifiers held and leaves the selection alone.
#[trial]
fn a_press_on_a_trailing_part_reports_its_modifiers_and_keeps_the_selection() {
    let (mut fixture, seen) = rig(with_trailing);
    let part = fixture.by_label("Effects").expect("the part has a node");
    fixture.hold(Modifiers::ALT);
    fixture.click(&part);
    let events = seen.events.borrow();
    assert!(matches!(events.as_slice(), [RowEvent::Trailing { part, mods, .. }] if *part == PART && mods.alt), "{events:?}");
    assert_eq!(seen.selections.get(), 0);
}

/// Forward from the row reaches the part, and Enter activates it without modifiers.
#[trial]
fn forward_from_a_row_reaches_its_trailing_part() {
    let (mut fixture, seen) = rig(with_trailing);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    let events = seen.events.borrow();
    assert!(matches!(events.as_slice(), [RowEvent::Trailing { part, mods, .. }] if *part == PART && *mods == Mods::NONE), "{events:?}");
}

/// A row with a thumbnail whose rect `rect` records.
fn with_thumb(rect: Rc<Cell<Rect>>) -> impl Fn(usize) -> ListRow<'static> {
    move |key| {
        let rect = rect.clone();
        ListRow::new(format!("Row {key}")).thumb(ThumbSize::Medium, move |_, at| rect.set(at)).thumb_drag(ThumbTarget::First, key)
    }
}

/// A click on a thumbnail reports the modifiers held.
#[trial]
fn a_modified_thumbnail_click_reports_its_modifiers() {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let (mut fixture, seen) = rig(with_thumb(rect.clone()));
    fixture.hold(Modifiers::SHIFT);
    fixture.click_at(rect.get().center());
    let events = seen.events.borrow();
    assert!(events.iter().any(|event| matches!(event, RowEvent::ThumbClicked { target: ThumbTarget::First, mods, .. } if mods.shift)), "{events:?}");
}

/// A drag from a thumbnail with `thumb_drag` runs the drag session with its payload, and the row does not drag.
#[trial]
fn a_thumbnail_drag_starts_one_session_with_the_payload() {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let (mut fixture, seen) = rig(with_thumb(rect.clone()));
    let from = rect.get().center();
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + Vec2::new(0.0, 10.0 * step as f32))]);
    }
    assert_eq!(drag::started(fixture.ctx()), 1);
    assert!(drag::active::<usize>(fixture.ctx()).is_some(), "targets of the payload type see the drag");
    assert!(seen.events.borrow().iter().any(|event| matches!(event, RowEvent::ThumbDrag(ThumbTarget::First))));
}

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 240.0);
const COL: f32 = 100.0;
const NAME: ColumnKey = ColumnKey::new("name");
const MUTE: ColumnKey = ColumnKey::new("mute");

/// A table with a toggle column: the events it reported and where row 1 was.
#[derive(Default)]
struct Scene {
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    row: Cell<Rect>,
}

fn show_table(ui: &mut Ui, scene: &Scene) {
    let props = CollectionProps::new().columns(vec![
        ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
        ColumnSpec::new(MUTE, "Mute", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
    ]);
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let out = Table::new(props).id_salt("table").show_items(ui, &Count(3), &mut |key, cx| {
            if *key == 1 {
                scene.row.set(cx.rect);
            }
            Row::new()
        });
        scene.events.borrow_mut().extend(out.events);
    });
}

fn toggle_cell(scene: &Scene) -> Pos2 {
    Pos2::new(ORIGIN.x + COL * 1.5, scene.row.get().center().y)
}

/// A click on a toggle cell reports the modifiers held.
#[trial]
fn a_toggle_click_reports_its_modifiers() {
    let scene = Scene::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, &scene);
    frames.idle(3, &mut show);
    frames.hold(Modifiers::SHIFT);
    pointer::click(&mut frames, toggle_cell(&scene), &mut show);
    let mods: Vec<Mods> = scene
        .events
        .borrow()
        .iter()
        .filter_map(|event| if let CollectionEvent::Toggle(edit) = event { edit.value().map(|toggle| toggle.mods) } else { None })
        .collect();
    assert!(!mods.is_empty() && mods.iter().all(|mods| mods.shift), "{mods:?}");
}

/// A right-click on a toggle cell names its column in the context request.
#[trial]
fn a_context_request_on_a_toggle_cell_names_the_column() {
    let scene = Scene::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, &scene);
    frames.idle(3, &mut show);
    let at = toggle_cell(&scene);
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::secondary(at, true)], &mut show);
    frames.frame(vec![pointer::secondary(at, false)], &mut show);
    frames.idle(1, &mut show);
    let columns: Vec<Option<ColumnKey>> = scene
        .events
        .borrow()
        .iter()
        .filter_map(|event| if let CollectionEvent::ContextMenu(request) = event { Some(request.column.clone()) } else { None })
        .collect();
    assert_eq!(columns, [Some(MUTE)]);
}

/// A footer pick reports the modifiers held at the pick.
#[trial]
fn a_footer_pick_reports_its_modifiers() {
    let picked = Rc::new(RefCell::new(Vec::new()));
    let sink = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = FooterBar::actions([FooterAction::new(IconId::new("trash"), "Delete")]).show_footer(ui);
        if out.picked.is_some() {
            sink.borrow_mut().push(out.mods);
        }
    });
    fixture.settle();
    let target = fixture.by_label("Delete").expect("the action has a node");
    fixture.hold(Modifiers::ALT);
    fixture.click(&target);
    assert!(picked.borrow().iter().any(|mods: &Mods| mods.alt), "{:?}", picked.borrow());
}
