//! Count trials of the window's command keys on craft-ui-rt: a chord bound to a command runs it once per press, waits
//! while a text stop has the keyboard, and follows the keymap's overrides. Relations only, no times.

use crate::skeleton::IconButton;
use craft_behavior::IconId;
use craft_keymap::{Binding, CommandDef, Keymap, Overrides, Platform, Scope, Scopes};
use craft_ui_contract::contract::{Common, Component, Input as InputKind, MeasureCx, Response, Role, Show, Ui};
use craft_ui_contract::runtime::commands::CommandBoard;
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::Harness;
use craft_ui_rt::{Key, Mods, Sense, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use std::cell::Cell;
use std::rc::Rc;

/// A text stop: a focused `Field` has the keyboard for typing.
#[derive(Component)]
struct Field {
    #[common]
    common: Common,
}

impl Component for Field {
    const ROLE: Role = Role::BUTTON;
    const INPUT: InputKind = InputKind::Text;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FieldStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(24.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn finish(self, cx: craft_ui_contract::contract::Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A window with a button, then a text field, whose board counts the commands that run.
fn window(overrides: Option<Overrides>) -> (Harness<()>, Rc<Cell<usize>>) {
    let go = CommandDef::new("trial.go", "Go", Scope::app()).with_default(Binding::parse("J").expect("a valid binding"));
    let mut keymap = Keymap::new(vec![go], Scopes::new(), Platform::Linux);
    if let Some(overrides) = overrides {
        keymap = keymap.with_overrides(overrides);
    }
    let board = CommandBoard::new(keymap);
    let runs = Rc::new(Cell::new(0));
    let seen = Rc::clone(&runs);
    board.on_app(move |_, _| seen.set(seen.get() + 1));
    let mut h = Harness::builder().with_commands(board).build_ui(|ui: &mut Ui<'_, '_>| {
        IconButton::new(IconId::new("go"), "Go").show(ui);
        drop(Show::show(Field { common: Common::new() }, ui));
    });
    h.run();
    (h, runs)
}

#[test]
fn a_bound_key_runs_its_command_once_per_press_and_waits_while_a_text_stop_types() {
    let (mut h, runs) = window(None);
    h.key_press(Key::Tab);
    h.key_press(Key::J);
    assert_eq!(runs.get(), 1, "from the button the bound key runs the command once");
    h.key_press(Key::Tab);
    h.key_press(Key::J);
    assert_eq!(runs.get(), 1, "while the field has the keyboard the plain key is typing");
}

#[test]
fn a_rebound_command_runs_on_its_new_key_only() {
    let mut overrides = Overrides::default();
    overrides.insert("trial.go", vec!["F9".to_owned()]);
    let (mut h, runs) = window(Some(overrides));
    h.key_press(Key::Tab);
    h.key_press(Key::J);
    assert_eq!(runs.get(), 0, "the default key no longer runs it");
    h.key(Key::F9, Mods::NONE);
    assert_eq!(runs.get(), 1, "the new key does");
}
