//! The column bar.

use craft_ui::direction::{self, Direction};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, current, install};
use egui::accesskit::{Action, ActionData, ActionRequest, Role};
use egui::{Event, Key, Shape, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::column_bar::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// Moving divider 1 of [1, 1, 1, 1] by 0.1 changes columns 1 and 2 only and keeps the sum.
#[test]
fn a_divider_moves_only_its_two_columns() {
    let mut widths = [1.0, 1.0, 1.0, 1.0];
    move_divider(&mut widths, 1, 0.1);
    assert_eq!((widths[0], widths[3]), (1.0, 1.0));
    assert!(close(widths[1], 1.4) && close(widths[2], 0.6), "{widths:?}");
    assert!(close(widths.iter().sum::<f64>(), 4.0));
    assert!(close(shares(&[2.0, 6.0])[0], 0.25));
    assert_eq!(shares(&[0.0, 0.0]), [0.5, 0.5]);
}

// Dragging a divider between two 0.01 columns far either way keeps both at a quarter of their pair.
#[test]
fn narrow_pairs_never_panic() {
    for delta in [1.0, -1.0, f64::MAX, f64::NEG_INFINITY] {
        let mut widths = [0.01, 0.01, 0.98];
        move_divider(&mut widths, 0, delta);
        if delta.is_finite() {
            assert!(close(widths[0].min(widths[1]), 0.005), "{widths:?}");
            assert!(close(widths[0] + widths[1], 0.02));
        } else {
            assert_eq!(widths, [0.01, 0.01, 0.98]);
        }
    }
    let mut none: [f64; 0] = [];
    move_divider(&mut none, 0, 0.1);
}

#[derive(Default)]
struct App {
    events: Vec<EditEvent<Vec<f64>>>,
}

fn open() -> Harness<'static, App> {
    open_in(Direction::Ltr)
}

fn open_in(dir: Direction) -> Harness<'static, App> {
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(400.0, 100.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.events.extend(ColumnBar::new("bar", &[1.0, 1.0, 1.0, 1.0]).show(ui).edit);
            },
            App::default(),
        );
    harness.run();
    harness
}

// Tab to divider 0 and Right arrow reports Set with column 0 grown by 0.01 of the whole.
#[test]
fn arrow_keys_resize_the_focused_pair() {
    let mut harness = open();
    harness.key_press(Key::Tab);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    let events = &harness.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert!(close(value[0], 1.04) && close(value[1], 0.96), "{value:?}");
    assert_eq!((value[2], value[3]), (1.0, 1.0));
}

// Right to left the bar mirrors: divider 0 sits right of the middle, and the Right arrow, which points away from the first column, shrinks it.
#[test]
fn the_bar_mirrors_right_to_left() {
    let mut harness = open_in(Direction::Rtl);
    let left_to_right = open();
    let at = |harness: &Harness<'_, App>| harness.get_by_role_and_label(Role::Splitter, "Column 1 width").rect().center().x;
    assert!(at(&harness) > 200.0, "{}", at(&harness));
    assert!((at(&harness) + at(&left_to_right) - 400.0).abs() < 1.0);
    harness.key_press(Key::Tab);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    let events = &harness.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert!(close(value[0], 0.96) && close(value[1], 1.04), "{value:?}");
}

// The columns alternate the style's two fills, first column first.
#[test]
fn columns_alternate_their_fill() {
    fn rects(shape: &Shape, fills: &mut Vec<egui::Color32>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| rects(shape, fills)),
            Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT => fills.push(rect.fill),
            _ => {}
        }
    }
    let harness = open();
    let style = ColumnBar::style(&current(&harness.ctx));
    let mut fills = Vec::new();
    harness.output().shapes.iter().for_each(|clipped| rects(&clipped.shape, &mut fills));
    fills.retain(|fill| *fill == style.fill || *fill == style.fill_alt);
    assert_eq!(fills, [style.fill, style.fill_alt, style.fill, style.fill_alt]);
}

// A screen reader's SetValue gives column 0 the percentage (40 % of 4 equal columns): the next column gives up the difference.
#[test]
fn set_value_sets_the_columns_percentage() {
    let mut harness = open();
    let (target_node, target_tree) = harness.get_by_role_and_label(Role::Splitter, "Column 1 width").accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target_tree,
        target_node,
        data: Some(ActionData::NumericValue(40.0)),
    }));
    harness.run();
    let events = &harness.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert!(close(value[0], 1.6) && close(value[1], 0.4), "{value:?}");
}

fn painted_texts(harness: &Harness<'_, App>) -> Vec<String> {
    fn texts(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| texts(shape, out)),
            Shape::Text(text) => out.push(text.galley.text().to_owned()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    harness.output().shapes.iter().for_each(|clipped| texts(&clipped.shape, &mut out));
    out
}

// Each column of a 400 pt bar paints its percentage, as PdfCraft does; in a 100 pt bar the columns are 25 pt, under the style's label minimum, and paint none. The column's name is the divider's description.
#[test]
fn wide_columns_paint_their_percentage() {
    let harness = open();
    assert_eq!(painted_texts(&harness), ["25%", "25%", "25%", "25%"]);
    let divider = harness.get_by_role_and_label(Role::Splitter, "Column 1 width");
    assert_eq!(divider.accesskit_node().description().as_deref(), Some("Column A"));
    let narrow =
        HarnessBuilder::default().with_size(vec2(100.0, 100.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.events.extend(ColumnBar::new("bar", &[1.0, 1.0, 1.0, 1.0]).show(ui).edit);
            },
            App::default(),
        );
    let mut narrow = narrow;
    narrow.run();
    assert!(painted_texts(&narrow).is_empty());
}
