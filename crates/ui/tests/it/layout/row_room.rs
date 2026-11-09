//! The row-room rule: a container offers its children the room it was offered, and a child that fills an axis
//! takes that room only where the container is given the axis; on an axis the container sizes from its content,
//! the child takes its measured size.

use craft_behavior::ColumnWidth;
use craft_ui::buttons::Button;
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, RowAlign, Scroll, Show as _, ShowContainer as _, SideLabel};
use craft_ui::display::{Divider, Text};
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::sections::Section;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROOM: Vec2 = Vec2::new(800.0, 600.0);
const SLACK: f32 = 0.5;
/// The rect a fill row is shown at.
const AT: Vec2 = Vec2::new(300.0, 120.0);

/// The heights of a row, its divider and its button when the row holds both under `row`.
fn heights(row: impl Fn() -> Row + 'static) -> (f32, f32, f32) {
    let seen = Rc::new(Cell::new((0.0, 0.0, 0.0)));
    let out = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            let inner = row().show(ui, |ui, _| {
                let button = Button::new("Apply").show(ui).rect.height();
                let divider = Divider::vertical().id_salt("divider").show(ui).rect.height();
                (button, divider)
            });
            let (button, divider) = inner.inner.unwrap_or_default();
            out.set((inner.output.rect.height(), divider, button));
        });
    });
    fixture.room(ROOM).settle();
    seen.get()
}

#[trial]
fn a_divider_in_a_fill_width_row_is_as_high_as_the_row_and_its_button() {
    let (row, divider, button) = heights(|| Row::new().fill_width());
    assert!((row - button).abs() <= SLACK && (divider - button).abs() <= SLACK, "row {row}, divider {divider}, button {button}");
}

#[trial]
fn a_divider_in_a_scrolling_fill_width_row_is_as_high_as_the_row_and_its_button() {
    let (row, divider, button) = heights(|| Row::new().fill_width().scroll(Scroll::horizontal()));
    assert!((row - button).abs() <= SLACK && (divider - button).abs() <= SLACK, "row {row}, divider {divider}, button {button}");
}

#[trial]
fn a_divider_in_a_fill_row_spans_the_rect_it_is_shown_in() {
    let seen = Rc::new(Cell::new((Rect::NOTHING, 0.0)));
    let out = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let at = Rect::from_min_size(ui.cursor().min, AT);
        let shown = Row::new().fill().show_at(ui, at, |ui, _| Divider::vertical().id_salt("divider").show(ui).rect.height());
        out.set((shown.output.rect, shown.inner.unwrap_or_default()));
    });
    fixture.room(ROOM).settle();
    let (rect, divider) = seen.get();
    assert!((divider - rect.height()).abs() <= SLACK && rect.height() >= AT.y - SLACK, "divider {divider}, rect {rect:?}");
}

#[trial]
fn an_end_row_offers_its_children_the_row_and_its_filling_child_ends_at_the_row_edge() {
    let seen = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING)));
    let out = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            let filler = Cell::new(Rect::NOTHING);
            let row = Row::new().fill_width().align(RowAlign::End).show(ui, |ui, _| {
                let _ = Button::new("Apply").show(ui);
                filler.set(Stack::new().fill_width().show(ui, |ui, _| drop(Text::new("Note").show(ui))).output.rect);
            });
            out.set((row.output.rect, filler.get()));
        });
    });
    fixture.room(ROOM);
    fixture.settle();
    let (row, filler) = seen.get();
    assert!(filler.width() > 0.0, "the filling child has width: {filler:?}");
    assert!((filler.max.x - row.max.x).abs() <= SLACK, "the block ends at the row's end edge: child {filler:?}, row {row:?}");
}

#[trial]
fn a_section_of_labelled_rows_in_a_content_column_is_as_wide_as_its_widest_row() {
    static PANES: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::fill(1.0)];
    let seen = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING)));
    let out = Rc::clone(&seen);
    let mut on = false;
    let mut fixture = Fixture::new(move |ui| {
        Grid::new(&PANES).fill().show(ui, |ui, _| {
            let side = Stack::new().scroll(Scroll::vertical()).show(ui, |ui, _| {
                Section::fixed("Options").id_salt("options").show(ui, |ui, cx| {
                    cx.labelled(ui, "a", "Allow downloads".into(), SideLabel::new(), Checkbox::new(Bind::bound(&mut on)).id_salt("check"));
                })
            });
            let _ = Stack::new().fill().show(ui, |ui, _| {
                let _ = Button::new("Main").show(ui);
            });
            out.set((side.inner.map_or(Rect::NOTHING, |section| section.output.response.rect), side.output.rect));
        });
    });
    fixture.room(ROOM).settle();
    let (section, side) = seen.get();
    let check = fixture.by_label("Allow downloads").map(|target| target.rect);
    let dump = format!("section {section:?}, side {side:?}, check {check:?}");
    assert!(section.width() > 0.0 && section.width() < side.width(), "the section is as wide as its row: {dump}");
    assert!(check.is_some_and(|check| check.max.x <= section.max.x + SLACK), "the row fits the section: {dump}");
}

/// The rects of a wrapping row's leading button and of the row nested after it, in a room `width` wide.
fn wrapped(width: f32) -> (Rect, Rect) {
    let seen = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING)));
    let out = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        Row::new().wrap(true).show(ui, |ui, _| {
            let lead = Button::new("Pages from").show(ui).rect;
            let nested = Row::new().show(ui, |ui, _| {
                Button::new("to").show(ui);
                Button::new("Last page").show(ui);
            });
            out.set((lead, nested.output.rect));
        });
    });
    fixture.room(Vec2::new(width, ROOM.y)).settle();
    seen.get()
}

#[trial]
fn a_row_too_wide_for_the_rest_of_a_wrapping_row_moves_to_the_next_line_whole() {
    let (lead, nested) = wrapped(ROOM.x);
    assert!((nested.top() - lead.top()).abs() <= SLACK, "the nested row fits beside the lead: lead {lead:?}, nested {nested:?}");
    let width = nested.right() - lead.left() - 1.0;
    let (lead, nested) = wrapped(width);
    assert!(nested.top() >= lead.bottom() - SLACK, "the nested row wraps below the lead: lead {lead:?}, nested {nested:?}");
    assert!(nested.right() <= width + SLACK, "the wrapped row stays in the room {width}: {nested:?}");
}
