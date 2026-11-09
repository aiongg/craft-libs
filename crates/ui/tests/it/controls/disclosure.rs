//! `Disclosure`: toggles its body by pointer and key and reports the expanded state.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::controls::disclosure::Disclosure;
use craft_ui::testing::focus::{egui_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Rect, Ui};
use std::cell::Cell;

/// A disclosure and the body it opens; the body is shown only while the value is open.
fn frame(ui: &mut Ui, open: &mut bool, at: &Cell<Option<(Rect, Id)>>, body_shown: &Cell<bool>) {
    let response = Disclosure::new(Bind::bound(&mut *open)).label("Details").id_salt("disclosure").show(ui);
    at.set(Some((response.response.rect, response.response.id)));
    body_shown.set(*open);
    if *open {
        ui.label("body");
    }
}

