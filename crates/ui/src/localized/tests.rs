use egui::accesskit::Role;
use egui::{DragValue, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

#[test]
fn formats_and_reads_the_regions_numbers() {
    let de = LocaleNumbers::from_tag("de");
    assert_eq!(de.format(1.5, 0..=6), "1,5");
    assert_eq!(de.format(1234567.0, 0..=2), "1.234.567");
    assert_eq!(de.format(2.0, 2..=4), "2,00");
    assert_eq!(de.format(-0.001, 0..=2), "0");
    assert_eq!(de.parse("1,5"), Ok(1.5));
    assert_eq!(de.parse("1.234.567"), Ok(1234567.0));
    assert!(de.parse("1.234.").is_err_and(|e| incomplete("1.234.", e)), "a grouping mark still to be followed by digits");
    assert_eq!(de.parse("1.5"), Ok(1.5), "a lone point that can't be grouping");

    let en = LocaleNumbers::from_tag("en-US");
    assert_eq!(en.format(1234567.25, 0..=2), "1,234,567.25");
    assert_eq!(en.parse("1,234,567.25"), Ok(1234567.25));
    assert_eq!(en.parse("１２，３４５．５"), Ok(12345.5), "full-width digits and marks");
    assert_eq!(en.parse("1,5,5"), Err(NumberError::Separator));

    let ar = LocaleNumbers::from_tag("ar-EG");
    assert_eq!(ar.format(1234.5, 1..=1), "١٬٢٣٤٫٥");
    assert_eq!(ar.parse("١٬٢٣٤٫٥"), Ok(1234.5));
    assert_eq!(ar.parse(&ar.format(-12.5, 0..=2)), Ok(-12.5), "the sign and bidi marks read back");
    assert_eq!(ar.parse("12.5"), Ok(12.5));

    let fa = LocaleNumbers::from_tag("fa-IR");
    assert_eq!(fa.format(1.5, 0..=2), "۱٫۵");
    assert_eq!(fa.parse("۱٫۵"), Ok(1.5));
    assert_eq!(fa.parse("1۵"), Err(NumberError::MixedDigits));
}

#[test]
fn a_region_reads_back_its_own_decimal_mark() {
    // English in the US with a decimal comma and no grouping from the Windows settings.
    let mut region = craft_i18n::Region::from_tag("en-US");
    region.decimal = Some(',');
    let numbers = LocaleNumbers::new(Arc::new(Format::new(&region, craft_i18n::Digits::Locale)));
    let written = numbers.format(1234.5, 1..=1);
    assert_eq!((written.as_str(), numbers.parse(&written)), ("1.234,5", Ok(1234.5)));
    assert_eq!(numbers.parse("1.234"), Ok(1234.0), "the grouping moved to the point");
    region.grouping = Some(None);
    let numbers = LocaleNumbers::new(Arc::new(Format::new(&region, craft_i18n::Digits::Locale)));
    assert_eq!(numbers.format(1234.5, 1..=1), "1234,5");
}

#[test]
fn units_wrap_and_strip() {
    let unit = Unit::new("%{n}");
    assert_eq!((unit.prefix(), unit.suffix()), ("%", ""));
    let px = Unit::new("{n} px");
    assert_eq!(px.strip(" 12,5 px "), "12,5");
    assert_eq!(px.strip("12,5"), "12,5");
    let (i18n, _) = I18n::new(&[("fr", "unit\t{n} px\t{n} pt-écran\n", "")], &["fr"]);
    let fr = i18n.from_code("fr").unwrap_or(Lang::EN);
    assert_eq!(Unit::translated(&i18n, fr, "{n} px").suffix(), " pt-écran");
    assert_eq!(Unit::translated(&i18n, Lang::EN, "{n} px").suffix(), " px");
}

fn drag_harness(tag: &str, value: f64) -> Harness<'static, f64> {
    let numbers = LocaleNumbers::from_tag(tag);
    drag_harness_with(value, move |ui, value| LocalizedDragValue::new(DragValue::new(value).speed(0.1), &numbers).ui(ui))
}

fn drag_harness_with(value: f64, add: impl FnMut(&mut Ui, &mut f64) -> Response + 'static) -> Harness<'static, f64> {
    let mut add = add;
    // The default fonts have no Arabic or Persian digits; the fonts module adds them in apps.
    let mut h = Harness::builder().allow_missing_glyphs().build_ui_state(
        move |ui, value: &mut f64| {
            let label = ui.label("Width");
            add(ui, value).labelled_by(label.id);
        },
        value,
    );
    h.run();
    h
}

/// Clicks the drag value into text editing, types `text` over its selected text and presses Enter.
fn type_into_drag(h: &mut Harness<'_, f64>, text: &str) {
    h.get_by_role(Role::SpinButton).click();
    h.run();
    h.get_by_role(Role::SpinButton).type_text(text);
    h.run();
    h.key_press(Key::Enter);
    h.run();
}

#[test]
fn drag_value_shows_and_takes_german_decimals() {
    let mut h = drag_harness("de", 1.5);
    assert_eq!(h.get_by_role(Role::SpinButton).value().as_deref(), Some("1,5"));
    type_into_drag(&mut h, "2,25");
    assert_eq!(*h.state(), 2.25);
    assert_eq!(h.get_by_role(Role::SpinButton).value().as_deref(), Some("2,25"));
}

#[test]
fn drag_value_in_arabic_digits() {
    let mut h = drag_harness("ar-EG", 1.5);
    assert_eq!(h.get_by_role(Role::SpinButton).value().as_deref(), Some("١٫٥"));
    type_into_drag(&mut h, "٢٬٥٠٠٫٥");
    assert_eq!(*h.state(), 2500.5);
}

#[test]
fn drag_value_keeps_the_apps_text_around_the_number() {
    let numbers = LocaleNumbers::from_tag("de");
    let mut h = drag_harness_with(0.0, move |ui, value| {
        LocalizedDragValue::new(DragValue::new(value).prefix("x: ").suffix(" °"), &numbers)
            .or_format(|v, _| (v == 0.0).then(|| "Auto".to_owned()))
            .or_parse(|t| (t.trim() == "Auto").then_some(0.0))
            .ui(ui)
    });
    assert_eq!(h.get_by_role(Role::SpinButton).value().as_deref(), Some("x: Auto °"));
    type_into_drag(&mut h, "2,5");
    assert_eq!(*h.state(), 2.5);
    assert_eq!(h.get_by_role(Role::SpinButton).value().as_deref(), Some("x: 2,5 °"));

    let numbers = LocaleNumbers::from_tag("de");
    let h = drag_harness_with(1.5, move |ui, value| {
        LocalizedDragValue::new(DragValue::new(value).prefix("x: ").suffix(" °"), &numbers).unit(Unit::new("{n} px")).ui(ui)
    });
    let shown = h.get_by_role(Role::SpinButton).value().unwrap_or_default();
    assert!(shown.contains("x: 1,5 °") && shown.trim_end().ends_with("px"), "a unit goes outside them: {shown:?}");
}

#[test]
fn drag_value_marks_text_it_cannot_read() {
    let mut h = drag_harness("de", 1.5);
    h.get_by_role(Role::SpinButton).click();
    h.run();
    h.get_by_role(Role::SpinButton).type_text("1x");
    h.run();
    assert!(h.query_by_label("Not a number").is_some(), "marked while editing");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), 1.5, "the value stays");
    assert!(h.query_by_label("Not a number").is_some(), "still marked after reverting");
    assert_eq!(h.get_by_role(Role::SpinButton).value().as_deref(), Some("1,5"));
}

#[test]
fn drag_value_does_not_mark_a_number_being_typed() {
    let mut h = drag_harness("de", 1.5);
    h.get_by_role(Role::SpinButton).click();
    h.run();
    h.get_by_role(Role::SpinButton).type_text("1.");
    h.run();
    assert!(h.query_by_label("Check the decimal and grouping marks").is_none());
}

fn field_harness(tag: &str, value: f64) -> Harness<'static, f64> {
    let numbers = LocaleNumbers::from_tag(tag);
    let mut h = Harness::builder().allow_missing_glyphs().build_ui_state(
        move |ui, value: &mut f64| {
            let label = ui.label("Gap");
            ui.add(NumberField::new(value, &numbers).range(0.0..=10000.0).step(0.5).unit(Unit::new("{n} mm"))).labelled_by(label.id);
        },
        value,
    );
    h.run();
    h
}

fn type_into_field(h: &mut Harness<'_, f64>, text: &str) {
    h.get_by_role(Role::TextInput).click();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
    h.get_by_role(Role::TextInput).type_text(text);
    h.run();
    h.key_press(Key::Enter);
    h.run();
}

#[test]
fn number_field_reads_on_commit() {
    let mut h = field_harness("de", 1.5);
    assert_eq!(h.get_by_role(Role::TextInput).value().as_deref(), Some("1,5"));
    type_into_field(&mut h, "1.234,5 mm");
    assert_eq!(*h.state(), 1234.5);
    assert_eq!(h.get_by_role(Role::TextInput).value().as_deref(), Some("1.234,5"));
    type_into_field(&mut h, "20000");
    assert_eq!(*h.state(), 10000.0, "clamped to the range");
}

#[test]
fn number_field_takes_full_width_and_persian_digits() {
    let mut h = field_harness("en", 1.0);
    type_into_field(&mut h, "１２．５");
    assert_eq!(*h.state(), 12.5);
    let mut h = field_harness("fa", 1.0);
    assert_eq!(h.get_by_role(Role::TextInput).value().as_deref(), Some("۱"));
    type_into_field(&mut h, "۱۲٫۵");
    assert_eq!(*h.state(), 12.5);
    assert_eq!(h.get_by_role(Role::TextInput).value().as_deref(), Some("۱۲٫۵"));
}

#[test]
fn number_field_keeps_text_it_cannot_read() {
    let mut h = field_harness("de", 1.5);
    type_into_field(&mut h, "1,5,5");
    assert_eq!(*h.state(), 1.5, "the value stays");
    assert_eq!(h.get_by_role(Role::TextInput).value().as_deref(), Some("1,5,5"), "the text stays to be fixed");
    type_into_field(&mut h, "2,5");
    assert_eq!(*h.state(), 2.5);
}

#[test]
fn number_field_rounds_before_clamping() {
    let mut h = field_harness("de", 1.5);
    type_into_field(&mut h, "9999,99999999");
    assert_eq!(*h.state(), 10000.0, "rounded to six decimals, then clamped");
    let numbers = LocaleNumbers::from_tag("de");
    let mut h = Harness::builder().build_ui_state(
        move |ui, value: &mut i32| {
            let label = ui.label("Count");
            ui.add(NumberField::new(value, &numbers).range(-10.0..=10.0)).labelled_by(label.id);
        },
        0,
    );
    h.run();
    for (text, want) in [("2,7", 3), ("2,5", 3), ("-0,5", -1), ("-0,4", 0), ("9,6", 10), ("10,5", 10)] {
        h.get_by_role(Role::TextInput).click();
        h.run();
        h.key_press_modifiers(Modifiers::COMMAND, Key::A);
        h.run();
        h.get_by_role(Role::TextInput).type_text(text);
        h.run();
        h.key_press(Key::Enter);
        h.run();
        assert_eq!(*h.state(), want, "{text}");
    }
}

#[test]
fn number_field_steps_with_arrow_keys() {
    let mut h = field_harness("de", 1.5);
    h.get_by_role(Role::TextInput).click();
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(*h.state(), 2.0);
    assert_eq!(h.get_by_role(Role::TextInput).value().as_deref(), Some("2"));
}
