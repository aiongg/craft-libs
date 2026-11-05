//! Tests of Slice Options and Divide Slice.

use egui::accesskit::Role;
use egui::{Event, Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{DivideBy, DivideSlices, DivideSlicesDialog, SliceKind, SliceOptions, SliceOptionsDialog};
use craft_theme::Srgba;
use craft_ui::dialog::{DialogHost, DialogResult, StackDialog};
use craft_ui::theme::{Base, install};

struct App<D: StackDialog> {
    host: DialogHost,
    answers: Vec<D::Result>,
    cancelled: usize,
}

fn harness<D: StackDialog>(dialog: D) -> Harness<'static, App<D>> {
    let mut app = App::<D> { host: DialogHost::default(), answers: Vec::new(), cancelled: 0 };
    app.host.open(dialog);
    let mut h =
        HarnessBuilder::default().with_size(vec2(900.0, 700.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App<D>| {
                app.host.show(ui.ctx());
                match app.host.take::<D>() {
                    Some(DialogResult::Ok(answer)) => app.answers.push(answer),
                    Some(DialogResult::Cancel) => app.cancelled += 1,
                    _ => {}
                }
            },
            app,
        );
    h.run();
    h
}

fn click<D: StackDialog>(h: &mut Harness<'_, App<D>>, role: Role, label: &str) {
    h.get_by_role_and_label(role, label).click();
    h.run();
}

fn type_into<D: StackDialog>(h: &mut Harness<'_, App<D>>, role: Role, label: &str, text: &str) {
    h.get_by_role_and_label(role, label).focus();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.event(Event::Text(text.into()));
    h.run();
    h.key_press(Key::Tab);
    h.run();
}

fn count<D: StackDialog>(h: &Harness<'_, App<D>>, role: Role, label: &str) -> usize {
    h.query_all_by_role_and_label(role, label).count()
}

// Rows by count 3 and columns by size 50 come back as `DivideSlices`; both off disables OK.
#[test]
fn divide_slices_count_and_size_modes() {
    let mut h = harness(DivideSlicesDialog::new(DivideSlices::default()));
    type_into(&mut h, Role::SpinButton, "Divide Horizontally Into, slices, evenly spaced", "3");
    click(&mut h, Role::CheckBox, "Divide Vertically Into");
    h.get_by_role_and_label(Role::ComboBox, "Divide Vertically Into, slices, evenly spaced / pixels per slice").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("pixels per slice")).click();
    h.run_steps(3);
    type_into(&mut h, Role::SpinButton, "Divide Vertically Into, pixels per slice", "50");
    click(&mut h, Role::Button, "OK");
    assert_eq!(h.state().answers, [DivideSlices { rows: Some(DivideBy::Count(3)), columns: Some(DivideBy::Size(50)) }]);

    let mut h = harness(DivideSlicesDialog::new(DivideSlices::default()));
    click(&mut h, Role::CheckBox, "Divide Horizontally Into");
    assert!(h.get_by_role_and_label(Role::Button, "OK").accesskit_node().is_disabled());
    click(&mut h, Role::Button, "OK");
    assert!(h.state().answers.is_empty());
    click(&mut h, Role::Button, "Cancel");
    assert_eq!(h.state().cancelled, 1);
}

// Image shows Alt Tag, No Image shows Text; the Background row only with `background_row`; Cancel
// answers nothing.
#[test]
fn slice_options_fields_follow_the_slice_type() {
    let mut h = harness(SliceOptionsDialog::new(SliceOptions { name: "hero".into(), ..SliceOptions::default() }));
    assert_eq!((count(&h, Role::TextInput, "Alt Tag"), count(&h, Role::TextInput, "Text")), (1, 0));
    assert_eq!(count(&h, Role::Button, "Background"), 0);
    h.get_by_role_and_label(Role::ComboBox, "Slice Type").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("No Image")).click();
    h.run_steps(3);
    assert_eq!((count(&h, Role::TextInput, "Alt Tag"), count(&h, Role::TextInput, "Text")), (0, 1));
    type_into(&mut h, Role::TextInput, "URL", "https://example.test");
    click(&mut h, Role::Button, "OK");
    let [answer] = h.state().answers.as_slice() else { panic!("OK answers once") };
    assert_eq!((answer.kind, answer.name.as_str(), answer.url.as_str()), (SliceKind::NoImage, "hero", "https://example.test"));

    let options = SliceOptions { background_row: true, background: Some(Srgba::new(1.0, 0.0, 0.0, 1.0)), ..SliceOptions::default() };
    let mut h = harness(SliceOptionsDialog::new(options));
    assert_eq!(count(&h, Role::Button, "None"), 1);
    click(&mut h, Role::Button, "None");
    click(&mut h, Role::Button, "Cancel");
    assert!(h.state().answers.is_empty());
    assert_eq!(h.state().cancelled, 1);
}
