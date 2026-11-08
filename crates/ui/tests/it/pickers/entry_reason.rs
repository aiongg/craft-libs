//! `Entry::disabled_reason`: a disabled option says why to assistive technology.

use craft_ui::contract::{Bind, Entry, Props as _, Show as _};
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

const REASON: &str = "Needs a layer with a mask";

/// A disabled entry with a reason reports the reason as its row's description; an enabled one reports none.
#[trial]
fn a_disabled_option_reports_its_reason_as_its_description() {
    let mut fixture = Fixture::new(|ui| {
        let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply").disabled_reason(REASON)];
        let mut mode = 0u8;
        Select::new("Blend mode", Bind::bound(&mut mode), &entries).id_salt("mode").show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    let multiply = fixture.by_label("Multiply").expect("the disabled option is listed");
    let normal = fixture.by_label("Normal").expect("the enabled option is listed");
    let tree = fixture.tree();
    assert_eq!(tree.description_of(multiply.node), Some(REASON));
    assert_eq!(tree.description_of(normal.node), None);
}
