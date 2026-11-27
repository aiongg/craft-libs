//! A native-owned Menu text press keeps draft Escape precedence; the separate SDK-arena Checkbox oracle is unchanged.
use craft_behavior::MotionMode;
use craft_ui::Id;
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::geom::Rect;
use craft_ui::input::{Key, Mods};
use craft_ui::menu::Menu;
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Harness, pointer, trial};
use craft_ui::text::TextField;
use egui::Event;
use std::cell::RefCell;
use std::rc::Rc;
struct State {
    header: Option<Rect>,
    field: Option<Rect>,
    root: Option<Id>,
    pressed: bool,
    value: String,
    commits: usize,
}
#[trial]
fn a_menu_native_text_press_preserves_draft_escape_without_owner_close_or_blur_commit() {
    let state = Rc::new(RefCell::new(State { header: None, field: None, root: None, pressed: false, value: "seed".to_owned(), commits: 0 }));
    let held = state.clone();
    let mut fixture = Harness::new_ui(move |ui| {
        let mut state = held.borrow_mut();
        let live = !ui.is_measuring() && !ui.is_measuring();
        let header = ui.button("Real text editor header");
        if live {
            state.header = Some(header.rect);
            state.field = None;
            state.pressed = false;
        }
        Menu::context("Editor options", &header).id_salt("real-text-menu").show(ui, |ui, _| {
            let mut value = state.value.clone();
            let shown = TextField::new("Draft inside Menu", Bind::bound(&mut value)).id_salt("draft-field").show(ui);
            if !ui.is_measuring() && !ui.is_measuring() {
                state.value = value;
                state.field = Some(shown.response.interact_rect);
                state.root = Some(shown.response.id);
                // The SDK rewrites this flag from its actual arena phase, clearing native flags first. This is not
                // pointer.press_targets, whose native target may remain latched after cancellation.
                state.pressed = shown.response.is_pointer_button_down_on();
                state.commits += usize::from(shown.events.committed().is_some());
            }
        });
    });
    fixture.motion(MotionMode::Off).settle();
    let header = state.borrow().header.expect("real header").center();
    fixture.send(pointer::moved(header));
    fixture.send(pointer::secondary(header, true));
    fixture.send(pointer::secondary(header, false));
    fixture.settle();
    let field = state.borrow().field.expect("actual canonical Menu editor").center();
    fixture.click_at(field);
    fixture.type_text("Q7");
    assert!(fixture.texts().iter().any(|run| run.text.contains("Q7")), "a real typed draft was painted");
    assert_eq!(state.borrow().commits, 0, "typing did not already commit by blur");
    let native = fixture.ctx().memory(|memory| memory.focused()).expect("real typing established the actual native text owner");
    assert_ne!(Some(native), state.borrow().root, "the native editor is distinct from its SDK component root");
    let at = state.borrow().field.expect("same live editor").center();
    fixture.frame(vec![pointer::moved(at), pointer::press(at)]);
    let target = craft_ui::runtime::pointer::with_pass_pointer(fixture.ctx(), |pointer| pointer.press_targets.click);
    assert_eq!(target, Some(native), "this fresh physical press actually selects the same native text owner that typed the draft");
    assert!(!state.borrow().pressed, "a registered native text press intentionally goes to no SDK root arena");
    assert!(fixture.ctx().input(|input| input.pointer.primary_down()));
    fixture.send(key_press(Key::Escape, Modifiers::NONE));
    assert!(fixture.ctx().input(|input| input.pointer.primary_down()));
    assert_eq!(state.borrow().commits, 0);
    fixture.send(Event::Key { key: Key::Escape, physical_key: Some(Key::Escape), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    assert!(state.borrow().field.is_some(), "the existing editor/draft capture precedence must not become a Menu close");
    assert_eq!(state.borrow().commits, 0, "no draft blur commit was manufactured");
    fixture.send(pointer::release(at));
    assert_eq!(state.borrow().commits, 0, "the cancelled old release does not commit");
    fixture.type_text("Z8");
    assert!(fixture.texts().iter().any(|run| run.text.contains("Z8")), "fresh real editor input still works");
    fixture.send(key_press(Key::Enter, Modifiers::NONE));
    fixture.send(Event::Key { key: Key::Enter, physical_key: Some(Key::Enter), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    assert_eq!(state.borrow().commits, 1, "only the fresh explicit Enter commits once");
    assert!(state.borrow().value.contains("Z8"));
}
