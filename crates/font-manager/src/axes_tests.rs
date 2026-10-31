//! Axes: reading and the sliders.

use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::axes::*;

const WGHT: [u8; 4] = *b"wght";
const WDTH: [u8; 4] = *b"wdth";
const GRAD: [u8; 4] = *b"GRAD";

fn fixed(v: f32) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

/// A font with only `fvar` and `name` tables: Weight 100..900 (default 400), Width 75..100
/// (default 100), a hidden GRAD axis, and the instances Regular and Bold.
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

// A static face has no axes; a variable face lists them in font order with min, default and max.
#[test]
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
    values: Vec<([u8; 4], f32)>,
    changed: Vec<([u8; 4], f32)>,
    settled: Vec<([u8; 4], f32)>,
    picked: Option<String>,
}

fn harness() -> Harness<'static, State> {
    let data = variable_font();
    let state = State {
        axes: read_axes(&data, 0),
        instances: read_named_instances(&data, 0),
        values: vec![(WGHT, 400.0)],
        changed: Vec::new(),
        settled: Vec::new(),
        picked: None,
    };
    Harness::<State>::builder().with_size([320.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, s: &mut State| {
            let r = AxisSliders::new(&s.axes, &mut s.values).instances(&s.instances).show(ui);
            s.changed.extend(r.changed);
            s.settled.extend(r.settled);
            s.picked = r.instance.map(|i| i.name).or(s.picked.take());
        },
        state,
    )
}

fn button(pos: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

// Dragging the Weight slider reports one change on release; picking an instance sets every axis.
#[test]
fn slider_drag_is_one_edit_and_instance_sets_all_axes() {
    let mut harness = harness();
    // Hidden axes get no row; the others do.
    assert_eq!(harness.query_all_by_role(Role::Slider).count(), 2);

    let rect = harness.get_all_by_role(Role::Slider).next().map(|n| n.rect()).expect("weight slider");
    let at = |f: f32| pos2(rect.left() + rect.width() * f, rect.center().y);
    harness.event(Event::PointerMoved(at(0.5)));
    harness.run();
    harness.event(button(at(0.5), true));
    harness.run();
    harness.event(Event::PointerMoved(at(0.8)));
    harness.run();
    assert!(!harness.state().changed.is_empty(), "the drag previews");
    assert!(harness.state().settled.is_empty(), "nothing is settled mid-drag");
    harness.event(button(at(0.8), false));
    harness.run();
    let s = harness.state();
    assert_eq!(s.settled.len(), 1, "one edit per gesture: {:?}", s.settled);
    let (tag, value) = s.settled[0];
    assert_eq!(tag, WGHT);
    assert!(value > 600.0 && value.fract() == 0.0, "whole units on a wide axis: {value}");
    assert_eq!(s.values.iter().find(|(t, _)| *t == WGHT).map(|v| v.1), Some(value));

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Bold").click();
    harness.run();
    let s = harness.state();
    assert_eq!(s.picked.as_deref(), Some("Bold"));
    for (tag, v) in [(WGHT, 700.0), (WDTH, 100.0), (GRAD, 0.0)] {
        assert_eq!(s.values.iter().find(|(t, _)| *t == tag).map(|x| x.1), Some(v), "{}", String::from_utf8_lossy(&tag));
        assert!(s.settled.contains(&(tag, v)));
    }
}
