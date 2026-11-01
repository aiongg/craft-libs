//! Slicers.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{PointerButton, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;

use super::slicer::{Slicer, SlicerAction, slicer_layout};
use super::style::SheetStyle;

fn style() -> SheetStyle {
    let mut style = None;
    egui::__run_test_ui(|ui| style = Some(SheetStyle::of(&Tokens::of(ui))));
    style.unwrap()
}

// slicer_layout places tiles in columns with the gaps.
#[test]
fn layout_fills_columns() {
    let style = style();
    let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(300.0, 200.0));
    let layout = slicer_layout(rect, 2.0, 5, 2, &style);
    let (pad, gap, tile) = (style.slicer_padding * 2.0, style.slicer_gap * 2.0, style.slicer_tile * 2.0);
    assert_eq!(layout.header, Rect::from_min_size(rect.min, vec2(300.0, style.slicer_header * 2.0)));
    assert_eq!(layout.tiles.len(), 5);
    let width = (300.0 - 2.0 * pad - gap) / 2.0;
    assert_eq!(layout.tiles[0].size(), vec2(width, tile));
    assert_eq!(layout.tiles[1].left(), layout.tiles[0].right() + gap);
    assert_eq!(layout.tiles[2].top(), layout.tiles[0].bottom() + gap);
    assert_eq!(layout.tiles[2].left(), layout.tiles[0].left());
    assert_eq!(layout.tiles[0].top(), layout.header.bottom() + pad);
    assert_eq!(layout.clear.right(), layout.header.right() - pad);
    assert_eq!(layout.clear.center().y, layout.header.center().y);
    let one = slicer_layout(rect, 1.0, 2, 0, &style);
    assert_eq!(one.tiles[1].top(), one.tiles[0].bottom() + style.slicer_gap, "zero columns count as one");
}

#[derive(Default)]
struct App {
    actions: Vec<SlicerAction>,
}

fn card() -> Rect {
    Rect::from_min_size(pos2(20.0, 20.0), vec2(240.0, 200.0))
}

fn items() -> Vec<(String, bool)> {
    ["Red", "Green", "Blue"].iter().map(|name| (name.to_string(), *name == "Green")).collect()
}

fn show(ui: &mut Ui, app: &mut App) {
    let items = items();
    let out = Slicer::new("slicer", "Colour", &items).columns(1).filtered(true).show(ui, card());
    app.actions.extend(out.action);
}

fn harness() -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([400.0, 300.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App::default());
    harness.run();
    harness
}

// a tile click reports Toggle, the funnel Clear.
#[test]
fn clicks_toggle_and_clear() {
    let style = style();
    let layout = slicer_layout(card(), 1.0, 3, 1, &style);
    let mut harness = harness();
    harness.click_at_with(layout.tiles[2].center(), PointerButton::Primary, egui::Modifiers::NONE);
    harness.click_at_with(layout.clear.center(), PointerButton::Primary, egui::Modifiers::NONE);
    harness.click_at_with(layout.header.left_center() + vec2(2.0, 0.0), PointerButton::Primary, egui::Modifiers::NONE);
    assert_eq!(harness.state().actions, [SlicerAction::Toggle("Blue".into()), SlicerAction::Clear]);
}
