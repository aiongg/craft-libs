//! Behaviour tests of `testing::frame`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use egui::{Key, KeyboardShortcut, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::HarnessExt as _;

#[derive(Default)]
struct Clicks {
    plain: usize,
    shift: usize,
}

// click_at clicks a button once, and click_at_with delivers Shift with the click (a button reading modifiers sees them). Frame order is tested by click_presses_and_releases_on_separate_frames.
#[test]
fn click_at_counts_once_and_modifiers_reach_the_press() {
    let mut h = Harness::builder().with_size(vec2(200.0, 100.0)).build_ui_state(
        |ui, state: &mut Clicks| {
            if ui.button("Go").clicked() {
                if ui.input(|i| i.modifiers.shift) {
                    state.shift += 1;
                } else {
                    state.plain += 1;
                }
            }
        },
        Clicks::default(),
    );
    h.run_steps(2);
    let at = h.get_by_label("Go").rect().center();
    h.click_at(at);
    assert_eq!((h.state().plain, h.state().shift), (1, 0));
    h.click_at_with(at, egui::PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!((h.state().plain, h.state().shift), (1, 1));
}

// drag moves a DragValue by the pointer distance and ends with the button up.
#[test]
fn drag_moves_through_every_step_and_releases() {
    let mut h = Harness::builder().with_size(vec2(300.0, 100.0)).build_ui_state(
        |ui, value: &mut f32| {
            let label = ui.label("Amount");
            ui.add(egui::DragValue::new(value).speed(1.0)).labelled_by(label.id);
        },
        0.0_f32,
    );
    h.run_steps(2);
    let from = h.get_by_role(egui::accesskit::Role::SpinButton).rect().center();
    h.drag(from, from + vec2(40.0, 0.0), Modifiers::NONE);
    // egui starts a drag only after the pointer has left its click radius, so the first few points are lost.
    assert!((30.0..=40.0).contains(h.state()), "dragged by about the pointer distance, got {}", h.state());
    assert!(!h.ctx.input(|i| i.pointer.any_down()), "button released");
}

// press(Cmd+Z) is seen by ctx.input_mut().consume_shortcut, and type_text fills a focused TextEdit.
#[test]
fn keys_and_text_reach_the_focused_widget() {
    let shortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
    let mut h = Harness::builder().with_size(vec2(300.0, 100.0)).build_ui_state(
        move |ui, state: &mut (String, usize)| {
            if ui.input_mut(|i| i.consume_shortcut(&shortcut)) {
                state.1 += 1;
            }
            let label = ui.label("Name");
            ui.add(egui::TextEdit::singleline(&mut state.0)).labelled_by(label.id);
        },
        (String::new(), 0),
    );
    h.run_steps(2);
    h.press(shortcut);
    assert_eq!(h.state().1, 1);
    let at = h.get_by_role(egui::accesskit::Role::TextInput).rect().center();
    h.click_at(at);
    h.type_text("Layer 2");
    assert_eq!(h.state().0, "Layer 2");
}

// pump_idle returns true once a worker thread's flag clears, and false at the timeout while it never does.
#[test]
fn pumps_wait_for_background_work_and_time_out() {
    let mut h = Harness::builder().with_size(vec2(100.0, 50.0)).build_ui_state(|_, _: &mut Arc<AtomicBool>| {}, Arc::new(AtomicBool::new(true)));
    let flag = h.state().clone();
    let worker = {
        let flag = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            flag.store(false, Ordering::SeqCst);
        })
    };
    assert!(h.pump_idle(Duration::from_secs(10), |h| h.state().load(Ordering::SeqCst)));
    worker.join().unwrap();

    let never = Arc::new(AtomicBool::new(true));
    assert!(!h.pump_idle(Duration::from_millis(100), |_| never.load(Ordering::SeqCst)));
    assert!(!h.pump_until(Duration::from_millis(100), |_| false));
    assert!(h.pump_until(Duration::from_millis(100), |_| true));
}

// The harness plays the shared event frames: a click's move, press and release arrive in three frames, each with the pointer state the frame before left.
#[test]
fn click_presses_and_releases_on_separate_frames() {
    use std::cell::RefCell;
    let seen = std::rc::Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let mut h = Harness::builder().with_size(vec2(200.0, 100.0)).build_ui(move |ui| {
        let (pressed, released, down) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.primary_down()));
        if pressed || released {
            log.borrow_mut().push((pressed, released, down));
        }
        let _ = ui.button("Go");
    });
    h.run_steps(2);
    let at = h.get_by_label("Go").rect().center();
    h.click_at(at);
    assert_eq!(*seen.borrow(), vec![(true, false, true), (false, true, false)], "press and release land in different frames");
}
