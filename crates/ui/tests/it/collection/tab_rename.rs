//! Tab in a rename editor commits it and continues on the next renamable row.

use super::kit::{Dismissed, List, Row, Words};
use craft_behavior::{CancelReason, EditEvent, Mixed};
use craft_ui::contract::{CollectionEvent, CollectionResponse, Props as _, SelectMode, ShowCollection as _};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, input, trial};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);

#[derive(Default)]
struct Seen {
    id: Cell<Option<Id>>,
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
}

impl Seen {
    fn take(&self, out: CollectionResponse<usize, Dismissed>) {
        self.id.set(Some(out.response.id));
        self.events.borrow_mut().extend(out.events);
    }

    /// The rename events in order: `begin <row>`, `commit <row> <name>`, `cancel <reason>`.
    fn renames(&self) -> Vec<String> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                CollectionEvent::Rename(EditEvent::Begin { before: Mixed::Same(before), .. }) => Some(format!("begin {}", before.key)),
                CollectionEvent::Rename(EditEvent::Commit { after, .. }) => Some(format!("commit {} {}", after.key, after.name)),
                CollectionEvent::Rename(EditEvent::Cancel { reason, .. }) => Some(format!("cancel {reason:?}")),
                _ => None,
            })
            .collect()
    }
}

/// Five rows, the second of which cannot be renamed.
fn show_rows(ui: &mut Ui, words: &Words, seen: &Seen) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let list = List::new(SelectMode::Single).rename().text_editor().locked(&[1]);
        seen.take(list.id_salt("list").show_items(ui, words, &mut |_, _| Row::new()));
    });
}

fn press(frames: &mut Frames, key: Key, mods: Modifiers, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![input::key_press(key, mods)], &mut *show);
    frames.frame(vec![craft_ui::testing::focus::key_release(key, mods)], &mut *show);
    frames.idle(2, &mut *show);
}

fn rename_row(frames: &mut Frames, seen: &Seen, row: usize, show: &mut impl FnMut(&mut Ui)) {
    frames.idle(2, &mut *show);
    craft_ui::contract::request_rename(&frames.ctx, seen.id.get().expect("the list ran"), &row);
    frames.idle(2, &mut *show);
}

/// Tab commits the edit and opens the next renamable row, Shift+Tab the previous one, a locked row is skipped, and
/// Escape in the continued editor cancels it.
#[trial]
fn tab_continues_the_rename_on_the_next_renamable_row() {
    {
        let words = Words::flat(&["a", "b", "c", "d", "e"]);
        let seen = Seen::default();
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_rows(ui, &words, &seen);
        rename_row(&mut frames, &seen, 0, &mut show);
        frames.frame(vec![Event::Text("!".into())], &mut show);
        press(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
        assert_eq!(seen.renames(), ["begin 0", "commit 0 a!", "begin 2"], "one commit, then the editor of the row after the locked one");
        frames.frame(vec![Event::Text("?".into())], &mut show);
        press(&mut frames, Key::Tab, Modifiers::SHIFT, &mut show);
        assert_eq!(seen.renames()[3..], ["commit 2 c?", "begin 0"], "Shift+Tab goes back over the locked row");
        press(&mut frames, Key::Escape, Modifiers::NONE, &mut show);
        assert_eq!(seen.renames()[5..], [format!("cancel {:?}", CancelReason::Escape)], "Escape cancels with no commit");
        assert_eq!(seen.renames().len(), 6);
    }
    {
        let words = Words::flat(&["a", "b", "c"]);
        let seen = Seen::default();
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_rows(ui, &words, &seen);
        rename_row(&mut frames, &seen, 2, &mut show);
        frames.frame(vec![Event::Text("!".into())], &mut show);
        press(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
        assert_eq!(seen.renames(), ["begin 2", "commit 2 c!"]);
        assert_eq!(craft_ui::testing::focus::egui_focused(&frames.ctx), seen.id.get(), "focus returns to the list");
    }
}
