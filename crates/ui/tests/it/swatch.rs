//! `Swatch`: a colour well named by its label whose value is the colour's hex; a click reports and, with `picker`,
//! files a colour request; swatches of a `RadioGroup` are one Tab stop of radio buttons; a cell is a list box option.

use craft_behavior::Mixed;
use craft_theme::Srgba;
use craft_ui::choice::{RadioGroup, Swatch, SwatchOutput};
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::patterns::form_dialogs::ColorRequest;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

#[trial]
fn a_swatch_is_a_colour_well_named_by_its_label_with_the_hex_as_value() {
    let ids: Rc<RefCell<Vec<Id>>> = Rc::default();
    let shown = ids.clone();
    let mut fixture = Fixture::new(move |ui| {
        let found = vec![
            Swatch::new("Opaque", BLUE).show(ui).response.id,
            Swatch::new("Translucent", Srgba { r: 1.0, g: 0.0, b: 0.0, a: 0.5 }).show(ui).response.id,
            Swatch::new("Several", Mixed::<Srgba>::Mixed).show(ui).response.id,
        ];
        *shown.borrow_mut() = found;
    });
    fixture.settle();
    let ids = ids.borrow().clone();
    let tree = fixture.tree();
    for (id, name, value) in [(ids[0], "Opaque", "#0000ff"), (ids[1], "Translucent", "#ff000080"), (ids[2], "Several", "Mixed")] {
        assert_eq!(tree.role(id), Some(Role::COLOR_WELL), "{name} is a colour well");
        assert_eq!(tree.name(id), Some(name));
        assert_eq!(tree.value(id), Some(value), "{name} reports its colour as its value");
    }
}

struct Clicks {
    clicked: Cell<usize>,
    requests: RefCell<Vec<ColorRequest>>,
}

fn click_rig(enabled: bool) -> (Fixture, Rc<Clicks>) {
    let clicks = Rc::new(Clicks { clicked: Cell::new(0), requests: RefCell::new(Vec::new()) });
    let seen = clicks.clone();
    let mut fixture = Fixture::new(move |ui| {
        let SwatchOutput { clicked, pick_color, .. } = Swatch::new("Fill", BLUE).picker("fill").enabled(enabled).show(ui);
        seen.clicked.set(seen.clicked.get() + usize::from(clicked));
        seen.requests.borrow_mut().extend(pick_color);
    });
    fixture.settle();
    (fixture, clicks)
}

#[trial]
fn a_click_files_one_colour_request_with_the_key_and_the_colour() {
    let (mut fixture, clicks) = click_rig(true);
    let well = fixture.by_label("Fill").expect("shown");
    fixture.click(&well);
    assert_eq!(clicks.clicked.get(), 1);
    let requests = clicks.requests.borrow();
    assert_eq!(requests.len(), 1);
    assert_eq!((requests[0].key.as_str(), requests[0].color), ("fill", BLUE));
    drop(requests);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::Space, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(clicks.requests.borrow().len(), 2, "Space on the focused well files a request too");
}

#[trial]
fn a_disabled_swatch_reports_no_click_and_files_no_request() {
    let (mut fixture, clicks) = click_rig(false);
    let well = fixture.by_label("Fill").expect("shown");
    fixture.click(&well);
    assert_eq!((clicks.clicked.get(), clicks.requests.borrow().len()), (0, 0));
}

fn group(value: Rc<Cell<u8>>, count: u8) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let mut picked = value.get();
        RadioGroup::new("Colour", Bind::bound(&mut picked)).show(ui, |ui, _| {
            for n in 0..count {
                Swatch::new(format!("Colour {n}"), Srgba::rgb(f32::from(n) / 4.0, 0.0, 0.0)).radio(n).show(ui);
            }
        });
        value.set(picked);
    });
    fixture.settle();
    fixture
}

#[trial]
fn swatches_in_a_radio_group_are_one_tab_stop_arrows_move_and_space_picks() {
    let value = Rc::new(Cell::new(0u8));
    let mut fixture = group(value.clone(), 3);
    let radios = fixture.by_role(Role::RADIO_BUTTON);
    assert_eq!(radios.len(), 3, "three radio nodes");
    let checked: Vec<_> = radios.iter().filter(|radio| fixture.tree().toggled_of(radio.node) == Some(Mixed::Same(true))).collect();
    assert_eq!(checked.len(), 1, "one of them is checked");
    for key in [Key::Tab, Key::ArrowRight, Key::ArrowRight, Key::Space] {
        fixture.key(key, Modifiers::NONE);
        fixture.idle(2);
    }
    assert_eq!(value.get(), 2, "Tab enters the group once, two arrows reach the third swatch, Space picks it");
}

#[trial]
fn a_cell_swatch_is_a_list_box_option() {
    let mut fixture = Fixture::new(|ui| {
        Swatch::new("Cell", BLUE).cell().selected(true).show(ui);
    });
    fixture.settle();
    assert_eq!(fixture.by_role(Role::LIST_BOX_OPTION).len(), 1);
}
