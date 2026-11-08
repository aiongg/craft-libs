//! A key activation carries the modifiers held with the key, as a pointer activation carries the ones held with
//! the click: Alt+Enter on a button can add beside, the way Alt+click does.

use craft_behavior::{Metrics, Sense};
use craft_keymap::Mods;
use craft_ui::contract::{Activation, Common, Component, Cx, MeasureCx, Pointer as _, Props as _, Role, Show as _};
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Response, Ui, Vec2};
use std::cell::Cell;

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct AdderStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A button that records the activation the pipeline gave it.
#[derive(Component)]
pub struct Adder<'a> {
    #[common]
    common: Common,
    seen: &'a Cell<Option<Activation>>,
}

impl Component for Adder<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = AdderStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: AdderStyleKey) -> AdderStyle {
        AdderStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(activation) = cx.activation() {
            self.seen.set(Some(activation));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The activation that pressing `key` with `modifiers` on the focused button gives it.
fn activation_by(key: Key, modifiers: Modifiers) -> Option<Activation> {
    let seen = Cell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Adder { common: Common::new(), seen: &seen }.id_salt("adder").show(ui);
    };
    frames.frame(vec![], &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.hold(modifiers);
    frames.frame(vec![key_press(key, modifiers)], &mut show);
    seen.get()
}

#[trial]
fn alt_enter_on_a_button_activates_it_with_alt() {
    let plain = activation_by(Key::Enter, Modifiers::NONE).expect("Enter activates the focused button");
    assert!(plain.by_key, "Enter is a key activation");
    assert_eq!(plain.mods, Mods::NONE, "a plain Enter holds no modifier");
    let alt = activation_by(Key::Enter, Modifiers::ALT).expect("Alt+Enter activates the focused button");
    assert!(alt.by_key, "Alt+Enter is a key activation");
    assert!(alt.mods.alt, "the activation carries the Alt held with Enter: {:?}", alt.mods);
}
