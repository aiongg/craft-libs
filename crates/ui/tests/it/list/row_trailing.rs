//! Trailing items, modifiers on activations, column context requests, thumbnail drags and footer picks.

use crate::collection::kit::{Count, Dismissed, Row};
use crate::collection::table::Table;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, IconId, ItemSource};
use craft_keymap::Mods;
use craft_ui::contract::{
    Bind, CollectionEvent, CollectionProps, ColumnKind, ColumnSpec, Props as _, ReorderRule, Role, SelectMode, ShowCollection as _, ToggleColumn,
};
use craft_ui::list::{List, ListRow, RowEvent, ThumbSize, ThumbTarget, TrailingFace, TrailingPart};
use craft_ui::runtime::drag;
use craft_ui::sections::{FooterAction, FooterBar};
use craft_ui::testing::internals::place_row;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Frames, pointer, trial};
use craft_ui::text::HexField;
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
    rig_with(false, make)
}

/// `rig` whose rows reorder when `reorder`.
fn rig_with(reorder: bool, make: impl Fn(usize) -> ListRow<'static> + 'static) -> (Fixture, Rc<Seen>) {
    let seen = Rc::new(Seen::default());
    let sink = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let list = List::new("Items").select(SelectMode::Single);
        let list = if reorder { list.reorder(ReorderRule::new()) } else { list };
        let out = list.show_items(ui, &Two, &mut |key, _| make(*key));
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

/// A disabled trailing part is a disabled node: a press on it reports nothing, and Forward from the row does not
/// enter it.
#[trial]
fn a_disabled_trailing_part_takes_no_press_and_no_focus() {
    let (mut fixture, seen) = rig(|key| with_trailing(key).trailing_enabled(PART, false));
    let part = fixture.by_label("Effects").expect("the part has a node");
    assert!(fixture.tree().is_disabled_key(part.node), "the node is disabled");
    fixture.click(&part);
    assert!(seen.events.borrow().is_empty(), "{:?}", seen.events.borrow());
    let row = on_a_row(&mut fixture);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(fixture.focused_stop(), Some(row), "Forward finds no part to enter");
}

/// A progress row describes its percentage for a screen reader and keeps its label's room.
#[trial]
fn a_progress_row_describes_its_percentage() {
    let (fixture, _) = rig(|key| ListRow::new(format!("Row {key}")).progress(0.42));
    let row = fixture.by_label("Row 0").expect("the row has a node");
    let described = fixture.tree().description_of(row.node).unwrap_or_default();
    assert!(described.contains("42"), "{described:?}");
}

/// Tab into the list and down to its second row: the stop focus is on.
fn on_a_row(fixture: &mut Fixture) -> (egui::Id, Option<craft_behavior::PartRef>) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    let row = fixture.focused_stop().expect("the list holds focus");
    assert!(row.1.is_none(), "focus is on the row, not a part");
    row
}

/// Forward from the row moves focus to its trailing part, and Back returns to the row.
#[trial]
fn forward_from_a_row_reaches_its_trailing_part() {
    let (mut fixture, _) = rig(with_trailing);
    let row = on_a_row(&mut fixture);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let part = fixture.focused_stop().expect("focus stays in the list");
    assert!(part.1.is_some(), "Forward entered the part: {part:?}");
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(fixture.focused_stop(), Some(row), "Back returns to the row");
}

/// Space and Enter on the focused trailing part activate it without modifiers.
#[trial]
fn space_and_enter_on_a_trailing_part_activate_it() {
    for key in [Key::Space, Key::Enter] {
        let (mut fixture, seen) = rig(with_trailing);
        on_a_row(&mut fixture);
        fixture.key(Key::ArrowRight, Modifiers::NONE);
        fixture.idle(2);
        fixture.key(key, Modifiers::NONE);
        fixture.idle(2);
        let events = seen.events.borrow();
        assert!(matches!(events.as_slice(), [RowEvent::Trailing { part, mods, .. }] if *part == PART && *mods == Mods::NONE), "{key:?}: {events:?}");
    }
}

/// Rows with parts add no Tab stops: Tab never lands on a part, and a second Tab does not step inside the list.
#[trial]
fn a_list_whose_rows_have_parts_is_one_tab_stop() {
    let (mut fixture, _) = rig(with_trailing);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    let list = fixture.focused_stop().expect("Tab enters the list");
    for _ in 0..3 {
        fixture.key(Key::Tab, Modifiers::NONE);
        fixture.idle(2);
        assert!(
            fixture.focused_stop().is_none_or(|stop| stop == list),
            "Tab stays on the list's one stop or leaves it: {:?}",
            fixture.focused_stop()
        );
    }
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
    let (mut fixture, seen) = rig_with(true, with_thumb(rect.clone()));
    let from = rect.get().center();
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + Vec2::new(0.0, 10.0 * step as f32))]);
    }
    assert_eq!(drag::started(fixture.ctx()), 1);
    let payload = drag::active::<usize>(fixture.ctx()).expect("targets of the payload type see the drag");
    assert_eq!(*payload, 1, "the session carries the thumbnail's payload, the key of the row the drag started on");
    assert!(seen.events.borrow().iter().any(|event| matches!(event, RowEvent::ThumbDrag(ThumbTarget::First))));
}

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 240.0);
const COL: f32 = 100.0;
const NAME: ColumnKey = ColumnKey::new("name");
const MUTE: ColumnKey = ColumnKey::new("mute");

/// A table with a toggle column: the events it reported and where row 1 was.
struct Scene {
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    row: Cell<Rect>,
}

impl Scene {
    fn new() -> Self {
        Scene { events: RefCell::new(Vec::new()), row: Cell::new(Rect::NOTHING) }
    }
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
    let scene = Scene::new();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, &scene);
    frames.idle(3, &mut show);
    frames.hold(Modifiers::SHIFT);
    pointer::click(&mut frames, toggle_cell(&scene), &mut show);
    let mods: Vec<Mods> = scene
        .events
        .borrow()
        .iter()
        .filter_map(|event| {
            if let CollectionEvent::Toggle(edit) = event {
                edit.value()
                    .filter(|_| matches!(edit, craft_behavior::EditEvent::Set { .. } | craft_behavior::EditEvent::Commit { .. }))
                    .map(|toggle| toggle.mods)
            } else {
                None
            }
        })
        .collect();
    assert!(!mods.is_empty() && mods.iter().all(|mods| mods.shift), "{mods:?}");
}

/// A right-click on a toggle cell names its column in the context request.
#[trial]
fn a_context_request_on_a_toggle_cell_names_the_column() {
    let scene = Scene::new();
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

/// A field hosted in a row takes the typing aimed at it, leaves the row's selection and focus model alone, and Tab
/// leaves the collection for the next stop after it.
#[trial]
fn a_hosted_field_takes_typing_and_tab_leaves_it() {
    let committed = Rc::new(Cell::new(None));
    let sink = committed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let sink = sink.clone();
        let _ = List::new("Items").select(SelectMode::Single).show_items(ui, &Two, &mut |key, _| {
            let sink = sink.clone();
            let row = ListRow::new(format!("Row {key}"));
            if *key == 0 {
                row.trailing_field(HexField::hex("Hex", Bind::given(craft_theme::Rgba::rgb(0, 0, 0))), move |out| {
                    if let Some(value) = out.value() {
                        sink.set(Some(*value));
                    }
                })
            } else {
                row
            }
        });
        let _ = craft_ui::buttons::Button::new("After the list").show(ui);
    });
    fixture.settle();
    let field = fixture.by_role(Role::TEXT_INPUT).into_iter().next().expect("the row hosts a field");
    fixture.click(&field);
    fixture.idle(2);
    fixture.type_text("ff0000");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(committed.get(), Some(craft_theme::Rgba::rgb(255, 0, 0)), "the typing reached the field");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    let after = fixture.by_label("After the list").expect("the control after the list has a node");
    let focused = fixture.focused().expect("Tab lands on a stop");
    assert_eq!(craft_ui::testing::access::NodeKey::of(focused), after.node, "Tab leaves the collection for the next control");
}

/// Forward from a row reaches the field hosted in it.
#[trial]
fn forward_from_a_row_enters_its_hosted_field() {
    let mut fixture = Fixture::new(move |ui| {
        let _ = List::new("Items").select(SelectMode::Single).show_items(ui, &Two, &mut |key, _| {
            let row = ListRow::new(format!("Row {key}"));
            if *key == 0 { row.trailing_field(HexField::hex("Hex", Bind::given(craft_theme::Rgba::rgb(0, 0, 0))), |_| {}) } else { row }
        });
    });
    fixture.settle();
    let field = fixture.by_role(Role::TEXT_INPUT).into_iter().next().expect("the row hosts a field");
    let row = fixture.by_label("Row 0").expect("the row has a node");
    fixture.click(&row);
    fixture.idle(2);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let focused = fixture.focused().expect("Forward lands on a stop");
    assert_eq!(craft_ui::testing::access::NodeKey::of(focused), field.node, "Forward from the row reaches the field");
}
