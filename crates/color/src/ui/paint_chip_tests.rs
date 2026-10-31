//! The paint chip in a kittest harness: its tabs and what a screen reader hears.

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{GradientPreview, Paint, PaintChip};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

struct Host {
    paint: Paint,
    gradient: bool,
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
            if let Some(event) = chip.show(ui).event {
                if let EditEvent::Set { value, .. } = &event {
                    host.paint = value.clone();
                }
                host.events.push(event);
            }
        },
        Host { paint, gradient, events: Vec::new() },
    );
    harness.run();
    harness
}

fn open(harness: &mut Harness<'_, Host>) {
    harness.get_by_role(Role::ColorWell).click();
    harness.run();
}

fn tab<'h>(harness: &'h Harness<'_, Host>, label: &'h str) -> egui_kittest::Node<'h> {
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
