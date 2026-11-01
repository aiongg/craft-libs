//! Tests of the Levels panel (task AD-1).

use egui::accesskit::Role;
use egui::{Key, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{Levels, LevelsPanel, ToneChannels, ToneSpace};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

const IDENTITY: Levels = Levels { in_black: 0.0, gamma: 1.0, in_white: 255.0, out_black: 0.0, out_white: 255.0 };

/// A host of RGB levels that applies every value and restores the start value on a cancel.
struct Host {
    levels: Vec<Levels>,
    histograms: Vec<[u32; 256]>,
    before: Option<Levels>,
    events: Vec<(usize, EditEvent<Levels>)>,
}

fn rig(levels: Vec<Levels>, histograms: Vec<[u32; 256]>) -> Harness<'static, Host> {
    let state = Host { levels, histograms, before: None, events: Vec::new() };
    let mut h = Harness::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let out = LevelsPanel::new("levels-panel", &host.levels, ToneChannels::of(ToneSpace::Rgb, false), &numbers)
                .histograms(&host.histograms)
                .show(ui);
            let Some(event) = out.edit.event else { return };
            match &event {
                EditEvent::Begin { .. } => host.before = host.levels.get(out.channel).copied(),
                EditEvent::Cancel { .. } => {
                    if let (Some(before), Some(slot)) = (host.before.take(), host.levels.get_mut(out.channel)) {
                        *slot = before;
                    }
                }
                other => {
                    if let (Some(&value), Some(slot)) = (other.value(), host.levels.get_mut(out.channel)) {
                        *slot = value;
                    }
                }
            }
            host.events.push((out.channel, event));
        },
        state,
    );
    h.run_steps(3);
    h
}

fn flat() -> Vec<[u32; 256]> {
    vec![[10; 256]; 4]
}

fn thumb(h: &Harness<'_, Host>) -> Pos2 {
    h.get_by_role_and_label(Role::Slider, "Input black").rect().center()
}

// Auto on RGB with a fake histogram reports one `Set` of the shown channel: inputs at the 0.1 % clips, gamma 1, outputs kept.
#[test]
fn levels_panel_auto_sets_black_and_white_inputs_from_a_tenth_of_a_percent_clip() {
    let mut composite = [0u32; 256];
    composite[0] = 10;
    composite[1] = 1;
    composite[254] = 11;
    composite[255] = 5;
    composite[100] = 9973;
    let mut histograms = flat();
    histograms[0] = composite;
    let start = Levels { in_black: 5.0, gamma: 2.0, in_white: 200.0, out_black: 10.0, out_white: 240.0 };
    let mut h = rig(vec![start; 4], histograms);
    h.get_by_role_and_label(Role::Button, "Auto").click();
    h.run_steps(3);
    // The total is 10000, so the clip is 10: bin 0 clips, bin 1 would pass it; the top bin 255 clips, 254 would pass it.
    let want = Levels { in_black: 1.0, gamma: 1.0, in_white: 254.0, out_black: 10.0, out_white: 240.0 };
    assert_eq!(h.state().events, [(0, EditEvent::set(want, egui::Modifiers::NONE))]);
}

// Picking Red in the Channel select and dragging a thumb reports channel 1 with Red's levels.
#[test]
fn levels_panel_reports_the_shown_channel_with_the_edit() {
    let red = Levels { in_black: 20.0, ..IDENTITY };
    let mut h = rig(vec![IDENTITY, red, IDENTITY, IDENTITY], flat());
    h.get_by_role_and_label(Role::ComboBox, "Channel").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Red")).click();
    h.run_steps(3);
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, "Input black").accesskit_node().numeric_value(), Some(20.0), "Red's levels show");

    let from = thumb(&h);
    h.hover_at(from);
    h.step();
    h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    h.step();
    let to = pos2(from.x + 30.0, from.y);
    h.hover_at(to);
    h.step();
    h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
    h.run_steps(3);
    let events = &h.state().events;
    assert!(!events.is_empty() && events.iter().all(|(channel, _)| *channel == 1), "{events:?}");
    assert!(h.state().levels[1].in_black > 20.0, "Red's levels changed");
    assert_eq!(h.state().levels[0], IDENTITY, "the composite did not");
}

// Esc during a thumb drag reports `Cancel`; the panel's value is unchanged (the host restores).
#[test]
fn levels_panel_cancel_event_does_not_change_the_value() {
    let mut h = rig(vec![IDENTITY; 4], flat());
    let from = thumb(&h);
    h.hover_at(from);
    h.step();
    h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    h.step();
    let to = pos2(from.x + 40.0, from.y);
    h.hover_at(to);
    h.step();
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(matches!(h.state().events.last(), Some((0, EditEvent::Cancel { .. }))), "{:?}", h.state().events);
    assert_eq!(h.state().levels[0], IDENTITY, "the host restored what it kept at `Begin`");
}

// `Levels::auto` copies PhotoCraft's loops: clip = total / 1000, lo < 253, hi > lo + 2 (an all-empty histogram gives 253 and 255).
#[test]
fn levels_auto_follows_the_clip_rule_at_both_ends() {
    let start = Levels { in_black: 3.0, gamma: 0.5, in_white: 100.0, out_black: 7.0, out_white: 9.0 };
    let empty = start.auto(&[0; 256]);
    assert_eq!((empty.in_black, empty.in_white, empty.gamma), (253.0, 255.0, 1.0));
    assert_eq!((empty.out_black, empty.out_white), (7.0, 9.0), "the outputs stay");

    // Total 20000: clip 20. The bottom takes bins while the running sum stays <= 20, the top likewise.
    let mut counts = [0u32; 256];
    counts[0] = 20;
    counts[1] = 1;
    counts[255] = 12;
    counts[254] = 8;
    counts[253] = 1;
    counts[128] = 20000 - 42;
    let auto = start.auto(&counts);
    assert_eq!((auto.in_black, auto.in_white), (1.0, 253.0));
}
