//! The label column of a group: which rows read it.

use crate::label::group::Group;
use crate::label::kit::{Dial, Field, LabelledDial, Seen};
use craft_ui::contract::{LabelKind, Props as _, Show as _, ShowContainer as _, SideLabel};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 160.0) };

#[trial]
fn only_the_rows_directly_in_a_label_column_share_it() {
    let mut frames = Frames::new();
    let (short, long, caption) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let (mut a, mut b, mut c) = (1.0, 1.0, 1.0);
    frames.idle(4, |ui| {
        Group::column().id_salt("column").show_at(ui, ROW, |ui, _| {
            Field::new("Size", SideLabel::new()).id_salt("short").show(ui, |ui, _| {
                Dial::new(&mut a, &short).show(ui);
            });
            Field::new("Hue and saturation", SideLabel::new()).id_salt("long").show(ui, |ui, _| {
                Dial::new(&mut b, &long).show(ui);
            });
            // The leaf runs its own label step: a barrier, so its label keeps its own width.
            LabelledDial::new("Gap", SideLabel::new(), &mut c, &caption).id_salt("caption").show(ui);
        });
    });
    let start = |seen: &RefCell<Seen>| seen.borrow().rect.min.x;
    assert!((start(&short) - start(&long)).abs() < 0.5, "the field labels share the widest width");
    assert!(start(&caption) < start(&short) - 1.0, "the leaf's own label keeps its own width");
}

#[trial]
fn a_hidden_label_row_in_a_label_column_starts_its_control_in_the_control_column() {
    let mut frames = Frames::new();
    let (named, hidden) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let (mut a, mut b) = (1.0, 1.0);
    frames.idle(4, |ui| {
        Group::column().id_salt("hidden").show_at(ui, ROW, |ui, _| {
            Field::new("Opacity", SideLabel::new()).id_salt("named").show(ui, |ui, _| {
                Dial::new(&mut a, &named).show(ui);
            });
            Field::new("Lock", SideLabel::new().kind(LabelKind::Hidden)).id_salt("hidden").show(ui, |ui, _| {
                Dial::new(&mut b, &hidden).show(ui);
            });
        });
    });
    let (named, hidden) = (named.borrow().rect.min.x, hidden.borrow().rect.min.x);
    assert!((named - hidden).abs() < 0.5, "the hidden row's control lines up with the named row's: {named} vs {hidden}");
    assert!(hidden > ROW.min.x + 1.0, "the control does not start at the row start");
}
