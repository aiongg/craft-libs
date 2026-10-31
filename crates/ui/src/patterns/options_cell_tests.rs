//! The options bar's number, percent and unit cells.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, ScrollArea, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::OptionsBar;
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::localized::units::{LengthUnit, UnitContext};
use crate::theme::{Base, install};

fn builder(size: [f32; 2]) -> egui_kittest::HarnessBuilder<Cells> {
    Harness::<Cells>::builder().with_size(size).with_step_dt(1.0 / 60.0).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

struct Cells {
    percent: f64,
    pixels: f64,
    unit: LengthUnit,
    ppi: f64,
    events: Vec<EditEvent<f64>>,
    picked: Option<LengthUnit>,
}

impl Cells {
    fn new() -> Self {
        Self { percent: 40.0, pixels: 100.0, unit: LengthUnit::Pixels, ppi: 72.0, events: Vec::new(), picked: None }
    }

    fn keep(&mut self, edit: &EditEvent<f64>, target: fn(&mut Cells) -> &mut f64) {
        self.events.push(edit.clone());
        if let Some(&v) = edit.value() {
            *target(self) = v;
        }
    }
}

fn percent_cell(ui: &mut egui::Ui, cells: &mut Cells) {
    let numbers = LocaleNumbers::from_tag("en-US");
    let edit = OptionsBar::percent_cell(ui, "opacity", "Opacity", cells.percent, &numbers);
    if let Some(event) = &edit.event {
        cells.keep(event, |c| &mut c.percent);
    }
}

fn unit_cell(ui: &mut egui::Ui, cells: &mut Cells) {
    let context = UnitContext { pixels_per_inch: cells.ppi, ..UnitContext::default() };
    let out = OptionsBar::unit_cell(ui, "width", "Width", cells.pixels, 1.0..=300_000.0, cells.unit, &LengthUnit::ALL, context);
    if let Some(event) = &out.edit.event {
        cells.keep(event, |c| &mut c.pixels);
    }
    cells.picked = cells.picked.or(out.unit_picked);
}

fn pointer(h: &mut Harness<'_, Cells>, at: Pos2, button: PointerButton, pressed: bool) {
    h.event(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
    h.run();
}

// A 20-point caption drag from 40 % gives 50 % as one gesture; the press alone changes nothing.
#[test]
fn the_percent_caption_scrubs_half_a_point_per_point_from_the_press() {
    let mut h = builder([320.0, 80.0]).build_ui_state(percent_cell, Cells::new());
    h.run_steps(2);
    let caption = h.get_by_label("Opacity:").rect();
    let from = caption.center();
    h.event(Event::PointerMoved(from));
    h.run();
    pointer(&mut h, from, PointerButton::Primary, true);
    assert!(h.state().events.is_empty(), "the press alone is no edit: {:?}", h.state().events);
    for dx in [8.0, 20.0] {
        h.event(Event::PointerMoved(from + vec2(dx, 0.0)));
        h.run();
    }
    assert_eq!(h.state().percent, 50.0);
    pointer(&mut h, from + vec2(20.0, 0.0), PointerButton::Primary, false);
    h.run_steps(2);
    let events = &h.state().events;
    assert_eq!(events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count(), 1, "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { value: 50.0, .. })), "{events:?}");
    assert_eq!(h.state().percent, 50.0);
}

// The caption's scrub area runs over the gap to the box, not only over the words.
#[test]
fn the_caption_scrub_covers_the_gap_after_it() {
    let mut h = builder([320.0, 80.0]).build_ui_state(percent_cell, Cells::new());
    h.run_steps(2);
    let caption = h.get_by_label("Opacity:").rect();
    let in_gap = Pos2::new(caption.right() + 3.0, caption.center().y);
    h.event(Event::PointerMoved(in_gap));
    h.run();
    pointer(&mut h, in_gap, PointerButton::Primary, true);
    h.event(Event::PointerMoved(in_gap + vec2(20.0, 0.0)));
    h.run();
    assert_eq!(h.state().percent, 50.0);
}

// The ▾ opens a popup slider over 0..=100 named "Opacity".
#[test]
fn the_percent_cells_arrow_opens_a_slider_named_after_the_caption() {
    let mut h = builder([320.0, 200.0]).build_ui_state(percent_cell, Cells::new());
    h.run_steps(2);
    h.get_by_role_and_label(Role::Button, "Opacity").click();
    h.run_steps(3);
    let slider = h.get_by_role_and_label(Role::Slider, "Opacity");
    let data = slider.accesskit_node().data().clone();
    assert_eq!((data.min_numeric_value(), data.max_numeric_value()), (Some(0.0), Some(100.0)));
}

// Typing "1in" at 300 ppi reports 300; leaving a field showing 1.389 in keeps 100 px.
#[test]
fn the_unit_cell_reports_pixels_and_leaving_keeps_them_exact() {
    let mut h = builder([320.0, 80.0]).build_ui_state(unit_cell, Cells { ppi: 300.0, ..Cells::new() });
    h.run_steps(2);
    h.get_by_role_and_label(Role::SpinButton, "Width").click();
    h.run_steps(2);
    h.event(Event::Text("1in".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    assert_eq!(h.state().pixels, 300.0, "{:?}", h.state().events);

    let mut h = builder([320.0, 80.0]).build_ui_state(unit_cell, Cells { unit: LengthUnit::Inches, ..Cells::new() });
    h.run_steps(2);
    let field = h.get_by_role_and_label(Role::SpinButton, "Width");
    assert_eq!(field.accesskit_node().data().value(), Some("1.389 in"));
    field.click();
    h.run_steps(2);
    h.event(Event::Text("1.389".into()));
    h.run();
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().pixels, 100.0, "{:?}", h.state().events);
}

// The units menu pick comes back in unit_picked.
#[test]
fn a_secondary_click_on_a_unit_cell_picks_a_unit() {
    let mut h = builder([320.0, 320.0]).build_ui_state(unit_cell, Cells::new());
    h.run_steps(2);
    let at = h.get_by_role_and_label(Role::SpinButton, "Width").rect().center();
    h.event(Event::PointerMoved(at));
    h.run();
    pointer(&mut h, at, PointerButton::Secondary, true);
    pointer(&mut h, at, PointerButton::Secondary, false);
    h.run_steps(3);
    assert_eq!(h.get_by_label("Pixels").accesskit_node().data().toggled(), Some(egui::accesskit::Toggled::True), "the current unit is checked");
    let to = h.get_by_label("Millimeters").rect().center();
    h.event(Event::PointerMoved(to));
    h.run();
    pointer(&mut h, to, PointerButton::Primary, true);
    pointer(&mut h, to, PointerButton::Primary, false);
    h.run_steps(2);
    assert_eq!(h.state().picked, Some(LengthUnit::Millimetres));
}

// Tab into a cell outside a narrow bar's view scrolls the bar to centre it.
#[test]
fn a_control_tabbed_into_scrolls_into_view() {
    struct Bar {
        ids: Vec<egui::Id>,
        focus: Option<usize>,
        offset: f32,
        view: egui::Rect,
    }
    let state = Bar { ids: Vec::new(), focus: None, offset: 0.0, view: egui::Rect::NOTHING };
    let mut h = Harness::builder().with_size([200.0, 60.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, bar: &mut Bar| {
            let out = ScrollArea::horizontal().id_salt("bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    bar.ids = (0..9).map(|i| ui.add(egui::Button::new(format!("Option {i}")).min_size(vec2(100.0, 20.0))).id).collect();
                    if let Some(i) = bar.focus.take() {
                        ui.memory_mut(|m| m.request_focus(bar.ids[i]));
                    }
                });
                OptionsBar::reveal_focused_control(ui);
            });
            bar.offset = out.state.clamped_offset().x;
            bar.view = out.inner_rect;
        },
        state,
    );
    h.run_steps(2);
    assert_eq!(h.state().offset, 0.0);
    h.state_mut().focus = Some(4);
    h.run_steps(4);
    let button = h.get_by_label("Option 4").rect();
    let view = h.state().view;
    assert!(h.state().offset > 0.0, "the bar scrolled");
    assert!((button.center().x - view.center().x).abs() < 1.5, "centred: {button:?} in {view:?}");
}
