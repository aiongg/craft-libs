//! A layer's root node and its anchor's popup relations.

use super::kit::{Opened, Opener, Seen};
use craft_ui::contract::{Overlay, PopupKind, Props as _, Show as _, Trigger};
use craft_ui::testing::{Frames, trial};
use egui::{Id, Pos2, Rect, Ui};
use std::cell::{Cell, RefCell};

const OPENER: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(180.0, 120.0) };

/// Opens a popup of `make`, then closes it; returns the anchor's tree facts with it open and closed.
fn anchor_facts(make: fn() -> Overlay) -> (Facts, Facts) {
    let mut frames = Frames::new();
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    let (open, close) = (Cell::new(false), Cell::new(false));
    let anchor = Cell::new(Id::NULL);
    let mut show = |ui: &mut Ui| {
        let opener = Opener::new(&opened, &content, Trigger::Manual, make).open(open.get()).close(close.get());
        anchor.set(opener.id_salt("opener").show_at(ui, OPENER).id);
    };
    frames.idle(3, &mut show);
    open.set(true);
    frames.frame(vec![], &mut show);
    open.set(false);
    frames.idle(30, &mut show);
    let while_open = facts(&mut frames, anchor.get());
    close.set(true);
    frames.frame(vec![], &mut show);
    close.set(false);
    frames.idle(30, &mut show);
    let after = facts(&mut frames, anchor.get());
    (while_open, after)
}

struct Facts {
    has_popup: Option<PopupKind>,
    expanded: Option<bool>,
    /// For each controlled node: whether it is modal.
    controls: Vec<bool>,
}

fn facts(frames: &mut Frames, anchor: Id) -> Facts {
    let tree = frames.tree();
    Facts {
        has_popup: tree.has_popup(anchor),
        expanded: tree.expanded(anchor),
        controls: tree.controls(anchor).into_iter().map(|key| tree.is_modal(key)).collect(),
    }
}

#[trial]
fn a_popup_anchor_has_the_kind_the_state_and_its_root_and_a_modal_root_is_modal() {
    let (open, closed) = anchor_facts(Overlay::dropdown);
    assert_eq!((open.has_popup, open.expanded, open.controls), (Some(PopupKind::Listbox), Some(true), vec![false]));
    assert_eq!((closed.expanded, closed.controls.len()), (Some(false), 0), "a closed anchor is collapsed and controls nothing");
    let (open, _) = anchor_facts(Overlay::menu);
    assert_eq!(open.has_popup, Some(PopupKind::Menu));
    let (open, _) = anchor_facts(Overlay::modal);
    assert_eq!((open.has_popup, open.controls), (Some(PopupKind::Dialog), vec![true]));
}
