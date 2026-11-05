//! The paint chip in a kittest harness: its tabs and what a screen reader hears.

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{
    FillKind, GradientPreview, ImagePreview, Paint, PaintChip, PaintLink, PaintLinkKind, PaintList, PaintListEvent, PaintListOptions, PaintRow,
};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

struct Host {
    paint: Paint,
    gradient: bool,
    images: bool,
    events: Vec<EditEvent<Paint>>,
}

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

const FALLBACK: Srgba = Srgba::rgb(0.2, 0.4, 0.8);

fn chip(paint: Paint, gradient: bool) -> Harness<'static, Host> {
    let mut harness = harness().build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let mut chip = PaintChip::new("fill", "Fill", &host.paint, FALLBACK, &numbers);
            if host.gradient {
                chip = chip.gradient_editor(GradientPreview { stops: vec![(0.0, FALLBACK), (1.0, Srgba::rgb(1.0, 1.0, 1.0))] }, |_| None);
            }
            if host.images {
                let tile = ImagePreview { name: "Bubbles".to_owned(), texture: None, size: [8.0, 8.0] };
                chip = chip.pattern_editor(tile.clone(), |_| None).picture_editor(ImagePreview { name: "photo.png".to_owned(), ..tile }, |_| None);
            }
            if let Some(event) = chip.show(ui).event {
                if let EditEvent::Set { value, .. } = &event {
                    host.paint = value.clone();
                }
                host.events.push(event);
            }
        },
        Host { paint, gradient, images: false, events: Vec::new() },
    );
    harness.run();
    harness
}

fn chip_with_images(paint: Paint) -> Harness<'static, Host> {
    let mut harness = chip(paint, true);
    harness.state_mut().images = true;
    harness.run();
    harness
}

fn tab_labels(harness: &Harness<'_, Host>) -> Vec<String> {
    harness.query_all_by_role(Role::RadioButton).filter_map(|n| n.accesskit_node().label()).collect()
}

fn open(harness: &mut Harness<'_, Host>) {
    harness.get_by_role(Role::ColorWell).click();
    harness.run();
}

fn tab<'h, State>(harness: &'h Harness<'_, State>, label: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by(|n| n.role() == Role::RadioButton && n.label().as_deref() == Some(label))
}

/// Switching to Solid starts from the fallback colour and to No Color sets none.
#[test]
fn switching_tabs_sets_the_paint() {
    let mut harness = chip(Paint::None, true);
    open(&mut harness);
    tab(&harness, "Solid").click();
    harness.run();
    assert_eq!(harness.state().paint, Paint::Solid(FALLBACK));
    tab(&harness, "No Color").click();
    harness.run();
    assert_eq!(harness.state().paint, Paint::None);
    tab(&harness, "Gradient").click();
    harness.run();
    assert!(matches!(harness.state().paint, Paint::Gradient(ref g) if g.stops.len() == 2), "{:?}", harness.state().paint);
}

/// Without a gradient editor there is no Gradient tab.
#[test]
fn no_gradient_tab_without_an_editor() {
    let mut harness = chip(Paint::Solid(FALLBACK), false);
    open(&mut harness);
    let tabs: Vec<_> = harness.query_all_by_role(Role::RadioButton).filter_map(|n| n.accesskit_node().label()).collect();
    assert_eq!(tabs, ["No Color", "Solid"]);
}

/// The chip is a ColorWell whose value names the paint.
#[test]
fn the_chip_names_its_paint() {
    let value = |paint| {
        let harness = chip(paint, true);
        let well = harness.get_by_role(Role::ColorWell);
        assert_eq!(well.accesskit_node().label().as_deref(), Some("Fill"));
        well.accesskit_node().value().map(|v| v.to_owned())
    };
    assert_eq!(value(Paint::None).as_deref(), Some("No Color"));
    assert_eq!(value(Paint::Gradient(GradientPreview::default())).as_deref(), Some("Gradient"));
    assert_eq!(value(Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0))).as_deref(), Some("#ff0000"));
}

// Task SH-T14: the Pattern and Picture tabs and the paint list's row actions.

/// `kinds()` lists Pattern and Picture only with their editors; a chip with both shows five tabs,
/// and a switch to Pattern reports `Set(Paint::Pattern(default))`.
#[test]
fn paint_chip_pattern_and_picture_tabs_only_when_offered() {
    let numbers = LocaleNumbers::from_tag("en");
    let plain = PaintChip::new("fill", "Fill", &Paint::None, FALLBACK, &numbers);
    assert_eq!(plain.kinds(), [FillKind::None, FillKind::Solid]);
    let tile = ImagePreview { name: "Bubbles".to_owned(), texture: None, size: [8.0, 8.0] };
    let both = PaintChip::new("fill", "Fill", &Paint::None, FALLBACK, &numbers).pattern_editor(tile.clone(), |_| None).picture_editor(tile, |_| None);
    assert_eq!(both.kinds(), [FillKind::None, FillKind::Solid, FillKind::Pattern, FillKind::Picture]);

    let mut harness = chip_with_images(Paint::Solid(FALLBACK));
    open(&mut harness);
    assert_eq!(tab_labels(&harness), ["No Color", "Solid", "Gradient", "Pattern", "Picture"]);
    tab(&harness, "Pattern").click();
    harness.run();
    assert_eq!(harness.state().paint, Paint::Pattern(ImagePreview { name: "Bubbles".to_owned(), texture: None, size: [8.0, 8.0] }));
    assert_eq!(FillKind::of(&harness.state().paint), FillKind::Pattern);
    tab(&harness, "Picture").click();
    harness.run();
    assert!(matches!(&harness.state().paint, Paint::Picture(p) if p.name == "photo.png"), "{:?}", harness.state().paint);
    let well = harness.get_by_role(Role::ColorWell);
    assert_eq!(well.accesskit_node().value().as_deref(), Some("Picture: photo.png"));
}

/// Without `pattern_editor` and `picture_editor` the chip and popover read as before: three tabs,
/// the same AccessKit values.
#[test]
fn paint_chip_without_new_tabs_is_unchanged() {
    let mut harness = chip(Paint::Solid(FALLBACK), true);
    open(&mut harness);
    assert_eq!(tab_labels(&harness), ["No Color", "Solid", "Gradient"]);
    let well = harness.get_by_role(Role::ColorWell);
    assert_eq!(well.accesskit_node().value().as_deref(), Some("#3366cc"));
}

struct ListHost {
    paints: Vec<(String, Paint)>,
    rows: Vec<PaintRow>,
    options: PaintListOptions,
    events: Vec<PaintListEvent>,
}

fn list(options: PaintListOptions) -> Harness<'static, ListHost> {
    let paints = vec![("Red".to_owned(), Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0))), ("Sky".to_owned(), Paint::Solid(Srgba::rgb(0.0, 0.5, 1.0)))];
    let rows = vec![PaintRow::default(), PaintRow::linked(PaintLink { name: "Brand".to_owned(), kind: PaintLinkKind::Style })];
    let mut harness =
        Harness::<ListHost>::builder().with_size([720.0, 320.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut ListHost| {
                let shown = PaintList::new("paints", "Paints", &host.paints).options(host.options).rows(&host.rows).show(ui);
                host.events.extend(shown.events);
            },
            ListHost { paints, rows, options, events: Vec::new() },
        );
    harness.run();
    harness
}

fn by_label<'h>(harness: &'h Harness<'_, ListHost>, role: Role, label: &'h str, nth: usize) -> egui_kittest::Node<'h> {
    let Some(node) = harness.query_all_by_role_and_label(role, label).nth(nth) else { panic!("no {role:?} named {label} at {nth}") };
    node
}

/// With `PaintListOptions::SELECTION_COLORS`: a chip click and a picker change report `Edit`, a
/// typed hex `Hex`, a typed opacity an `Opacity` edit, Bind and Detach `Bind` and `Detach`, the
/// select button `SelectUsers`.
#[test]
fn paint_list_rows_edit_hex_opacity_bind_and_select_users() {
    let mut harness = list(PaintListOptions::SELECTION_COLORS);

    by_label(&harness, Role::ColorWell, "Edit color", 0).click();
    harness.run();
    tab(&harness, "No Color").click();
    harness.run();
    assert!(
        matches!(harness.state().events.last(), Some(PaintListEvent::Edit { row: 0, event }) if event.value() == Some(&Paint::None)),
        "{:?}",
        harness.state().events
    );

    harness.state_mut().events.clear();
    by_label(&harness, Role::TextInput, "Color", 1).focus();
    harness.run();
    by_label(&harness, Role::TextInput, "Color", 1).type_text("00ff00");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(
        harness
            .state()
            .events
            .iter()
            .any(|e| matches!(e, PaintListEvent::Hex { row: 1, color } if color.g > 0.99 && color.r < 0.01 && color.a == 1.0)),
        "{:?}",
        harness.state().events
    );

    harness.state_mut().events.clear();
    by_label(&harness, Role::SpinButton, "Opacity", 0).focus();
    harness.run();
    by_label(&harness, Role::SpinButton, "Opacity", 0).type_text("50");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(
        harness
            .state()
            .events
            .iter()
            .any(|e| matches!(e, PaintListEvent::Opacity { row: 0, event } if event.value().is_some_and(|v| (v - 0.5).abs() < 1e-4))),
        "{:?}",
        harness.state().events
    );

    harness.state_mut().events.clear();
    by_label(&harness, Role::Button, "Color styles", 0).click();
    by_label(&harness, Role::Button, "Detach style", 0).click();
    by_label(&harness, Role::Button, "Select layers with this color", 1).click();
    harness.run();
    assert_eq!(harness.state().events, [PaintListEvent::Bind { row: 0 }, PaintListEvent::Detach { row: 1 }, PaintListEvent::SelectUsers { row: 1 }]);
}

/// A rename reports `Rename` (and `renamed`); Remove reports `Remove { row }`; Add reports `Add`.
#[test]
fn paint_list_rename_and_remove_are_set_events() {
    let mut harness = list(PaintListOptions { remove: true, add: true, ..PaintListOptions::default() });
    by_label(&harness, Role::Button, "Remove", 1).click();
    harness.run();
    by_label(&harness, Role::Button, "Add", 0).click();
    harness.run();
    assert_eq!(harness.state().events, [PaintListEvent::Remove { row: 1 }, PaintListEvent::Add]);

    let mut plain = list(PaintListOptions::default());
    plain.get_by_label_contains("Red").click();
    plain.run();
    plain.key_press(egui::Key::F2);
    plain.run();
    by_label(&plain, Role::TextInput, "Red", 0).type_text("Crimson");
    plain.key_press(egui::Key::Enter);
    plain.run();
    assert!(matches!(plain.state().events.as_slice(), [PaintListEvent::Rename { row: 0, name }] if name == "Crimson"), "{:?}", plain.state().events);
}

/// A chip that was Solid, then None, switches back to the colour it had, not the fallback.
#[test]
fn switching_back_to_solid_restores_the_last_solid_colour() {
    let mut harness = chip(Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0)), true);
    open(&mut harness);
    tab(&harness, "No Color").click();
    harness.run();
    tab(&harness, "Solid").click();
    harness.run();
    assert_eq!(harness.state().paint, Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0)));
}

/// A row that was never solid switches to Solid at the list's `fallback`, not at black.
#[test]
fn paint_list_row_switches_to_the_host_fallback() {
    const HOST_DEFAULT: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
    let paints = vec![("Nothing".to_owned(), Paint::None)];
    let mut harness =
        Harness::<ListHost>::builder().with_size([720.0, 320.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut ListHost| {
                let shown = PaintList::new("paints", "Paints", &host.paints).options(host.options).fallback(HOST_DEFAULT).show(ui);
                host.events.extend(shown.events);
            },
            ListHost { paints, rows: Vec::new(), options: PaintListOptions { edit: true, ..PaintListOptions::default() }, events: Vec::new() },
        );
    harness.run();
    by_label(&harness, Role::ColorWell, "Edit color", 0).click();
    harness.run();
    tab(&harness, "Solid").click();
    harness.run();
    assert!(
        matches!(harness.state().events.last(), Some(PaintListEvent::Edit { row: 0, event }) if event.value() == Some(&Paint::Solid(HOST_DEFAULT))),
        "{:?}",
        harness.state().events
    );
}
