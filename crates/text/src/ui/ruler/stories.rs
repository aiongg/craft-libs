//! Stories of the text ruler: Word's ruler with stops and indents, the Tabs panel's with the selector and a picked
//! stop, and a table row's column borders.

use super::items::FIRST;
use super::{RulerValues, TabAlign, TabLeader, TabStop, TextRuler};
use crate::ui::length::DocUnit;
use craft_behavior::Mixed;
use craft_ui::contract::{Bind, Show as _, Validity};
use craft_ui::runtime::env::{Message, UnitKind};
use craft_ui::testing::{Drive, Grip, StoryData, StoryOutput, story};
use egui::Ui;

/// The column's width, points: a 6.5 in text width.
const COLUMN: f32 = 468.0;
/// The widest left indent the invalid story accepts, points.
const MOST_INDENT: f32 = 216.0;

fn stop(position: f32, align: TabAlign) -> TabStop {
    TabStop { position, align, leader: TabLeader::None }
}

fn word() -> RulerValues {
    RulerValues {
        stops: vec![stop(72.0, TabAlign::Left), stop(180.0, TabAlign::Center), stop(288.0, TabAlign::Right), stop(360.0, TabAlign::Decimal)],
        left_indent: 36.0,
        first_line_indent: -18.0,
        right_indent: 18.0,
    }
}

fn inches() -> DocUnit {
    DocUnit::new(UnitKind::Inches, 96.0)
}

#[story(TextRuler<'static>, covers = [Default], drive = Drive::Drag(Grip::new(FIRST)))]
fn with_stops_and_indents(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", word());
    TextRuler::new("Ruler", Bind::bound(&mut *values), COLUMN).unit(inches()).show(ui).into()
}

#[story(TextRuler<'static>, covers = [Mixed], drive = Drive::Never)]
fn paragraphs_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", Mixed::<RulerValues>::Mixed);
    TextRuler::new("Ruler", Bind::bound_mixed(&mut *values), COLUMN).unit(inches()).show(ui).into()
}

#[story(TextRuler<'static>, covers = [Invalid], drive = Drive::Never)]
fn indent_too_wide(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", RulerValues { left_indent: MOST_INDENT + 36.0, ..word() });
    let narrow = |values: &RulerValues| {
        if values.left_indent > MOST_INDENT { Validity::Refuse(Message::new("The indent leaves too little room")) } else { Validity::Ok }
    };
    TextRuler::new("Ruler", Bind::bound(&mut *values).check(narrow), COLUMN).unit(inches()).show(ui).into()
}

#[story(TextRuler<'static>, covers = [], drive = Drive::Never)]
fn tabs_panel(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", word());
    TextRuler::new("Ruler", Bind::bound(&mut *values), COLUMN).unit(inches()).kind(TabAlign::Decimal).picked_stop(1).show(ui).into()
}

#[story(TextRuler<'static>, covers = [], drive = Drive::Never)]
fn table_column_borders(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", RulerValues::default());
    TextRuler::new("Ruler", Bind::bound(&mut *values), COLUMN).unit(inches()).columns(&[0.0, 156.0, 312.0, COLUMN]).show(ui).into()
}
