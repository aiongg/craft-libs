//! The outline band.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use std::collections::BTreeMap;

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Modifiers, PointerButton, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;

use super::grid::{GridLayout, LineIndex, SheetGeometry};
use super::outline::{Frame, OutlineAction, OutlineAxis, OutlineBand, OutlineGroup, lane};
use super::style::SheetStyle;

fn style() -> SheetStyle {
    let mut style = None;
    egui::__run_test_ui(|ui| style = Some(SheetStyle::of(&Tokens::of(ui))));
    style.unwrap()
}

// thickness is (levels + 1) lanes + 3, 0 without an outline.
#[test]
fn band_thickness_counts_a_lane_per_level_plus_one() {
    let style = style();
    let step = lane(style.outline_lane, 1.0);
    assert_eq!(OutlineBand::thickness(0, 1.0, &style), 0.0);
    let slack = OutlineBand::thickness(2, 1.0, &style) - 3.0 * step;
    assert!((slack - 0.2 * step).abs() < 1e-3, "{slack}");
    assert!(OutlineBand::thickness(3, 1.0, &style) > OutlineBand::thickness(2, 1.0, &style));
    assert_eq!(lane(style.outline_lane, 0.1), lane(style.outline_lane, 0.75), "lanes stop shrinking at the least zoom");
    assert_eq!(lane(style.outline_lane, 2.0), (style.outline_lane * 2.0).round());
}

/// Rows 4 to 6 (one-based) in a group, its summary row below.
fn rows() -> OutlineAxis {
    let group = OutlineGroup { start: 3, end: 5, level: 1, collapsed: false, summary_before: false };
    OutlineAxis { levels: 1, groups: vec![group] }
}

fn geometry(style: &SheetStyle, bands: egui::Vec2) -> SheetGeometry {
    let none = BTreeMap::new();
    let layout = GridLayout {
        rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 400.0)),
        zoom: 1.0,
        rows: LineIndex::new(&none, 20.0, 1000),
        columns: LineIndex::new(&none, 80.0, 100),
        frozen: (0, 0),
        scroll: egui::Vec2::ZERO,
        headings: true,
        bands,
        right_to_left: false,
    };
    SheetGeometry::new(layout, style)
}

#[derive(Default)]
struct App {
    actions: Vec<OutlineAction>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let style = style();
    let band = vec2(OutlineBand::thickness(1, 1.0, &style), 0.0);
    let geometry = geometry(&style, band);
    let none = OutlineAxis::default();
    app.actions.extend(OutlineBand::new("outline", &rows(), &none, &geometry, 1.0).show(ui).action);
}

// a click on a level button and a − button report their actions.
#[test]
fn buttons_report_levels_and_toggles() {
    let style = style();
    let band = vec2(OutlineBand::thickness(1, 1.0, &style), 0.0);
    let geometry = geometry(&style, band);
    let none = OutlineAxis::default();
    let rows = rows();
    let layout = Frame::new(&geometry, &rows, &none, 1.0, &style).layout();
    let target = |wanted: fn(&OutlineAction) -> bool| layout.buttons.iter().find(|b| wanted(&b.action)).expect("button").rect.center();
    let mut harness = Harness::<App>::builder()
        .with_size([500.0, 400.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App::default());
    harness.run();
    harness.click_at_with(target(|a| matches!(a, OutlineAction::Level { level: 2, .. })), PointerButton::Primary, Modifiers::NONE);
    harness.click_at_with(target(|a| matches!(a, OutlineAction::Toggle { .. })), PointerButton::Primary, Modifiers::NONE);
    let actions = &harness.state().actions;
    assert_eq!(actions[0], OutlineAction::Level { rows: true, level: 2 });
    assert!(matches!(actions[1], OutlineAction::Toggle { rows: true, group } if group == rows.groups[0]));
    assert_eq!(actions.len(), 2);
    // The group's bracket runs from its first row to the button on the summary row.
    assert_eq!(layout.buttons.iter().filter(|b| matches!(b.action, OutlineAction::Toggle { .. })).count(), 1);
}
