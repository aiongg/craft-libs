//! The transport bar and media controls (egui_kittest).

use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::transport::{MediaControls, TransportBar, TransportBarSettings, TransportButton};

fn harness<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S) -> Harness<'static, S> {
    let mut harness = Harness::<S>::builder()
        .with_size([720.0, 200.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, state);
    harness.run();
    harness
}

#[derive(Default)]
struct Bar {
    settings: TransportBarSettings,
    pressed: Vec<TransportButton>,
    changed: usize,
    playing: bool,
}

fn show_bar(ui: &mut Ui, app: &mut Bar) {
    let out = TransportBar::new("bar", &TransportButton::PROGRAM).playing(app.playing).settings(&mut app.settings).show(ui);
    app.pressed.extend(out.pressed);
    app.changed += usize::from(out.settings_changed);
}

// PlayStop shows Pause while playing; a click reports Pressed.
#[test]
fn buttons_report_presses_and_show_pause_while_playing() {
    assert_eq!(TransportButton::PlayStop.icon(false), Icon::Play);
    assert_eq!(TransportButton::PlayStop.icon(true), Icon::Pause);
    let mut harness = harness(show_bar, Bar::default());
    harness.get_by_role_and_label(Role::Button, "Play-Stop Toggle (Space)").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Mark In (I)").click();
    harness.run();
    assert_eq!(harness.state().pressed, [TransportButton::PlayStop, TransportButton::MarkIn]);
}

// hidden buttons don't show; the editor's checkboxes report Hidden.
#[test]
fn hidden_buttons_are_left_out_and_the_editor_changes_them() {
    let mut harness = harness(show_bar, Bar::default());
    harness.state_mut().settings.hidden.push(TransportButton::Loop);
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, "Loop").is_none());
    assert!(harness.query_by_role_and_label(Role::Button, "Mark In (I)").is_some());

    harness.get_by_role_and_label(Role::Button, "Button Editor").click();
    harness.run();
    harness.get_by_role_and_label(Role::CheckBox, "Mark In (I)").click();
    harness.run();
    assert_eq!(harness.state().settings.hidden, [TransportButton::Loop, TransportButton::MarkIn]);
    assert_eq!(harness.state().changed, 1);
    assert!(harness.query_by_role_and_label(Role::Button, "Mark In (I)").is_none());

    harness.get_by_role_and_label(Role::Button, "Reset").click();
    harness.run();
    assert!(harness.state().settings.hidden.is_empty());
    assert_eq!(harness.state().changed, 2);
    assert!(harness.query_by_role_and_label(Role::Button, "Loop").is_some());
}

#[derive(Default)]
struct Media {
    position: f64,
    toggles: usize,
    nudges: Vec<f64>,
    seeks: Vec<EditEvent<f64>>,
}

fn show_media(ui: &mut Ui, app: &mut Media) {
    let out = MediaControls::new("media", app.position, 10.0).trim(2.0, 8.0).show(ui);
    app.toggles += usize::from(out.toggle);
    app.nudges.extend(out.nudge);
    app.seeks.extend(out.seek);
}

// play toggles, the arrows nudge 0.25 s, the track seeks.
#[test]
fn media_controls_toggle_nudge_and_seek() {
    let mut harness = harness(show_media, Media { position: 6.5, ..Media::default() });
    harness.get_by_role_and_label(Role::Button, "Play or pause media").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Back 0.25 seconds").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Forward 0.25 seconds").click();
    harness.run();
    assert_eq!(harness.state().toggles, 1);
    assert_eq!(harness.state().nudges, [-0.25, 0.25]);

    let seek = harness.get_by_role_and_label(Role::Slider, "Seek");
    assert_eq!(seek.accesskit_node().data().value(), Some("0:06.50"), "minutes, seconds and hundredths");
    let centre = seek.rect().center();
    harness.click_at(centre);
    let value = harness.state().seeks.iter().find_map(|event| event.value().copied()).expect("a click on the track seeks");
    assert!((4.0..=6.0).contains(&value), "mid-track is about 5 s: {value}");

    harness.state_mut().seeks.clear();
    harness.get_by_role_and_label(Role::Slider, "Seek").focus();
    harness.run();
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight));
    let step = harness.state().seeks.iter().find_map(|event| match event {
        EditEvent::Set { value, .. } => Some(*value),
        _ => None,
    });
    assert_eq!(step, Some(6.75), "Right moves a quarter second");
}

// the buttons are `size.control.md` squares, not the toolbar's own size.
#[test]
fn transport_buttons_use_the_timeline_leaf_size() {
    let harness = harness(
        |ui, side: &mut f32| {
            *side = Tokens::of(ui).size.control.md;
            let _ = TransportBar::new("bar", &TransportButton::PROGRAM).show(ui);
        },
        0.0,
    );
    let side = *harness.state();
    let rect = harness.get_by_role_and_label(Role::Button, "Mark In (I)").rect();
    assert!((rect.height() - side).abs() < 0.5, "{} against {side}", rect.height());
    assert!((rect.width() - side).abs() < 0.5, "{} against {side}", rect.width());
}

// the media-controls bar is a named toolbar.
#[test]
fn media_controls_are_a_named_toolbar() {
    let harness = harness(show_media, Media::default());
    assert!(harness.query_by_role_and_label(Role::Toolbar, "Media controls").is_some());
}
