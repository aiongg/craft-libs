//! Roving groups, arena members and part stops: what Tab reaches and what a component reads as focused.

use super::{control, focus, focused, id, tap};
use craft_behavior::{Input, Intent, NavShape, PartKey, Roving};
use craft_ui::runtime::focus::KeyUse;
use craft_ui::testing::focus::{
    AccessActionKind, Group, Stop, access_request, activated_member, active_part, intents_of, is_focused, register_group, register_node,
    register_stop, widget,
};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Ui};
use std::cell::{Cell, RefCell};

/// Skeleton `r8_member_focus::member_focus_moves_with_arrows_and_reads_through_the_focus_service`: a radio
/// group is one Tab stop that arrives at the selected member; arrows move focus between members and leave the
/// value; Space activates the focused member through the group; egui focus stays on the root.
#[trial]
fn a_roving_group_is_one_tab_stop_and_its_members_read_their_own_focus() {
    let mut frames = Frames::new();
    let reads = RefCell::new([false; 4]);
    let activated = Cell::new((false, None));
    let mut show = |ui: &mut Ui| {
        control(ui, "before");
        widget(ui, id("group"));
        for member in ["m1", "m2", "m3"] {
            register_stop(ui.ctx(), Stop::new(id(member), KeyUse::of(Input::Pointer)).root(id("group")).in_group(id("group")));
        }
        register_group(ui.ctx(), Group::new(id("group"), Roving::new(NavShape::Flat)).selected((id("m2"), None)));
        let ctx = ui.ctx();
        *reads.borrow_mut() = ["m1", "m2", "m3", "group"].map(|name| is_focused(ctx, id(name)));
        let mut group = intents_of(ctx, Some(id("group")), false);
        if group.take(Intent::Activate) {
            activated.set((true, activated_member(ctx, id("group"))));
        }
        let member_moves = intents_of(ctx, Some(id("m3")), false).take_move();
        assert!(member_moves.is_none(), "a member gets no move: the group's roving step takes it");
    };
    frames.idle(2, &mut show);
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("before")));
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("group")), "Tab reaches the group as one stop");
    assert_eq!(*reads.borrow(), [false, true, false, false], "Tab lands on the selected member, not the root");
    tap(&mut frames, Key::ArrowRight, Modifiers::NONE, &mut show);
    assert_eq!(*reads.borrow(), [false, false, true, false]);
    assert_eq!(focused(&frames.ctx), Some(id("group")), "egui focus stays on the arena root");
    tap(&mut frames, Key::Home, Modifiers::NONE, &mut show);
    assert_eq!(*reads.borrow(), [true, false, false, false], "Home goes to the first member");
    tap(&mut frames, Key::ArrowRight, Modifiers::NONE, &mut show);
    tap(&mut frames, Key::ArrowRight, Modifiers::NONE, &mut show);
    tap(&mut frames, Key::Space, Modifiers::NONE, &mut show);
    assert_eq!(activated.get(), (true, Some(id("m3"))), "Space activates the focused member through the group");
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("before")), "Tab leaves the group, wrapping to the first stop");
    assert_eq!(*reads.borrow(), [false; 4], "focus leaving the group clears every member's focus");
}

/// Skeleton `frames_state::tab_into_a_component_sets_its_first_part_active`: Tab arrival sets the first part
/// of a component with focusable parts active, and the component reads as focused.
#[trial]
fn tab_into_a_component_sets_its_first_part_active() {
    const KNOB: PartKey = PartKey::new("knob");
    let mut frames = Frames::new();
    let read = Cell::new((false, None));
    let mut show = |ui: &mut Ui| {
        control(ui, "before");
        widget(ui, id("pair"));
        for part in 0..2 {
            register_stop(ui.ctx(), Stop::new(id("pair"), KeyUse::of(Input::Pointer)).part(KNOB.at(part)));
        }
        read.set((is_focused(ui.ctx(), id("pair")), active_part(ui.ctx(), id("pair"))));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "before", &mut show);
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(read.get(), (true, Some(KNOB.at(0))));
}

fn members(ui: &mut Ui, names: &[&str], group: Group) {
    widget(ui, id("group"));
    for member in names {
        register_stop(ui.ctx(), Stop::new(id(member), KeyUse::of(Input::Pointer)).root(id("group")).in_group(id("group")));
    }
    register_group(ui.ctx(), group);
}

fn focused_members(ctx: &egui::Context, names: &[&str]) -> Vec<String> {
    names.iter().filter(|name| is_focused(ctx, id(name))).map(|name| name.to_string()).collect()
}

/// 9.2: a physical arrow reads in the receiving group's direction.
#[trial]
fn a_right_to_left_group_reads_horizontal_arrows_in_its_direction() {
    const ALL: [&str; 3] = ["m1", "m2", "m3"];
    for (rtl, right, left) in [(false, "m3", "m1"), (true, "m1", "m3")] {
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| members(ui, &ALL, Group::new(id("group"), Roving::new(NavShape::Flat)).selected((id("m2"), None)).rtl(rtl));
        frames.idle(2, &mut show);
        tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
        assert_eq!(focused_members(&frames.ctx, &ALL), ["m2"]);
        tap(&mut frames, Key::ArrowRight, Modifiers::NONE, &mut show);
        assert_eq!(focused_members(&frames.ctx, &ALL), [right], "ArrowRight, rtl {rtl}");
        tap(&mut frames, Key::ArrowLeft, Modifiers::NONE, &mut show);
        tap(&mut frames, Key::ArrowLeft, Modifiers::NONE, &mut show);
        assert_eq!(focused_members(&frames.ctx, &ALL), [left], "ArrowLeft, rtl {rtl}");
    }
}

/// 9.5: when the focused member of a group is no longer registered, focus goes to the next member.
#[trial]
fn focus_moves_to_the_next_member_when_the_focused_member_is_removed() {
    let present = RefCell::new(vec!["m1", "m2", "m3"]);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| members(ui, &present.borrow(), Group::new(id("group"), Roving::new(NavShape::Flat)).selected((id("m1"), None)));
    frames.idle(2, &mut show);
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    tap(&mut frames, Key::ArrowRight, Modifiers::NONE, &mut show);
    assert_eq!(focused_members(&frames.ctx, &["m1", "m2", "m3"]), ["m2"]);
    present.borrow_mut().retain(|name| *name != "m2");
    frames.idle(2, &mut show);
    assert_eq!(focused_members(&frames.ctx, &["m1", "m2", "m3"]), ["m3"]);
    assert_eq!(focused(&frames.ctx), Some(id("group")), "egui focus stays on the arena root");
}

/// 9.2 rule 1: an AccessKit click on a group member activates it through the group.
#[trial]
fn an_accesskit_click_on_a_group_member_activates_it_through_the_group() {
    let mut frames = Frames::new();
    let activated = Cell::new((false, None));
    let mut show = |ui: &mut Ui| {
        members(ui, &["m1", "m2", "m3"], Group::new(id("group"), Roving::new(NavShape::Flat)));
        register_node(ui.ctx(), id("m3"), id("m3"), None, true);
        let ctx = ui.ctx();
        if intents_of(ctx, Some(id("group")), false).take(Intent::Activate) {
            activated.set((true, activated_member(ctx, id("group"))));
        }
    };
    frames.idle(2, &mut show);
    frames.frame(vec![access_request(AccessActionKind::Click, id("m3"))], &mut show);
    assert_eq!(activated.get(), (true, Some(id("m3"))));
}

/// 9.5: a group of parts of one component arrives at the part its registration names, and at the new one after the
/// selection moves.
#[trial]
fn tab_arrives_at_the_selected_part_of_a_part_group() {
    const KNOB: PartKey = PartKey::new("knob");
    let selected = Cell::new(1u64);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        control(ui, "before");
        widget(ui, id("row"));
        for part in 0..3 {
            register_stop(ui.ctx(), Stop::new(id("row"), KeyUse::of(Input::Pointer)).part(KNOB.at(part)).in_group(id("row-group")));
        }
        let arrival = (id("row"), Some(KNOB.at(selected.get())));
        register_group(ui.ctx(), Group::new(id("row-group"), Roving::new(NavShape::Flat)).selected(arrival));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "before", &mut show);
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(active_part(&frames.ctx, id("row")), Some(KNOB.at(1)), "Tab arrives at the selected part, not the first");
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    selected.set(2);
    frames.idle(2, &mut show);
    tap(&mut frames, Key::Tab, Modifiers::SHIFT, &mut show);
    assert_eq!(active_part(&frames.ctx, id("row")), Some(KNOB.at(2)), "after the selection moves Tab arrives at the new part");
}
