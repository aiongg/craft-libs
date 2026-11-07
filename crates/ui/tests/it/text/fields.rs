//! Typing runs, submit and escape, undo, completion, the number field and the caret blink.

use super::kit::Rig;
use crate::label::kit::Field;
use craft_behavior::MotionMode;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::contract::{ShowContainer as _, SideLabel};
use craft_ui::primitives::{Completer, Completion};
use craft_ui::runtime::commands::{CommandRef, EditCommands};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::drag;
use craft_ui::testing::text::{caret, painted};
use craft_ui::testing::trial;
use craft_ui::text::{NumberField, TextField};
use egui::{Event, Key, Modifiers};
use std::ops::Range;

/// Typing then Ctrl+Z undoes the whole run; Ctrl+Z with nothing to undo fires the app's Undo.
#[trial]
fn typing_then_undo_takes_back_the_whole_run_and_an_empty_stack_reaches_the_app() {
    let mut rig = Rig::new("hello");
    rig.focus();
    let _ = take_output(&rig.frames.ctx);
    rig.frame(vec![key_press(Key::Z, Modifiers::COMMAND)]);
    let fired: Vec<_> = take_output(&rig.frames.ctx).fired.into_iter().map(|f| f.command).collect();
    assert_eq!(fired, [CommandRef::from(EditCommands::Undo)], "nothing to undo in the field: the app's Undo fires");
    for typed in ["a", "b", "c"] {
        rig.frame(vec![Event::Text(typed.into())]);
    }
    rig.idle(1);
    assert_eq!(rig.text, "helloabc");
    rig.frames.advance(2.0);
    rig.idle(3);
    let _ = take_output(&rig.frames.ctx);
    rig.frame(vec![key_press(Key::Z, Modifiers::COMMAND)]);
    rig.idle(3);
    assert_eq!(rig.text, "hello", "one undo step takes back the whole typing run");
    assert!(take_output(&rig.frames.ctx).fired.is_empty(), "the field took the undo");
}

/// Enter submits the typed value; Escape restores the value from before the run.
#[trial]
fn enter_submits_and_escape_restores() {
    let mut rig = Rig::new("hello");
    rig.focus();
    rig.frame(vec![Event::Text("!".into())]);
    rig.idle(2);
    assert_eq!(rig.text, "hello!", "plain text applies as typed");
    rig.frame(vec![key_press(Key::Escape, Modifiers::NONE)]);
    let mut escaped = rig.seen.escaped;
    for _ in 0..3 {
        rig.idle(1);
        escaped |= rig.seen.escaped;
    }
    assert_eq!(rig.text, "hello", "Escape restores the value from before the run");
    assert!(escaped, "the response says the edit was abandoned");
    rig.frame(vec![Event::Text("?".into())]);
    rig.idle(2);
    rig.frame(vec![key_press(Key::Enter, Modifiers::NONE)]);
    let mut submitted = rig.seen.submitted;
    for _ in 0..3 {
        rig.idle(1);
        submitted |= rig.seen.submitted;
    }
    assert_eq!(rig.text, "hello?", "Enter keeps the typed value");
    assert!(submitted, "and reports the submit");
}

struct Prefixes;

impl Completer for Prefixes {
    fn complete(&self, text: &str, caret: usize, out: &mut Vec<Completion>) -> Option<Range<usize>> {
        let before = text.get(..caret)?;
        let hits: Vec<_> = ["alpha", "alps", "beta"].into_iter().filter(|word| !before.is_empty() && word.starts_with(before)).collect();
        out.extend(hits.iter().map(|word| Completion::new(*word, *word)));
        (!hits.is_empty()).then_some(0..caret)
    }
}

/// The completion list takes Up, Down and Tab while it is open, and the caret does not move.
#[trial]
fn completion_list_takes_arrows_and_tab_while_open_and_the_caret_does_not_move() {
    let mut rig = Rig::new("");
    rig.focus();
    let mut frame = |rig: &mut Rig, events: Vec<Event>| {
        let (text, seen) = (&mut rig.text, &mut rig.seen);
        rig.frames.frame(events, |ui| {
            let out = TextField::new("Name", Bind::bound(&mut *text)).complete(Prefixes).id_salt("name").show(ui);
            seen.rect = out.response.rect;
            seen.id = out.response.id;
        });
    };
    frame(&mut rig, vec![Event::Text("al".into())]);
    for _ in 0..4 {
        frame(&mut rig, vec![]);
    }
    let shapes = {
        let (text, seen) = (&mut rig.text, &mut rig.seen);
        rig.frames.frame_shapes(|ui| {
            let out = TextField::new("Name", Bind::bound(&mut *text)).complete(Prefixes).id_salt("name").show(ui);
            seen.id = out.response.id;
        })
    };
    let shown = painted(&shapes);
    assert!(shown.iter().any(|t| t == "alpha") && shown.iter().any(|t| t == "alps"), "typing opens the list: {shown:?}");
    let widget = rig.widget();
    let before = caret(&rig.frames.ctx, widget);
    for key in [Key::ArrowDown, Key::ArrowDown, Key::ArrowUp, Key::ArrowDown] {
        frame(&mut rig, vec![key_press(key, Modifiers::NONE)]);
        frame(&mut rig, vec![]);
    }
    assert_eq!(caret(&rig.frames.ctx, widget), before, "a claimed Up or Down does not also move the caret");
    let focus = craft_ui::testing::focus::egui_focused(&rig.frames.ctx);
    frame(&mut rig, vec![key_press(Key::Tab, Modifiers::NONE)]);
    for _ in 0..4 {
        frame(&mut rig, vec![]);
    }
    assert_eq!(craft_ui::testing::focus::egui_focused(&rig.frames.ctx), focus, "Tab went to the list, not to the next stop");
    assert_eq!(rig.text, "alps", "Tab accepted the highlighted candidate");
}

/// A number field parses through its codec, rejects text that is no number and keeps the value.
#[trial]
fn number_field_parses_through_its_codec_and_rejects_bad_text() {
    let mut frames = craft_ui::testing::Frames::new();
    let value = std::cell::RefCell::new(Some(1.0f64));
    let rect = std::cell::Cell::new(egui::Rect::NOTHING);
    let mut show = |ui: &mut egui::Ui| {
        let out = NumberField::new("Size", Bind::bound(&mut *value.borrow_mut())).id_salt("size").show(ui);
        rect.set(out.response.rect);
    };
    frames.idle(3, &mut show);
    craft_ui::testing::pointer::click(&mut frames, rect.get().center(), &mut show);
    let mut typed = |frames: &mut craft_ui::testing::Frames, text: &str, show: &mut dyn FnMut(&mut egui::Ui)| {
        frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut *show);
        frames.frame(vec![Event::Text(text.into())], &mut *show);
        frames.frame(vec![], &mut *show);
        frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut *show);
        for _ in 0..3 {
            frames.frame(vec![], &mut *show);
        }
    };
    typed(&mut frames, "2+3", &mut show);
    assert_eq!(*value.borrow(), Some(5.0), "an expression parses on commit");
    typed(&mut frames, "x1", &mut show);
    assert_eq!(*value.borrow(), Some(5.0), "text that is no number leaves the value");
}

/// The caret blinks while it can and stays solid when motion is off.
#[trial]
fn caret_blinks_and_stops_when_motion_is_off() {
    let visible_over_time = |mode: MotionMode| {
        let mut rig = Rig::new("hello").with_motion(mode);
        rig.focus();
        let mut seen = Vec::new();
        for _ in 0..8 {
            rig.frames.advance(0.3);
            let shapes = rig.frames.frame_shapes(|ui| {
                TextField::new("Name", Bind::bound(&mut rig.text)).id_salt("name").show(ui);
            });
            let strokes = shapes.iter().filter(|shape| matches!(shape, egui::Shape::LineSegment { points, stroke } if points[0].x == points[1].x && stroke.color != egui::Color32::TRANSPARENT)).count();
            seen.push(strokes > 0);
        }
        seen
    };
    let full = visible_over_time(MotionMode::Full);
    assert!(full.contains(&true) && full.contains(&false), "the caret blinks: {full:?}");
    let off = visible_over_time(MotionMode::Off);
    assert!(off.iter().all(|shown| *shown), "the caret stays solid: {off:?}");
}

/// A number field with `scrub_by_label` follows a drag on its side label, one point of drag per unit.
#[trial]
fn a_number_field_scrubs_by_its_label() {
    const ROW: egui::Rect = egui::Rect { min: egui::Pos2::new(10.0, 10.0), max: egui::Pos2::new(310.0, 40.0) };
    const DRAG: f32 = 30.0;
    const ROUNDING: f64 = 1.5;
    let mut frames = craft_ui::testing::Frames::new();
    let value = std::cell::RefCell::new(Some(10.0f64));
    let mut show = |ui: &mut egui::Ui| {
        Field::new("Opacity", SideLabel::new()).id_salt("row").show_at(ui, ROW, |ui, _| {
            NumberField::new("Opacity", Bind::bound(&mut *value.borrow_mut())).scrub_by_label().id_salt("scrubbed").show(ui);
        });
    };
    frames.idle(4, &mut show);
    drag(&mut frames, ROW.min + egui::vec2(4.0, 12.0), egui::vec2(DRAG, 0.0), &mut show);
    let scrubbed = value.borrow().unwrap_or_default();
    assert!((scrubbed - 10.0 - f64::from(DRAG)).abs() < ROUNDING, "a drag on the label scrubs by its length: {scrubbed}");
}
