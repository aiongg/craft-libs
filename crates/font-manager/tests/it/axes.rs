//! `AxisSliders` and the axis readers.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::EditEvent;
use craft_font_manager::axes::{AxisSliders, AxisValues, FontAxis, NamedInstance, axis_label, matching_instance, read_axes, read_named_instances};
use craft_ui::contract::{Bind, Role};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

/// The weight one arrow press above the default, on an axis wide enough to step by whole units.
const ONE_UNIT_UP: f32 = 401.0;

const WGHT: [u8; 4] = *b"wght";
const WDTH: [u8; 4] = *b"wdth";
const GRAD: [u8; 4] = *b"GRAD";

fn fixed(v: f32) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

/// A font with only `fvar` and `name` tables: Weight 100..900 (default 400), Width 75..100 (default 100), a hidden
/// GRAD axis, and the instances Regular and Bold.
fn variable_font() -> Vec<u8> {
    let axes = [(WGHT, 100.0, 400.0, 900.0, 0u16, 256u16), (WDTH, 75.0, 100.0, 100.0, 0, 0), (GRAD, 0.0, 0.0, 100.0, 1, 0)];
    let instances = [(257u16, [400.0, 100.0, 0.0]), (258, [700.0, 100.0, 0.0])];
    let mut fvar = Vec::new();
    for v in [1u16, 0, 16, 2, 3, 20, 2, 4 + 4 * 3] {
        fvar.extend(v.to_be_bytes());
    }
    for (tag, min, default, max, flags, name) in axes {
        fvar.extend(tag);
        for v in [min, default, max] {
            fvar.extend(fixed(v));
        }
        fvar.extend(flags.to_be_bytes());
        fvar.extend(name.to_be_bytes());
    }
    for (name, coords) in instances {
        fvar.extend(name.to_be_bytes());
        fvar.extend(0u16.to_be_bytes());
        for c in coords {
            fvar.extend(fixed(c));
        }
    }
    let strings = [(256u16, "Weight"), (257, "Regular"), (258, "Bold")];
    let mut name = Vec::new();
    for v in [0u16, strings.len() as u16, 6 + 12 * strings.len() as u16] {
        name.extend(v.to_be_bytes());
    }
    let mut pool = Vec::new();
    for (id, text) in strings {
        let utf16: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        for v in [3u16, 1, 0x409, id, utf16.len() as u16, pool.len() as u16] {
            name.extend(v.to_be_bytes());
        }
        pool.extend(utf16);
    }
    name.extend(pool);
    let tables = [(*b"fvar", fvar), (*b"name", name)];
    let mut font = Vec::new();
    font.extend(0x0001_0000u32.to_be_bytes());
    font.extend((tables.len() as u16).to_be_bytes());
    font.extend([0u8; 6]);
    let mut offset = 12 + 16 * tables.len();
    let mut body = Vec::new();
    for (tag, data) in &tables {
        font.extend(tag);
        font.extend(0u32.to_be_bytes());
        font.extend((offset as u32).to_be_bytes());
        font.extend((data.len() as u32).to_be_bytes());
        body.extend(data);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        offset = 12 + 16 * tables.len() + body.len();
    }
    font.extend(body);
    font
}

/// A static face has no axes; a variable face lists them in font order with min, default and max.
#[trial]
fn read_axes_of_a_variable_face() {
    assert!(read_axes(b"not a font", 0).is_empty());
    assert!(read_named_instances(b"not a font", 0).is_empty());

    let data = variable_font();
    let axes = read_axes(&data, 0);
    let summary: Vec<_> = axes.iter().map(|a| (a.tag, a.min, a.default, a.max, a.hidden)).collect();
    assert_eq!(summary, [(WGHT, 100.0, 400.0, 900.0, false), (WDTH, 75.0, 100.0, 100.0, false), (GRAD, 0.0, 0.0, 100.0, true)]);
    assert_eq!(axis_label(&axes[0]), "Weight", "the font's own name");
    assert_eq!(axis_label(&axes[1]), "Width", "the registered axis's name");
    assert_eq!(axis_label(&axes[2]), "GRAD", "the tag");

    let instances = read_named_instances(&data, 0);
    assert_eq!(instances.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["Regular", "Bold"]);
    assert_eq!(instances[1].coordinates, [(WGHT, 700.0), (WDTH, 100.0), (GRAD, 0.0)]);
    let bold = [(WGHT, 700.0), (WDTH, 100.0), (GRAD, 0.0)];
    assert_eq!(matching_instance(&instances, &bold).map(|i| i.name.as_str()), Some("Bold"));
    assert!(matching_instance(&instances, &[(WGHT, 650.0), (WDTH, 100.0), (GRAD, 0.0)]).is_none());
}

struct State {
    axes: Vec<FontAxis>,
    instances: Vec<NamedInstance>,
    values: AxisValues,
    edits: Vec<EditEvent<AxisValues>>,
    picked: Option<String>,
}

fn rig(values: AxisValues, drawn: Option<(f32, bool)>) -> (Fixture, Rc<RefCell<State>>) {
    let data = variable_font();
    let state = Rc::new(RefCell::new(State {
        axes: read_axes(&data, 0),
        instances: read_named_instances(&data, 0),
        values,
        edits: Vec::new(),
        picked: None,
    }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut state = shown.borrow_mut();
        let State { axes, instances, values, edits, picked } = &mut *state;
        let mut sliders = AxisSliders::new("Axes", axes, Bind::bound(values)).instances(instances);
        if let Some((weight, italic)) = drawn {
            sliders = sliders.weight(weight).italic(italic);
        }
        let out = sliders.show(ui);
        edits.extend(out.edit);
        *picked = out.instance.map(|i| i.name).or(picked.take());
    });
    fixture.settle();
    (fixture, state)
}

/// A key step on the Weight slider is one edit of the values; picking an instance sets every axis in one edit, and
/// hidden axes get no slider.
#[trial]
fn slider_drag_is_one_edit_and_instance_sets_all_axes() {
    let (mut fixture, state) = rig(vec![(WGHT, 400.0)], None);
    let sliders = fixture.by_role(Role::SLIDER);
    assert_eq!(sliders.len(), 2, "the hidden axis has no row");

    fixture.access(AccessActionKind::Focus, &sliders[0]);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(3);
    let weight = state.borrow().values.iter().find(|(t, _)| *t == WGHT).map(|v| v.1);
    assert_eq!(weight, Some(ONE_UNIT_UP), "whole units on a wide axis");
    assert!(state.borrow().edits.iter().any(|e| e.value().is_some_and(|v| v.contains(&(WGHT, ONE_UNIT_UP)))));

    let before = state.borrow().edits.len();
    let select = fixture.by_label("Style").expect("the instance list");
    fixture.click(&select);
    fixture.idle(3);
    let bold = fixture.by_label("Bold").expect("the Bold option");
    fixture.click(&bold);
    fixture.idle(6);
    let state = state.borrow();
    assert_eq!(state.picked.as_deref(), Some("Bold"));
    for (tag, v) in [(WGHT, 700.0), (WDTH, 100.0), (GRAD, 0.0)] {
        assert_eq!(state.values.iter().find(|(t, _)| *t == tag).map(|x| x.1), Some(v), "{}", String::from_utf8_lossy(&tag));
    }
    let set = state.edits[before..].iter().filter(|e| matches!(e, EditEvent::Set { .. })).count();
    assert_eq!(set, 1, "the instance is one edit: {:?}", &state.edits[before..]);
}

/// With no stored coordinate the sliders show what the style draws: a Bold face reads Weight 700 and the Bold
/// instance, not the axis default.
#[trial]
fn unset_axes_show_the_drawn_style() {
    let (fixture, state) = rig(Vec::new(), Some((700.0, false)));
    assert!(fixture.texts().iter().any(|painted| painted.text == "700"), "the Weight field reads 700");
    assert!(state.borrow().values.is_empty(), "showing a drawn value stores nothing");
    assert!(fixture.by_label("Custom").is_none(), "the instance list matches Bold");
}
