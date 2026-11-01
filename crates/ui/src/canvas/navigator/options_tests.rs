//! Navigator Panel Options on screen.
#![allow(clippy::expect_used, reason = "tests")]

use craft_theme::Rgba;
use egui::accesskit::Role;
use egui::vec2;
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::{NAVIGATOR_PROXY_COLOR_KEY, NavigatorOptions, NavigatorOptionsDialog, ProxyColor};
use crate::dialog::{DialogHost, DialogResult};
use crate::theme::{Base, install};

#[derive(Default)]
struct App {
    host: DialogHost,
    answer: Option<DialogResult<NavigatorOptions>>,
}

fn open(initial: NavigatorOptions) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(NavigatorOptionsDialog::new().initial(initial));
    let mut harness =
        HarnessBuilder::default().with_size(vec2(800.0, 600.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(result) = app.host.take::<NavigatorOptionsDialog>() {
                    app.answer = Some(result);
                }
            },
            App { host, answer: None },
        );
    harness.run();
    harness
}

fn choose(harness: &mut Harness<'_, App>, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, "Color").click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
}

fn dialog<'a>(harness: &'a mut Harness<'_, App>) -> &'a mut NavigatorOptionsDialog {
    harness.state_mut().host.get_mut::<NavigatorOptionsDialog>().expect("open")
}

// OK answers the chosen colour (one setting); Cancel leaves the settings; Custom… asks once with
// `NAVIGATOR_PROXY_COLOR_KEY`.
#[test]
fn navigator_options_dialog_returns_one_setting() {
    let mut harness = open(NavigatorOptions::default());
    assert_eq!(dialog(&mut harness).options().proxy_color, ProxyColor::Default);
    choose(&mut harness, "Green");
    assert_eq!(dialog(&mut harness).options().proxy_color, ProxyColor::Green);

    choose(&mut harness, "Custom…");
    let request = dialog(&mut harness).pick_color().expect("Custom… asks for a colour");
    assert_eq!(request.key, NAVIGATOR_PROXY_COLOR_KEY);
    assert_eq!(request.title.as_deref(), Some("Color Picker (Proxy Color)"));
    assert!(dialog(&mut harness).pick_color().is_none(), "the request is taken once");
    dialog(&mut harness).cancel_color();
    harness.run();
    assert_eq!(dialog(&mut harness).options().proxy_color, ProxyColor::Green, "a cancelled pick restores the colour");

    choose(&mut harness, "Custom…");
    assert!(dialog(&mut harness).pick_color().is_some());
    dialog(&mut harness).set_color("other", Rgba::rgb(9, 9, 9).into());
    dialog(&mut harness).set_color(NAVIGATOR_PROXY_COLOR_KEY, Rgba::rgb(0x12, 0x34, 0x56).into());
    harness.run();
    assert_eq!(dialog(&mut harness).options().proxy_color, ProxyColor::Custom([0x12, 0x34, 0x56]));

    harness.get_by_role_and_label(Role::Button, "OK").click();
    harness.run();
    let answer = harness.state_mut().answer.take();
    assert!(matches!(answer, Some(DialogResult::Ok(o)) if o.proxy_color == ProxyColor::Custom([0x12, 0x34, 0x56])), "{answer:?}");

    let mut harness = open(NavigatorOptions { proxy_color: ProxyColor::Blue });
    choose(&mut harness, "Red");
    harness.get_by_role_and_label(Role::Button, "Cancel").click();
    harness.run();
    assert!(matches!(harness.state().answer, Some(DialogResult::Cancel)), "Cancel answers no options");
}
