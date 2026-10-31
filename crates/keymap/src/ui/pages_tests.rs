//! Shortcuts For areas, app tabs and the automation snapshot.

use egui::{Key as EguiKey, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::rows::tests::fixture;
use super::state::{Env, Target};
use super::text::Say;
use super::{Area, DialogState, DialogTab, Options, Outcome, ShortcutsDialog};
use crate::chord::{Chord, Platform};
use crate::command::CommandRef;

#[test]
// Area::photoshop puts File > Save in Application Menus, a panel-scoped command in Panel Menus and a Kind::Tool in Tools.
fn photoshop_areas_split_menus_panels_and_tools() {
    let areas = Area::photoshop();
    let labels: Vec<_> = areas.iter().map(|a| a.label).collect();
    assert_eq!(labels, ["Application Menus", "Panel Menus", "Tools"]);
    let km = fixture(Platform::Mac);
    let holding = |id: &str| -> Vec<&str> {
        let b = km.catalog().iter().find(|b| b.command.id == id).unwrap();
        areas.iter().filter(|a| (a.rows)(b)).map(|a| a.label).collect()
    };
    assert_eq!(holding("file.save"), ["Application Menus"]);
    assert_eq!(holding("timeline.loop"), ["Panel Menus"]);
    assert_eq!(holding("tool.marquee"), ["Tools"]);
}

struct Host {
    km: crate::Keymap,
    options: Options,
    state: DialogState,
    outcomes: Vec<Outcome>,
}

#[test]
// A second tab draws its body when chosen, Ctrl+Tab moves to it, and a body returning true reports Outcome::Changed.
fn app_tabs_switch_and_report_changes() {
    let host = Host { km: fixture(Platform::Windows), options: Options::default(), state: DialogState::new(), outcomes: Vec::new() };
    let mut h = Harness::builder().with_size(vec2(1000.0, 700.0)).build_ui_state(
        |ui, host: &mut Host| {
            let Host { km, options, state, outcomes } = host;
            let mut tabs = [DialogTab {
                id: "menus",
                label: "Menus".into(),
                body: Box::new(|ui| {
                    ui.label("Menu colours");
                    ui.button("Change colour").clicked()
                }),
            }];
            outcomes.extend(ShortcutsDialog::new(km).options(options).tabs(&mut tabs).ui(ui, state));
        },
        host,
    );
    h.run();
    assert!(h.query_by_label("Menu colours").is_none(), "Keyboard Shortcuts is the first tab");
    assert_eq!(h.state().state.snapshot(&h.state().km).map(|s| s.tab), Some("shortcuts".to_string()));

    h.get_by_label("Menus").click();
    h.run();
    assert!(h.query_by_label("Menu colours").is_some());
    assert_eq!(h.state().state.snapshot(&h.state().km).map(|s| s.tab), Some("menus".to_string()));

    h.key_press_modifiers(Modifiers::CTRL, EguiKey::Tab);
    h.run();
    assert!(h.query_by_label("Menu colours").is_none(), "Ctrl+Tab wraps to Keyboard Shortcuts");
    h.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, EguiKey::Tab);
    h.run();
    assert!(h.query_by_label("Menu colours").is_some(), "Ctrl+Shift+Tab goes back");

    h.get_by_label("Change colour").click();
    h.run();
    assert!(h.state().outcomes.iter().any(|o| matches!(o, Outcome::Changed)));
}

#[test]
// After recording a held chord the snapshot names the conflict's chord and holders; while recording, `recording` names the command.
fn snapshot_reports_conflict_and_recording() {
    let mut km = fixture(Platform::Mac);
    let options = Options::default();
    let mut env = Env { keymap: &mut km, sets: None, options: &options, tabs: None, text: Say::english() };
    let mut state = DialogState::new();
    assert_eq!(state.snapshot(env.keymap), None, "before the first frame");
    state.begin(&mut env);
    let export = CommandRef::from("file.export");

    state.record(&env, Target::Cell { command: export.clone(), old: None });
    let during = state.snapshot(env.keymap).unwrap();
    assert_eq!((during.recording, during.conflict), (Some(export.clone()), None));

    state.feed(&mut env, &[Chord::parse("Cmd+S").unwrap()], 0.0);
    let after = state.snapshot(env.keymap).unwrap();
    assert_eq!(after.conflict, Some(("Cmd+S".to_string(), vec![CommandRef::from("file.save")])));
    assert_eq!((after.recording, after.tab.as_str()), (None, "shortcuts"));
}
