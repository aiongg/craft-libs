//! `Section` keeps its open state in the caller's binding or a saved slot and shows its body only while open; a
//! `TitleRow` too narrow for both texts drops the subtitle.

use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::sections::{Section, TitleRow};
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 110.0) };

/// The point on the disclosure at the start of a section's header.
fn disclosure() -> Pos2 {
    Pos2::new(AREA.left() + 15.0, AREA.top() + 14.0)
}

#[trial]
fn a_section_shows_its_body_while_its_binding_is_open_and_the_disclosure_flips_the_binding() {
    let mut frames = Frames::new();
    let (mut open, ran) = (true, Cell::new(false));
    let show = |ui: &mut Ui, open: &mut bool| {
        ran.set(false);
        Section::new("Options", Bind::bound(open)).id_salt("section").show_at(ui, AREA, |_, _| ran.set(true));
    };
    frames.idle(3, |ui| show(ui, &mut open));
    assert!(ran.get(), "an open section runs its body");
    click(&mut frames, disclosure(), &mut |ui| show(ui, &mut open));
    frames.idle(2, |ui| show(ui, &mut open));
    assert!(!open && !ran.get(), "the click writes the binding and the body stops running");
}

#[trial]
fn a_saved_section_keeps_its_state_without_a_binding_and_a_fixed_section_is_always_open() {
    let mut frames = Frames::new();
    let ran = Cell::new(false);
    let show = |ui: &mut Ui, fixed: bool| {
        ran.set(false);
        let section = if fixed {
            Section::fixed("Options")
        } else {
            Section::saved("Options", craft_ui::contract::SaveKey::new("craft_ui_tests.section_open"))
        };
        section.id_salt(("section", fixed)).show_at(ui, AREA, |_, _| ran.set(true));
    };
    frames.idle(3, |ui| show(ui, false));
    assert!(ran.get(), "a saved section starts open");
    click(&mut frames, disclosure(), &mut |ui| show(ui, false));
    frames.idle(2, |ui| show(ui, false));
    assert!(!ran.get(), "the saved slot holds the closed state");
    frames.idle(3, |ui| show(ui, true));
    assert!(ran.get(), "a fixed section has no state to close");
}

#[trial]
fn a_title_row_drops_its_subtitle_before_it_elides_the_title() {
    let texts = |width: f32| {
        let mut frames = Frames::new();
        let id = Cell::new(None);
        let area = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(width, 30.0));
        frames.idle(3, |ui| {
            let row = TitleRow::new("Options").subtitle("A subtitle that is long").id_salt("row").show_at(ui, area);
            id.set(Some(row.edit.response.id));
        });
        frames.tree().run_texts(id.get().expect("the row showed"))
    };
    assert_eq!(texts(400.0).len(), 2, "a wide row shows title and subtitle");
    assert_eq!(texts(90.0), ["Options"], "a narrow row keeps the whole title and drops the subtitle");
}
