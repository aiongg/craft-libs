//! Keys on a toggle control inside a dialog.

use crate::contract::kit::PlainStyle;
use craft_behavior::{Input, Sense};
use craft_keymap::Chord;
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Common, Component, Cx, Focus as _, MeasureCx, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::runtime::focus::Initial;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::style::StyleCx;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

/// Space toggles the focused check box and nothing else: the dialog stays open and no button is picked. Enter on it is the
/// default button's (the full-window dialog trials cover that).
#[trial]
fn space_on_a_focused_check_box_toggles_it_and_leaves_the_dialog_open() {
    let checked = Rc::new(Cell::new(false));
    let picked = Rc::new(Cell::new(None));
    let (shown_checked, shown_picked) = (checked.clone(), picked.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shown_checked.get();
        let mut open = true;
        let shown = Dialog::new("Options")
            .opened(Bind::bound(&mut open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .show(ui, |ui, _| {
                Checkbox::new(Bind::bound(&mut value)).label("Show the Grid").show(ui);
            });
        if shown.output.picked.is_some() {
            shown_picked.set(shown.output.picked);
        }
        shown_checked.set(value);
    });
    fixture.settle();
    assert!(fixture.focused().is_some(), "the check box holds focus");
    fixture.key(Key::Space, Modifiers::NONE).settle();
    assert!(checked.get(), "Space toggles the check box");
    assert_eq!(picked.get(), None, "Space picks no button");
}

const ESCAPE: &[Chord] = &[Chord::plain(craft_keymap::Key::Escape)];

/// A control that is armed until Escape disarms it. While armed it declares Escape as its own.
#[derive(craft_ui::contract::Component)]
struct Armed<'a> {
    #[common]
    common: Common,
    armed: &'a Cell<bool>,
}

impl Component for Armed<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = egui::Response;

    fn keys(&self) -> KeyUse {
        let keys = KeyUse::of(Input::Pointer);
        if self.armed.get() { keys.with_claims(ESCAPE) } else { keys }
    }

    fn style(_: &StyleCx<'_>, _: ArmedStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> egui::Vec2 {
        egui::Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.armed.get() && cx.take_claimed(ESCAPE[0]) {
            self.armed.set(false);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}

/// Escape on a focused control that declares it disarms the control and keeps the dialog open; the next Escape, with
/// nothing armed, closes the dialog.
#[trial]
fn escape_disarms_an_armed_control_in_a_dialog_before_it_closes_the_dialog() {
    let armed = Rc::new(Cell::new(true));
    let open = Rc::new(Cell::new(true));
    let (shown_armed, shown_open) = (armed.clone(), open.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut is_open = shown_open.get();
        Dialog::new("Adjust")
            .opened(Bind::bound(&mut is_open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .show(ui, |ui, _| {
                Armed { common: Common::new(), armed: &shown_armed }.label("Pick").show(ui);
            });
        shown_open.set(is_open);
    });
    fixture.settle();
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(!armed.get(), "Escape disarmed the control");
    assert!(open.get(), "the first Escape leaves the dialog open");
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(!open.get(), "the second Escape closes the dialog");
}
