//! Drags that start on empty space or on a toggle cell: the marquee, and the cells a toggle press or sweep may touch.

use super::kit::{Count, Dismissed, Row};
use super::table::Table;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, ItemSource, Selection, SelectionModel};
use craft_ui::contract::{CollectionEvent, CollectionProps, ColumnKind, ColumnSpec, Props as _, SelectMode, ShowCollection as _, ToggleColumn};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, input, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 240.0);
const COL: f32 = 100.0;
const NAME: ColumnKey = ColumnKey::new("name");
const MUTE: ColumnKey = ColumnKey::new("mute");

/// What a collection reported and where its rows were.
#[derive(Default)]
struct Scene {
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    rows: RefCell<Vec<(usize, Rect)>>,
}

impl Scene {
    fn row(&self, key: usize) -> Rect {
        self.rows.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the row was shown")
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

    /// The selection the reported changes make, applied in order.
    fn selected(&self) -> Vec<usize> {
        let mut selection = Selection::<usize>::default();
        for event in self.events.borrow().iter() {
            if let CollectionEvent::Selection(change) = event {
                selection.apply(change.clone());
            }
        }
        selection.items
    }
}

fn show_table(ui: &mut Ui, props: CollectionProps, source: &dyn ItemSource<usize>, scene: &Scene) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let out = Table::new(props).id_salt("table").show_items(ui, source, &mut |key, cx| {
            let mut rows = scene.rows.borrow_mut();
            rows.retain(|(held, _)| held != key);
            rows.push((*key, cx.rect));
            Row::new()
        });
        scene.events.borrow_mut().extend(out.events);
    });
}

/// A drag from the empty space under the rows up to the middle of row 2 of five.
fn sweep_up(frames: &mut Frames, scene: &Scene, show: &mut impl FnMut(&mut Ui)) {
    let x = ORIGIN.x + COL / 2.0;
    let from = Pos2::new(x, scene.row(4).max.y + 40.0);
    let to = Pos2::new(x, scene.row(2).center().y);
    pointer::drag(frames, from, to - from, show);
}

/// A drag from empty space selects the rows its rectangle touches; with Shift they join the selection the drag found.
#[trial]
fn a_marquee_from_empty_space_selects_the_rows_it_touches() {
    let (scene, source) = (Scene::default(), Count(5));
    let props = || CollectionProps::new().select(SelectMode::Multi).marquee(true);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, props(), &source, &scene);
    frames.idle(3, &mut show);
    sweep_up(&mut frames, &scene, &mut show);
    assert_eq!(scene.selected(), vec![2, 3, 4]);

    pointer::click(&mut frames, scene.row(0).center(), &mut show);
    assert_eq!(scene.selected(), vec![0]);
    frames.hold(Modifiers::SHIFT);
    sweep_up(&mut frames, &scene, &mut show);
    assert_eq!(scene.selected(), vec![0, 2, 3, 4]);
}

/// Rows with a toggle cell, some with a locked cell and some section headers.
struct Switches {
    on: RefCell<Vec<bool>>,
    locked: Vec<usize>,
    sections: Vec<usize>,
}

impl Switches {
    fn new(rows: usize, locked: &[usize], sections: &[usize]) -> Self {
        Switches { on: RefCell::new(vec![false; rows]), locked: locked.to_vec(), sections: sections.to_vec() }
    }
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

    fn is_section(&self, at: usize) -> bool {
        self.sections.contains(&at)
    }
}

impl ItemSource<usize> for Switches {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == MUTE && self.on.borrow()[at]
    }

    fn toggle_enabled(&self, at: usize, _: &ColumnKey) -> bool {
        !self.locked.contains(&at)
    }
}

/// The toggles reported by a drag from row `from` to row `to` in the toggle column of a six-row table.
fn toggle_drag(source: &Switches, from: usize, to: usize) -> Vec<(usize, bool)> {
    let scene = Scene::default();
    let props = || {
        CollectionProps::new().columns(vec![
            ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
            ColumnSpec::new(MUTE, "Mute", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
        ])
    };
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        for (key, on) in scene.toggles() {
            source.on.borrow_mut()[key] = on;
        }
        show_table(ui, props(), source, &scene);
    };
    frames.idle(3, &mut show);
    let cell = |key: usize| Pos2::new(ORIGIN.x + COL * 1.5, scene.row(key).center().y);
    pointer::drag(&mut frames, cell(from), cell(to) - cell(from), &mut show);
    let events = scene.events.borrow();
    let phase = |wanted: craft_behavior::GesturePhase| {
        events.iter().filter(|event| matches!(event, CollectionEvent::Toggle(edit) if edit.phase() == Some(wanted))).count()
    };
    assert_eq!(phase(craft_behavior::GesturePhase::Commit), phase(craft_behavior::GesturePhase::Begin), "every flip commits once, on release");
    drop(events);
    scene.toggles()
}

/// A locked cell neither starts a sweep nor takes one; a section row's toggle column starts none.
#[trial]
fn locked_cells_and_section_rows_take_no_toggle_press_or_sweep() {
    assert!(toggle_drag(&Switches::new(6, &[0], &[]), 0, 3).is_empty(), "a press on a locked cell does nothing and sweeps nothing");
    assert_eq!(toggle_drag(&Switches::new(6, &[2], &[]), 1, 3), vec![(1, true), (3, true)], "a sweep passes over a locked cell without setting it");
    assert!(toggle_drag(&Switches::new(6, &[], &[0]), 0, 3).is_empty(), "a section row's toggle column starts no sweep");
}

#[trial]
fn toggle_cells_carry_a_name_and_exclusive_columns_are_radios() {
    {
        use craft_behavior::IconId;
        use craft_ui::contract::Role;
        let mut fixture = craft_ui::testing::Fixture::new(|ui| {
            let source = Switches::new(3, &[], &[]);
            let props = CollectionProps::new().columns(vec![
                ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
                ColumnSpec::new(MUTE, "", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())).icon(IconId::new("eye")),
                ColumnSpec::new(ColumnKey::new("solo"), "", ColumnWidth::Fixed(COL))
                    .kind(ColumnKind::Toggle(ToggleColumn::exclusive()))
                    .icon(IconId::new("solo")),
            ]);
            in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
                Table::new(props).id_salt("table").show_items(ui, &source, &mut |_, _| Row::new());
            });
        });
        fixture.settle();
        assert_eq!(fixture.by_role(Role::SWITCH).len(), 3, "the sweepable column's cells are switches");
        assert_eq!(fixture.by_role(Role::RADIO_BUTTON).len(), 3, "the exclusive column's cells are radios");
        assert!(fixture.by_label("eye").is_some(), "an icon-only column names its cells after the icon");
        assert!(fixture.by_label("solo").is_some());
    }
}
