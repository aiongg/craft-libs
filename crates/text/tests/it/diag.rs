//! TEMPORARY diagnostics.

use craft_behavior::PartKey;
use craft_text::ui::DocUnit;
use craft_text::ui::character::{CharacterControls, CharacterValues, Kerning};
use craft_text::ui::ruler::{RulerValues, TabAlign, TabLeader, TabStop, TextRuler};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

#[trial]
fn diag_ruler() {
    let stop = |position: f32, align| TabStop::new(position, align, TabLeader::None);
    let mut values = RulerValues::default();
    values.stops = vec![stop(72.0, TabAlign::Left), stop(180.0, TabAlign::Center)];
    values.left_indent = 36.0;
    values.first_line_indent = -18.0;
    values.right_indent = 18.0;
    let held = Rc::new(RefCell::new(values));
    let shown = held.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut v = shown.borrow_mut();
        let _ = TextRuler::new("Ruler", Bind::bound(&mut *v), 468.0).unit(DocUnit::new(UnitKind::Inches, 96.0)).show(ui);
    });
    fixture.settle();
    let id = fixture.root_id().expect("root");
    let mut out = format!("shown {:?}\n", fixture.shown());
    for key in ["first", "hanging", "left", "right"] {
        out +=
            &format!("{key}: rect {:?} node {:?}\n", fixture.part_rect(id, PartKey::new(key)), fixture.part(id, PartKey::new(key)).map(|t| t.rect));
    }
    let grip = fixture.part(id, PartKey::new("first")).expect("first node");
    fixture.drag(&grip, vec2(6.0, 0.0));
    out += &format!("after drag: {:?}\n", held.borrow());
    panic!("DIAG\n{out}");
}

#[trial]
fn diag_kerning() {
    let mut values = CharacterValues::default();
    values.kerning = Some(Kerning::Metrics);
    values.size = Some(12.0);
    let families = ["Hack".to_owned()];
    let mut fixture = Fixture::new(move |ui| {
        let _ = CharacterControls::new("Character", &values, &families, &families).show(ui);
    });
    fixture.settle();
    let dump = |fixture: &Fixture, title: &str| {
        let tree = fixture.tree();
        let mut s = format!("== {title}\n");
        for key in tree.keys() {
            if let Some(name) = tree.name_of(key) {
                s += &format!("{:?} {:?} value={:?}\n", tree.role_of(key), name, tree.value_of(key));
            }
        }
        s
    };
    let mut out = dump(&fixture, "settled");
    let target = fixture.by_label("Kerning").expect("kerning");
    out += &format!("kerning target {:?}\n", target.rect);
    fixture.click(&target);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    out += &dump(&fixture, "after click and Alt+Down");
    panic!("DIAG\n{out}");
}
