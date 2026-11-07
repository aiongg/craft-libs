//! The chrome toggle under a dialog layer: Tab toggles nothing.

use craft_behavior::Input;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::dialog::Alert;
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::focus::{Stop, named_id, register_stop, widget};
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Ui};

#[trial]
fn tab_inside_an_open_dialog_toggles_nothing() {
    let mut show = |ui: &mut Ui| {
        widget(ui, named_id("canvas"));
        register_stop(ui.ctx(), Stop::new(named_id("canvas"), KeyUse::CANVAS));
        widget(ui, named_id("button"));
        register_stop(ui.ctx(), Stop::new(named_id("button"), KeyUse::of(Input::Pointer)));
        let mut open = true;
        Alert::new("Error", "The name is taken.").opened(Bind::bound(&mut open)).show(ui);
    };
    let mut frames = Frames::new();
    let mut root = RootEnv::default();
    root.tab_toggles_chrome = true;
    frames.root(root);
    frames.frame(Vec::new(), &mut show);
    frames.frame(Vec::new(), &mut show);
    take_output(&frames.ctx);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    assert!(take_output(&frames.ctx).chrome_toggles.is_empty(), "no toggle under a dialog");
}
