//! A host that reads a toggle column through `SweepApply`: each crossed switch applies live, Escape or a lost press
//! capture restores them all, and an Alt sweep shows only its first cell once.

use super::kit::Row;
use super::table::Table;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, ItemSource};
use craft_ui::contract::{
    CollectionEvent, CollectionProps, ColumnKind, ColumnSpec, Props as _, ShowCollection as _, SweepApply, SweepStep, ToggleColumn,
};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, input, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 240.0);
const COL: f32 = 100.0;
const NAME: ColumnKey = ColumnKey::new("name");
const EYE: ColumnKey = ColumnKey::new("eye");

/// Rows whose eye state the host keeps.
struct Eyes(RefCell<Vec<bool>>);

impl DisplayOrder<usize> for Eyes {
    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.len()).then_some(*key)
    }
}

impl ItemSource<usize> for Eyes {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == EYE && self.0.borrow()[at]
    }
}

/// A table with an eye column and a host that applies the steps `SweepApply` gives.
struct Host {
    eyes: Eyes,
    apply: RefCell<SweepApply>,
    rows: RefCell<Vec<(usize, Rect)>>,
    show_only: RefCell<usize>,
    /// The state from before the last `ShowOnly`, which the host restores on `UndoShowOnly`.
    before: RefCell<Vec<bool>>,
    frames: RefCell<Frames>,
}

impl Host {
    fn new(state: &[bool]) -> Self {
        Self::with(state, SweepApply::new())
    }

    fn with(state: &[bool], apply: SweepApply) -> Self {
        Host {
            eyes: Eyes(RefCell::new(state.to_vec())),
            apply: RefCell::new(apply),
            rows: RefCell::default(),
            show_only: RefCell::new(0),
            before: RefCell::default(),
            frames: RefCell::new(Frames::new()),
        }
    }

    fn show(&self, ui: &mut Ui) {
        let props = CollectionProps::new().columns(vec![
            ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
            ColumnSpec::new(EYE, "Eye", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
        ]);
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = Table::new(props).id_salt("table").show_items(ui, &self.eyes, &mut |key, cx| {
                let mut rows = self.rows.borrow_mut();
                rows.retain(|(held, _)| held != key);
                rows.push((*key, cx.rect));
                Row::new()
            });
            for event in out.events {
                let CollectionEvent::Toggle(edit) = event else { continue };
                let mut eyes = self.eyes.0.borrow_mut();
                match self.apply.borrow_mut().feed(&edit) {
                    Some(SweepStep::Flip(toggle) | SweepStep::Restore(toggle)) => eyes[toggle.key] = toggle.on,
                    Some(SweepStep::ShowOnly(toggle)) => {
                        *self.show_only.borrow_mut() += 1;
                        *self.before.borrow_mut() = eyes.clone();
                        eyes.iter_mut().for_each(|on| *on = false);
                        eyes[toggle.key] = toggle.on;
                    }
                    Some(SweepStep::UndoShowOnly(_)) => *eyes = self.before.borrow().clone(),
                    None => {}
                    Some(_) => {}
                }
            }
        });
    }

    fn cell(&self, key: usize) -> Pos2 {
        let row = self.rows.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the row was shown");
        Pos2::new(ORIGIN.x + COL * 1.5, row.center().y)
    }

    fn frame(&self, events: Vec<egui::Event>) {
        self.frames.borrow_mut().frame(events, |ui| self.show(ui));
    }

    fn idle(&self, n: usize) {
        self.frames.borrow_mut().idle(n, |ui| self.show(ui));
    }

    fn state(&self) -> Vec<bool> {
        self.eyes.0.borrow().clone()
    }

    /// Presses the eye of row 0 and moves down through rows 1 and 2, with the pointer still down.
    fn sweep_down(&self) {
        self.idle(3);
        self.frame(vec![pointer::moved(self.cell(0))]);
        self.frame(vec![pointer::press(self.cell(0))]);
        self.frame(vec![pointer::moved(self.cell(1))]);
        self.frame(vec![pointer::moved(self.cell(2))]);
    }
}

/// Each switch the pointer crosses applies at once. Escape or a lost capture restores all of them, a release keeps
/// them; the gesture is known only while the pointer is down.
#[trial]
fn a_sweep_flips_each_switch_live_and_escape_restores_all() {
    for (end, kept) in [("escape", false), ("lost capture", false), ("release", true)] {
        let host = Host::new(&[false; 4]);
        assert_eq!(host.apply.borrow().gesture(), None);
        host.sweep_down();
        assert_eq!(host.state(), vec![true, true, true, false], "the crossed switches are on while the pointer is down");
        assert!(host.apply.borrow().gesture().is_some(), "a sweep runs");
        host.frame(vec![match end {
            "escape" => input::key_press(Key::Escape, Modifiers::NONE),
            "lost capture" => pointer::window_blurred(),
            _ => pointer::release(host.cell(2)),
        }]);
        host.idle(2);
        let want = if kept { vec![true, true, true, false] } else { vec![false; 4] };
        assert_eq!(host.state(), want, "{end}");
        assert_eq!(host.apply.borrow().gesture(), None, "{end} ended the gesture");
    }
}

/// An Alt sweep on a plain column ignores Alt: it flips every crossed row and Escape restores all of them.
#[trial]
fn an_alt_sweep_on_a_plain_column_flips_every_row_and_escape_restores_them() {
    let host = Host::new(&[false; 4]);
    host.frames.borrow_mut().hold(Modifiers::ALT);
    host.sweep_down();
    assert_eq!(host.state(), vec![true, true, true, false]);
    assert_eq!(*host.show_only.borrow(), 0, "a plain column never shows only");
    host.frame(vec![input::key_press(Key::Escape, Modifiers::NONE)]);
    host.idle(2);
    assert_eq!(host.state(), vec![false; 4]);
}

/// An Alt sweep on a show-only column shows only its first row, applying the rule once; a release keeps that, Escape
/// undoes it.
#[trial]
fn an_alt_sweep_shows_only_the_first_row_once() {
    for (escape, want) in [(false, [true, false, false, false]), (true, [false, true, true, true])] {
        let host = Host::with(&[false, true, true, true], SweepApply::show_only());
        host.frames.borrow_mut().hold(Modifiers::ALT);
        host.sweep_down();
        assert_eq!(host.state(), vec![true, false, false, false], "only the first row shows");
        assert_eq!(*host.show_only.borrow(), 1, "the rule applied once, not for every crossing");
        host.frame(vec![if escape { input::key_press(Key::Escape, Modifiers::NONE) } else { pointer::release(host.cell(2)) }]);
        host.idle(2);
        assert_eq!(host.state(), want, "escape: {escape}");
        assert_eq!(*host.show_only.borrow(), 1);
    }
}
