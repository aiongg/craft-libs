//! The LUT picker in a kittest harness: its rows, the wheel's previewed steps and Load 3D LUT….

use std::path::PathBuf;

use egui::accesskit::Role;
use egui::{Event, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{LutChoice, LutPicker};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

struct Host {
    value: LutChoice,
    folder: Option<PathBuf>,
    loads: usize,
    events: Vec<EditEvent<LutChoice>>,
}

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

fn picker(value: LutChoice, folder: Option<PathBuf>) -> Harness<'static, Host> {
    harness().build_ui_state(
        |ui, host: &mut Host| {
            let shown = LutPicker::new("lut", &host.value).folder(host.folder.clone()).show(ui);
            host.loads += usize::from(shown.load_requested);
            if let Some(event) = shown.edit.event {
                match &event {
                    EditEvent::Set { value, .. } | EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } => host.value = value.clone(),
                    EditEvent::Begin { value: Some(value), .. } => host.value = value.clone(),
                    _ => {}
                }
                host.events.push(event);
            }
        },
        Host { value, folder, loads: 0, events: Vec::new() },
    )
}

fn options(harness: &Harness<'_, Host>) -> Vec<String> {
    harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect()
}

/// The rows are Load, None, the built-ins, the folder files, then the loaded LUT.
#[test]
fn rows_follow_photocraft() {
    let dir = std::env::temp_dir().join(format!("craft-lut-rows-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["b.cube", "A.3dl", "notes.txt", ".hidden.cube"] {
        std::fs::write(dir.join(name), "x").unwrap();
    }
    let mut harness = picker(LutChoice::Loaded { name: "Film".into(), size: 33 }, Some(dir.clone()));
    // The scan runs on a worker; wait for its files to show.
    for _ in 0..200 {
        harness.run_steps(2);
        harness.get_by_role(Role::ComboBox);
        if options_after_open(&mut harness).contains(&"b.cube".to_owned()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let rows = options_after_open(&mut harness);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        rows,
        [
            "Load 3D LUT…",
            "None",
            "Warm Filter",
            "Cool Filter",
            "Teal & Orange",
            "Bleach Bypass",
            "Faded Film",
            "Day for Night",
            "Mono High Contrast",
            "Cross Process",
            "A.3dl",
            "b.cube",
            "Film (33³)",
        ]
    );
}

fn options_after_open(harness: &mut Harness<'_, Host>) -> Vec<String> {
    if options(harness).is_empty() {
        harness.get_by_role(Role::ComboBox).click();
        harness.run_steps(2);
    }
    options(harness)
}

/// The wheel steps through None and the built-ins only, as one previewed gesture.
#[test]
fn the_wheel_previews_built_ins_only() {
    let mut harness = picker(LutChoice::None, None);
    harness.run_steps(2);
    let at = harness.get_by_role(Role::ComboBox).rect().center();
    harness.hover_at(at);
    harness.run_steps(2);
    for _ in 0..2 {
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -1.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
        });
        harness.run_steps(2);
    }
    let events = &harness.state().events;
    assert!(
        matches!(
            events[..],
            [EditEvent::Begin { value: Some(LutChoice::Builtin("warm")), .. }, EditEvent::Update { value: LutChoice::Builtin("cool"), .. }]
        ),
        "{events:?}"
    );
    // Rest: the gesture commits with the last step.
    std::thread::sleep(std::time::Duration::from_millis(900));
    harness.run_steps(3);
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Commit { value: LutChoice::Builtin("cool"), changed: true, .. })), "{events:?}");
}

/// Load 3D LUT… asks the app and leaves the choice unchanged.
#[test]
fn load_asks_the_app() {
    let mut harness = picker(LutChoice::Builtin("warm"), None);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Load 3D LUT…").click();
    harness.run();
    assert_eq!(harness.state().loads, 1);
    assert_eq!(harness.state().value, LutChoice::Builtin("warm"));
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);
}
