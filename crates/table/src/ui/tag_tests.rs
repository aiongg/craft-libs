//! Error tag and filter button.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use std::collections::BTreeMap;

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Rect, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::error_tag::ErrorTag;
use super::filter::filter_button_rect;
use super::grid::{CellRef, GridLayout, LineIndex, SheetGeometry};
use super::style::SheetStyle;

fn style() -> SheetStyle {
    let mut style = None;
    egui::__run_test_ui(|ui| style = Some(SheetStyle::of(&Tokens::of(ui))));
    style.unwrap()
}

fn geometry(style: &SheetStyle) -> SheetGeometry {
    let none = BTreeMap::new();
    let layout = GridLayout {
        rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 400.0)),
        zoom: 1.0,
        rows: LineIndex::new(&none, 20.0, 1000),
        columns: LineIndex::new(&none, 80.0, 100),
        frozen: (0, 0),
        scroll: egui::Vec2::ZERO,
        headings: true,
        bands: egui::Vec2::ZERO,
        right_to_left: false,
    };
    SheetGeometry::new(layout, style)
}

fn cell(row: u32, column: u32) -> CellRef {
    CellRef { row, column }
}

// ErrorTag::rect is left of the cell, right when it would cover the headings.
#[test]
fn tag_sits_left_or_right_of_the_cell() {
    let style = style();
    let geometry = geometry(&style);
    let (side, gap) = (style.error_tag, style.error_tag_gap);
    let inner = cell(3, 2);
    let tag = ErrorTag::new("tag", inner, &geometry, "Problem", &[]).rect(side, gap).expect("inside the cells");
    let at = geometry.cell_rect(inner);
    assert_eq!(tag, Rect::from_min_size(pos2(at.left() - side - gap, at.top()), vec2(side, side)));
    let first = cell(3, 0);
    let tag = ErrorTag::new("tag", first, &geometry, "Problem", &[]).rect(side, gap).expect("inside the cells");
    assert_eq!(tag.left(), geometry.cell_rect(first).right() + gap, "the first column has no room on its left");
}

struct App {
    fix: Option<usize>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let style = style();
    let geometry = geometry(&style);
    let out = ErrorTag::new("tag", cell(3, 2), &geometry, "Number Stored as Text", &["Convert to Number", "Ignore Error"]).show(ui);
    app.fix = app.fix.or(out.fix);
}

// choosing a fix reports its index.
#[test]
fn fix_menu_reports_its_index() {
    let mut harness = Harness::<App>::builder()
        .with_size([500.0, 400.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App { fix: None });
    harness.run();
    let style = style();
    let tag = ErrorTag::new("tag", cell(3, 2), &geometry(&style), "", &[]).rect(style.error_tag, style.error_tag_gap).unwrap();
    harness.click_at(tag.center());
    harness.run();
    harness.get_by_label("Ignore Error").click();
    harness.run();
    assert_eq!(harness.state().fix, Some(1));
}

// filter_button_rect clamps to 10–18 and the reserved space.
#[test]
fn filter_button_size_is_clamped() {
    let style = style();
    let at = |height: f32, zoom: f32, space: f32| {
        let cell = Rect::from_min_size(pos2(100.0, 50.0), vec2(80.0, height));
        filter_button_rect(cell, zoom, space, &style)
    };
    assert_eq!(at(20.0, 1.0, 30.0).size(), vec2(16.0, 16.0));
    assert_eq!(at(60.0, 1.0, 30.0).size(), vec2(style.filter_button_max, style.filter_button_max));
    assert_eq!(at(8.0, 1.0, 30.0).size(), vec2(style.filter_button_min, style.filter_button_min));
    let narrow = at(20.0, 1.0, 9.0);
    assert_eq!(narrow.width(), 9.0 - style.filter_button_inset, "no wider than the reserved space less the inset");
    let rect = at(20.0, 2.0, 30.0);
    assert_eq!(rect.right(), 180.0 - 2.0 * style.filter_button_inset);
    assert_eq!(rect.bottom(), 70.0 - 2.0 * style.filter_button_inset);
}
