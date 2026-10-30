//! List row tests: state fills, anatomy, truncation and optional items, the toggle sweep, the
//! rename field, AccessKit and mirroring.

use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::row::{LayersArgs, ThumbArg, show_layers};
use super::*;
use crate::direction::{self, Direction};
use crate::edit::{EditEvent, GestureHost, UndoMarks};
use crate::icons::Icon;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, corner_radius, install, scope};
use craft_theme::Size;

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

/// A column `width` wide at the top left.
fn panel(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 0.0;
        add(ui);
    });
}

/// Every shape a frame paints, flattened.
fn shapes(ctx: &Context, add: impl FnMut(&mut Ui)) -> Vec<Shape> {
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    let mut add = add;
    let output = ctx.run_ui(egui::RawInput::default(), |ui| add(ui));
    let mut out = Vec::new();
    output.shapes.iter().for_each(|s| flatten(&s.shape, &mut out));
    output.drop_without_applying_deltas();
    out
}

fn context() -> Context {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx
}

#[allow(clippy::expect_used)]
fn style() -> ListRowStyle {
    let ctx = context();
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(ListRow::<u8>::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

fn fills(shapes: &[Shape]) -> Vec<&egui::epaint::RectShape> {
    shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { (r.fill.a() > 0).then_some(r) } else { None }).collect()
}

fn label_colors(shapes: &[Shape]) -> Vec<egui::Color32> {
    shapes.iter().filter_map(|s| if let Shape::Text(t) = s { t.galley.job.sections.first().map(|s| s.format.color) } else { None }).collect()
}

#[test]
fn state_fills_follow_selected_and_highlight() {
    let (ctx, s) = (context(), style());
    let row = |selected, highlight| {
        shapes(&ctx, |ui| {
            panel(ui, 240.0, |ui| {
                let _ = ListRow::new(1u8, "Layer").selected(selected).highlight(highlight).show(ui);
            })
        })
    };
    assert!(fills(&row(false, false)).is_empty(), "an idle row has no fill");
    assert_eq!(fills(&row(false, true))[0].fill, s.hover);
    assert_eq!(fills(&row(true, false))[0].fill, s.selected);
    assert_eq!(fills(&row(true, true))[0].fill, s.selected_hover);
}

#[test]
fn the_selected_row_is_a_fill_only_rounded_in_comfortable() {
    let ctx = context();
    for (size, rounded) in [(Size::Compact, false), (Size::Comfortable, true)] {
        let mut radius = 0.0;
        let mut row_height = 0.0;
        let drawn = shapes(&ctx, |ui| {
            scope(
                ui,
                |o| o.size = Some(size),
                |ui| {
                    let tokens = Tokens::of(ui);
                    radius = tokens.shape.radius.control;
                    row_height = tokens.size.control.md;
                    panel(ui, 240.0, |ui| {
                        let _ = ListRow::new(1u8, "Layer").selected(true).show(ui);
                    })
                },
            )
        });
        let fill = fills(&drawn).into_iter().next().expect("selected fill");
        assert_eq!(fill.corner_radius, corner_radius(if rounded { radius } else { 0.0 }), "{size:?}");
        assert_eq!(fill.rect.height(), row_height, "{size:?}");
        let outlined = drawn.iter().any(|s| matches!(s, Shape::Rect(r) if r.stroke.width > 0.0 && r.rect.size() == fill.rect.size()));
        assert!(!outlined, "{size:?}: the selection has no border");
    }
}

#[test]
fn dividers_belong_to_the_layer_form_in_compact_and_a_dimmed_label_and_detail_take_the_selected_fill_into_account() {
    let ctx = context();
    let s = style();
    let lines = |size, layer| {
        shapes(&ctx, |ui| {
            scope(
                ui,
                |o| o.size = Some(size),
                |ui| {
                    panel(ui, 240.0, |ui| {
                        let _ = ListRow::new(1u8, "Layer").toggle(ToggleColumn::eye("Visibility {row}", true)).layer(layer).show(ui);
                    })
                },
            )
        })
        .iter()
        .filter(|s| matches!(s, Shape::LineSegment { .. }))
        .count()
    };
    assert_eq!(lines(Size::Compact, true), 2, "one under the row and one at the column's trailing edge");
    assert_eq!(lines(Size::Comfortable, true), 0);
    assert_eq!(lines(Size::Compact, false), 0, "a plain row has none");
    let label = |dimmed, selected| {
        let drawn = shapes(&ctx, |ui| {
            panel(ui, 240.0, |ui| {
                let _ = ListRow::new(1u8, "Layer").dimmed(dimmed).selected(selected).detail("Smart filter").show(ui);
            })
        });
        label_colors(&drawn)
    };
    assert_eq!(label(false, false), [s.text, s.detail]);
    assert_eq!(label(true, false), [s.dimmed, s.detail]);
    assert_eq!(label(false, true), [s.selected_text, s.selected_detail]);
    // A hidden layer that is selected: its name and detail read on the selected fill.
    assert_eq!(label(true, true), [s.selected_dimmed, s.selected_detail]);
    for text in [s.selected_dimmed, s.selected_detail] {
        let ratio = craft_theme::contrast::contrast_ratio(to_rgba(text), to_rgba(s.selected));
        assert!(ratio >= 4.5, "{text:?} on the selected fill is {ratio:.2}");
    }
}

fn to_rgba(c: egui::Color32) -> craft_theme::color::Rgba {
    craft_theme::color::Rgba([c.r(), c.g(), c.b(), c.a()])
}

#[test]
fn drop_marks_draw_a_line_on_an_edge_or_an_outline_inside() {
    let (ctx, s) = (context(), style());
    let mark = |mark| {
        shapes(&ctx, |ui| {
            panel(ui, 240.0, |ui| {
                let _ = ListRow::new(1u8, "Layer").drop_mark(mark).show(ui);
            })
        })
    };
    let halo = s.drop_halo.width;
    let before = mark(Some(DropMark::Before));
    let line = fills(&before).into_iter().find(|r| r.fill == s.drop).expect("line");
    assert_eq!((line.rect.height(), line.rect.width(), line.rect.top()), (s.drop_width, 240.0, halo));
    let after = mark(Some(DropMark::After));
    let line = fills(&after).into_iter().find(|r| r.fill == s.drop).expect("line");
    assert_eq!((line.rect.height(), line.rect.bottom()), (s.drop_width, s.row_height - halo));
    let into = mark(Some(DropMark::Into));
    assert!(into.iter().any(|s2| matches!(s2, Shape::Rect(r) if r.stroke.color == s.drop && r.stroke.width == s.drop_width)));
    assert!(fills(&mark(None)).iter().all(|r| r.fill != s.drop));
}

/// The standalone line is the list's line: same colour and thickness on the same halo, whether
/// it runs across or down.
#[test]
fn drop_line_matches_the_list_marker() {
    let (ctx, s) = (context(), style());
    let at = |from: Pos2, to: Pos2| shapes(&ctx, |ui| drop_line(ui, from, to));
    let drawn = at(Pos2::new(10.0, 50.0), Pos2::new(90.0, 50.0));
    let across = fills(&drawn);
    let line = across.iter().find(|r| r.fill == s.drop).expect("line");
    assert_eq!((line.rect.height(), line.rect.width(), line.rect.center().y), (s.drop_width, 80.0, 50.0));
    let halo = across.iter().find(|r| r.fill == s.drop_halo.color).expect("halo");
    assert_eq!(halo.rect.height(), s.drop_width + 2.0 * s.drop_halo.width);
    let drawn = at(Pos2::new(30.0, 10.0), Pos2::new(30.0, 60.0));
    let down = fills(&drawn);
    let line = down.iter().find(|r| r.fill == s.drop).expect("line");
    assert_eq!((line.rect.width(), line.rect.height(), line.rect.center().x), (s.drop_width, 50.0, 30.0));
}

/// The halo is painted first and covers the marker with `drop_halo.width` to spare on each side
/// across the line, so the marker never touches a row fill; it stays inside the row.
#[test]
fn drop_marks_sit_on_their_halo() {
    let (ctx, s) = (context(), style());
    let w = s.drop_halo.width;
    assert!(w > 0.0 && s.drop_halo.color != s.drop);
    for mark in [DropMark::Before, DropMark::After, DropMark::Into] {
        let drawn = shapes(&ctx, |ui| {
            panel(ui, 240.0, |ui| {
                let _ = ListRow::new(1u8, "Layer").selected(true).drop_mark(Some(mark)).show(ui);
            })
        });
        let rects: Vec<_> = drawn.iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r) } else { None }).collect();
        let row = Rect::from_min_size(Pos2::ZERO, vec2(240.0, s.row_height));
        let is_halo = |r: &&&egui::epaint::RectShape| r.fill == s.drop_halo.color || r.stroke.color == s.drop_halo.color;
        let is_marker = |r: &&&egui::epaint::RectShape| r.fill == s.drop || r.stroke.color == s.drop;
        let halo_at = rects.iter().position(|r| is_halo(&r)).unwrap_or_else(|| panic!("{mark:?}: no halo"));
        let marker_at = rects.iter().position(|r| is_marker(&r)).unwrap_or_else(|| panic!("{mark:?}: no marker"));
        assert!(halo_at < marker_at, "{mark:?}: the halo is under the marker");
        let (halo, marker) = (rects[halo_at], rects[marker_at]);
        assert!(row.contains_rect(halo.rect), "{mark:?}: the halo stays inside the row");
        if mark == DropMark::Into {
            assert_eq!((halo.rect, halo.stroke.width), (row, s.drop_width + 2.0 * w));
            assert_eq!((marker.rect, marker.stroke.width), (row.shrink(w), s.drop_width));
        } else {
            assert_eq!(halo.rect.shrink2(vec2(0.0, w)), marker.rect, "{mark:?}");
        }
    }
}

#[test]
fn a_thumbnail_sets_the_row_height_and_is_framed() {
    let (ctx, s) = (context(), style());
    for (size, side) in [(ThumbSize::Small, s.thumb.sm), (ThumbSize::Medium, s.thumb.md), (ThumbSize::Large, s.thumb.lg)] {
        let mut painted = Rect::NOTHING;
        let mut height = 0.0;
        let drawn = shapes(&ctx, |ui| {
            panel(ui, 240.0, |ui| {
                let row = ListRow::new(1u8, "Layer").thumb(size, |_, rect| painted = rect).show(ui);
                height = row.response.rect.height();
            })
        });
        assert_eq!(height, (side + s.thumb_pad).max(s.thumb_row_min), "{size:?}");
        assert_eq!(height, s.height(size, false));
        assert_eq!(painted.size(), vec2(side, side), "{size:?}: the host paints the whole thumbnail");
        assert!(
            drawn.iter().any(|s2| matches!(s2, Shape::Rect(r)
                if r.stroke.color == s.thumb_border && r.stroke_kind == egui::StrokeKind::Outside && r.rect == painted)),
            "{size:?}: the frame sits just outside it"
        );
    }
    assert_eq!(s.height(ThumbSize::None, false), s.row_height);
    assert!(s.height(ThumbSize::None, true) > s.row_height, "a detail line makes the row taller");
}

#[test]
fn optional_items_drop_out_before_the_label_gets_too_narrow() {
    let s = style();
    let mut seen = Vec::new();
    for width in [320.0, 200.0, 140.0, 80.0] {
        let label = std::cell::Cell::new(Rect::NOTHING);
        let harness = harness(Direction::Ltr).build_ui(|ui| {
            panel(ui, width, |ui| {
                label.set(
                    ListRow::new(1u8, "A layer with a long name")
                        .toggle(ToggleColumn::eye("Visibility {row}", true))
                        .trailing_item(16.0, |ui| {
                            ui.label("fx");
                        })
                        .optional_item(56.0, |ui| {
                            ui.label("Multiply");
                        })
                        .show(ui)
                        .label,
                );
            })
        });
        let label = label.get();
        let (fx, multiply) = (harness.query_by_label("fx").is_some(), harness.query_by_label("Multiply").is_some());
        assert!(fx, "{width}: the required item stays");
        if multiply {
            assert!(label.width() >= s.min_label - 0.5, "{width}: the label keeps its least width with the optional item: {label:?}");
        }
        seen.push(multiply);
    }
    assert_eq!(seen, [true, true, false, false]);
}

#[test]
fn a_long_label_is_cut_before_the_trailing_items_and_keeps_its_name() {
    let s = style();
    let band = std::cell::Cell::new(Rect::NOTHING);
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 150.0, |ui| {
            let out = ListRow::new(1u8, "A layer with a long name that cannot fit")
                .trailing_item(16.0, |ui| {
                    ui.label("fx");
                })
                .show(ui);
            band.set(out.label);
        })
    });
    let row = harness.get_by_role(Role::ListBoxOption);
    assert_eq!(row.accesskit_node().label().as_deref(), Some("A layer with a long name that cannot fit"));
    let text = band.get();
    let fx = harness.get_by_label("fx").rect();
    assert!(text.right() <= fx.left() + 0.5, "{text:?} {fx:?}");
    assert!(fx.right() <= row.rect().right() - s.trailing_pad + 0.5, "the scroll bar's room is kept: {fx:?}");
}

#[test]
fn the_indent_shrinks_before_the_label_gets_too_narrow() {
    let s = style();
    let label_width = |indent| {
        let label = std::cell::Cell::new(Rect::NOTHING);
        let _harness = harness(Direction::Ltr).build_ui(|ui| {
            panel(ui, 120.0, |ui| {
                label.set(ListRow::new(1u8, "Layer").indent(indent).icon(Icon::Layers).show(ui).label);
            })
        });
        label.get().width()
    };
    assert_eq!(label_width(0.0), label_width(8.0) + 8.0, "room for the indent: it is taken from the label");
    let deep = label_width(500.0);
    assert!((deep - s.min_label).abs() < 0.5, "{deep}");
}

#[test]
fn a_row_is_a_labelled_list_box_option_and_no_tab_stop() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 260.0, |ui| {
            let _ = ListRow::new(1u8, "Layer 1")
                .detail("Raster layer")
                .toggle(ToggleColumn::eye("Visibility {row}", true))
                .toggle(ToggleColumn::lock("Lock {row}", false))
                .selected(true)
                .position_in_set(4)
                .show(ui);
        })
    });
    a11y::assert_labelled(&harness);
    let row = harness.get_by_role(Role::ListBoxOption);
    let node = row.accesskit_node();
    assert_eq!(node.label().as_deref(), Some("Layer 1, Raster layer"));
    assert_eq!(harness.query_all_by_label("Layer 1, Raster layer").count(), 1, "the label text is not a second node");
    assert_eq!(node.is_selected(), Some(true));
    assert_eq!(node.position_in_set(), Some(4));
    let eye = harness.get_by_label("Visibility Layer 1");
    let lock = harness.get_by_label("Lock Layer 1");
    assert_eq!((eye.accesskit_node().role(), lock.accesskit_node().role()), (Role::Button, Role::Button));
    assert_eq!(
        (eye.accesskit_node().toggled(), lock.accesskit_node().toggled()),
        (Some(egui::accesskit::Toggled::True), Some(egui::accesskit::Toggled::False))
    );
    assert!(a11y::tab_order(&mut harness).is_empty(), "the list is the one focus stop, not its rows");
}

#[test]
fn the_row_mirrors_in_a_right_to_left_interface() {
    let s = style();
    let rects = |dir| {
        let harness = harness(dir).build_ui(|ui| {
            panel(ui, 300.0, |ui| {
                let _ = ListRow::new(1u8, "Layer")
                    .toggle(ToggleColumn::eye("Visibility {row}", true))
                    .trailing_item(16.0, |ui| {
                        ui.label("fx");
                    })
                    .show(ui);
            })
        });
        (harness.get_by_role(Role::ListBoxOption).rect(), harness.get_by_label("Visibility Layer").rect(), harness.get_by_label("fx").rect())
    };
    let (row, eye, fx) = rects(Direction::Ltr);
    assert_eq!((eye.left() - row.left(), eye.width()), (0.0, s.toggle_column));
    assert!((row.right() - s.trailing_pad - 16.0 - fx.left()).abs() < 0.5, "the item is 16 wide, its content at its start: {fx:?}");
    let (row, eye, fx) = rects(Direction::Rtl);
    assert_eq!((row.right() - eye.right(), eye.width()), (0.0, s.toggle_column));
    assert!((fx.right() - row.left() - s.trailing_pad - 16.0).abs() < 0.5, "{fx:?}");
}

/// Eye columns of four rows, the first three on, and what the rows reported.
struct Sweeps {
    on: [bool; 4],
    events: Vec<EditEvent<ToggleEdit<usize>>>,
}

fn sweeping(ui: &mut Ui, state: &mut Sweeps) {
    panel(ui, 240.0, |ui| {
        for row in 0..4 {
            let out = ListRow::new(row, format!("Row {row}")).toggle(ToggleColumn::eye("Visibility {row}", state.on[row])).show(ui);
            for event in out.events {
                if let RowEvent::Toggle(event) = event {
                    if let EditEvent::Update { value, .. } | EditEvent::Set { value, .. } = &event {
                        state.on[value.row] = value.on;
                    }
                    state.events.push(event);
                }
            }
        }
    });
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// Presses on the eye cell of `from`, moves down to the cell of `to` in steps, and releases.
fn sweep<S>(harness: &mut Harness<'_, S>, from: usize, to: usize, modifiers: Modifiers) {
    let cell = |row: usize| harness.get_by_label(&format!("Visibility Row {row}")).rect().center();
    let (start, end) = (cell(from), cell(to));
    harness.event(Event::PointerMoved(start));
    harness.run();
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(button(start, true, modifiers));
    harness.run();
    for t in [0.25, 0.5, 1.0] {
        harness.event(Event::PointerMoved(start + (end - start) * t));
        harness.run();
    }
    harness.event(button(end, false, modifiers));
    harness.run();
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

#[test]
fn dragging_down_a_toggle_column_gives_every_row_the_first_clicks_state_as_one_gesture() {
    let mut harness = harness(Direction::Ltr).build_ui_state(sweeping, Sweeps { on: [true, true, false, true], events: Vec::new() });
    sweep(&mut harness, 0, 3, Modifiers::NONE);
    let state = harness.state();
    assert_eq!(state.on, [false; 4], "the pressed row's new state spreads down the column");
    let kinds: Vec<_> = state
        .events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin".to_owned(),
            EditEvent::Update { value, .. } => format!("update {}", value.row),
            EditEvent::Commit { value, .. } => format!("commit {}", value.row),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["begin", "update 0", "update 1", "update 3", "commit 0"], "row 2 was already off: no update");
    let ids: Vec<_> = state.events.iter().filter_map(|e| e.id()).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]), "one gesture: {ids:?}");
}

/// Eye states as a document with an undo history of whole states.
#[derive(Default)]
struct Eyes {
    on: [bool; 4],
    undo: Vec<[bool; 4]>,
}

impl UndoMarks for Eyes {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(on) = self.undo.get(mark) {
            self.on = *on;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

/// Eye columns whose sweeps a `GestureHost` turns into undo steps.
struct Hosted {
    doc: Eyes,
    host: GestureHost<Eyes>,
}

fn hosted(ui: &mut Ui, state: &mut Hosted) {
    panel(ui, 240.0, |ui| {
        for row in 0..4 {
            let out = ListRow::new(row, format!("Row {row}")).toggle(ToggleColumn::eye("Visibility {row}", state.doc.on[row])).show(ui);
            for event in out.events {
                if let RowEvent::Toggle(event) = event {
                    state.host.handle(ui.ctx(), &mut state.doc, event, |doc: &mut Eyes, edit: ToggleEdit<usize>| {
                        doc.undo.push(doc.on);
                        doc.on[edit.row] = edit.on;
                    });
                }
            }
        }
    });
    let _ = state.host.end_frame(ui.ctx(), &mut state.doc);
}

#[test]
fn a_gesture_host_keeps_a_sweep_as_one_undo_step_and_cancel_all_reverts_one() {
    let state = Hosted { doc: Eyes { on: [true; 4], undo: Vec::new() }, host: GestureHost::new() };
    let mut harness = harness(Direction::Ltr).build_ui_state(hosted, state);
    sweep(&mut harness, 0, 3, Modifiers::NONE);
    let state = harness.state();
    assert_eq!((state.doc.on, state.doc.undo.len(), state.host.is_active()), ([false; 4], 1, false), "the sweep lands as one step");

    let cell = |harness: &Harness<'_, Hosted>, row: usize| harness.get_by_label(&format!("Visibility Row {row}")).rect().center();
    let (start, end) = (cell(&harness, 0), cell(&harness, 2));
    harness.event(Event::PointerMoved(start));
    harness.event(button(start, true, Modifiers::NONE));
    harness.run();
    harness.event(Event::PointerMoved(end));
    harness.run();
    assert_eq!(harness.state().doc.on, [true, true, true, false], "rows 0 to 2 swept on");
    crate::edit::cancel_all(&harness.ctx, crate::edit::CancelReason::External);
    harness.run();
    harness.event(button(end, false, Modifiers::NONE));
    harness.run();
    let state = harness.state();
    assert!(!state.host.is_active(), "cancel_all ends the sweep");
    assert_eq!(state.doc.on, [true, true, true, false], "an external cancel keeps the document as it is");
    harness.state_mut().doc.on = [false; 4];
    harness.event(Event::PointerMoved(start));
    harness.event(button(start, true, Modifiers::NONE));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    harness.event(button(start, false, Modifiers::NONE));
    harness.run();
    assert_eq!((harness.state().doc.on, harness.state().host.is_active()), ([false; 4], false), "Esc cancels and reverts");
}

#[test]
fn a_toggle_click_reports_modifiers_and_a_screen_readers_click_is_one_set() {
    let mut harness = harness(Direction::Ltr).build_ui_state(sweeping, Sweeps { on: [true; 4], events: Vec::new() });
    sweep(&mut harness, 1, 1, Modifiers::ALT);
    let state = harness.state();
    assert_eq!(state.on, [true, false, true, true]);
    let alt = state.events.iter().find_map(|e| if let EditEvent::Update { modifiers, .. } = e { Some(modifiers.alt) } else { None });
    assert_eq!(alt, Some(true), "Alt-click is reported");
    assert!(matches!(state.events.last(), Some(EditEvent::Commit { .. })));

    harness.state_mut().events.clear();
    harness.get_by_label("Visibility Row 2").click_accesskit();
    harness.run();
    let events = &harness.state().events;
    assert!(matches!(events.as_slice(), [EditEvent::Set { value, .. }] if value.row == 2 && !value.on), "{events:?}");
}

#[test]
fn escape_cancels_a_sweep() {
    let mut harness = harness(Direction::Ltr).build_ui_state(sweeping, Sweeps { on: [true; 4], events: Vec::new() });
    let start = harness.get_by_label("Visibility Row 0").rect().center();
    harness.event(Event::PointerMoved(start));
    harness.run();
    harness.event(button(start, true, Modifiers::NONE));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    harness.event(button(start, false, Modifiers::NONE));
    harness.run();
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Cancel { .. })), "{events:?}");
    assert!(!events.iter().any(|e| matches!(e, EditEvent::Commit { .. })));
}

/// A renaming row and what it reported.
struct Rename {
    label: String,
    renaming: bool,
    events: Vec<RowEvent<u8>>,
}

fn renaming(ui: &mut Ui, state: &mut Rename) {
    panel(ui, 240.0, |ui| {
        // Neighbours for Tab and Shift+Tab to move the focus to.
        let _ = ui.button("Before");
        let out = ListRow::new(1u8, state.label.clone()).renaming(state.renaming).show(ui);
        let _ = ui.button("After");
        for event in out.events {
            match &event {
                RowEvent::Renamed { text, .. } if !text.is_empty() => state.label.clone_from(text),
                RowEvent::RenameEnded { .. } => state.renaming = false,
                _ => {}
            }
            state.events.push(event);
        }
    });
}

fn rename_harness() -> Harness<'static, Rename> {
    let mut harness = harness(Direction::Ltr).build_ui_state(renaming, Rename { label: "Layer 1".into(), renaming: true, events: Vec::new() });
    harness.run();
    harness
}

fn typed(harness: &mut Harness<'_, Rename>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

#[test]
fn the_rename_field_has_the_focus_and_the_whole_label_selected() {
    let mut harness = rename_harness();
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the field replaces the label");
    assert!(harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Layer 1")).is_none());
    typed(&mut harness, "Sky");
    harness.key_press(Key::Enter);
    harness.run();
    let state = harness.state();
    assert_eq!(state.label, "Sky", "typing replaced the selected text");
    assert_eq!(state.events, [RowEvent::Renamed { row: 1, text: "Sky".into() }, RowEvent::RenameEnded { row: 1, end: RenameEnd::Enter }]);
    assert!(!state.renaming);
}

#[test]
fn escape_cancels_the_rename_and_unchanged_text_renames_nothing() {
    let mut harness = rename_harness();
    typed(&mut harness, "Sky");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().events, [RowEvent::RenameEnded { row: 1, end: RenameEnd::Escape }]);
    assert_eq!(harness.state().label, "Layer 1");

    let mut harness = rename_harness();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().events, [RowEvent::RenameEnded { row: 1, end: RenameEnd::Enter }], "Enter on the unchanged label");
}

#[test]
fn empty_text_is_reported_for_the_host_to_decide() {
    let mut harness = rename_harness();
    harness.key_press(Key::Delete);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let state = harness.state();
    assert_eq!(state.events[0], RowEvent::Renamed { row: 1, text: String::new() });
    assert_eq!(state.label, "Layer 1", "this host ignores empty text");
}

#[test]
fn tab_and_shift_tab_commit_and_say_which_way_to_go() {
    for (modifiers, end) in [(Modifiers::NONE, RenameEnd::Tab), (Modifiers::SHIFT, RenameEnd::ShiftTab)] {
        let mut harness = rename_harness();
        typed(&mut harness, "Sky");
        harness.key_press_modifiers(modifiers, Key::Tab);
        harness.run();
        let state = harness.state();
        assert_eq!(state.events, [RowEvent::Renamed { row: 1, text: "Sky".into() }, RowEvent::RenameEnded { row: 1, end }], "{end:?}");
    }
}

#[test]
fn the_list_story_names_every_control_in_both_directions() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for width in [60.0, 160.0, 320.0] {
            for thumbs in ["None", "Large"] {
                let thumbnails = if thumbs == "None" { ThumbArg::None } else { ThumbArg::Large };
                let mut a = LayersArgs { thumbnails, detail: true, highlight_second_row: true, layer_form: true, width };
                let harness = harness(dir).build_ui(move |ui| show_layers(ui, &mut a));
                a11y::assert_labelled(&harness);
            }
        }
    }
}

/// The layer form (PhotoCraft `layer_row.rs:283-292,547`): the hover carries the accent tint; in a
/// theme whose PhotoCraft kind is not a Pro kind the selected row is the hover fill in a 1 pt inside
/// outline with normal text and the thumbnail frame is rounded; in a Pro theme it is the plain
/// selected fill. The plain form keeps the plain hover and selected fills.
#[test]
fn the_layer_form_tints_the_hover_and_outlines_the_selected_row_where_photocraft_does() {
    let ctx = context();
    for (id, outlined) in [("craft.artcraft", false), ("craft.studio", true), ("craft.classic", true)] {
        let theme = craft_theme::builtins().iter().find(|t| t.id.as_str() == id).expect("built-in theme").clone();
        let drawn = |layer: bool, selected: bool, hovered: bool| {
            let mut style = None;
            let drawn = shapes(&ctx, |ui| {
                scope(
                    ui,
                    |o| o.theme = Some(theme.clone()),
                    |ui| {
                        style = Some(ListRow::<u8>::style(&Tokens::of(ui)));
                        panel(ui, 240.0, |ui| {
                            let _ = ListRow::new(1u8, "Layer")
                                .thumb(ThumbSize::Small, |_, _| {})
                                .layer(layer)
                                .selected(selected)
                                .highlight(hovered)
                                .show(ui);
                        })
                    },
                )
            });
            (style.expect("style"), drawn)
        };
        let (s, plain_hover) = drawn(false, false, true);
        assert_eq!(fills(&plain_hover).first().map(|r| r.fill), Some(s.hover), "{id}: the plain hover");
        let (_, hover) = drawn(true, false, true);
        assert_eq!(fills(&hover).first().map(|r| r.fill), Some(s.layer_hover), "{id}: the layer hover");
        assert_ne!(s.layer_hover, s.hover, "{id}");

        let (_, plain) = drawn(false, true, false);
        assert_eq!(fills(&plain).first().map(|r| r.fill), Some(s.selected), "{id}");
        let (_, selected) = drawn(true, true, true);
        assert_eq!(fills(&selected).first().map(|r| r.fill), Some(s.layer_selected), "{id}: no change under the pointer");
        let row_outline = |shapes: &[Shape]| {
            shapes.iter().any(|s2| matches!(s2, Shape::Rect(r) if r.stroke.color == s.layer_outline && r.stroke.width == 1.0 && r.stroke_kind == egui::StrokeKind::Inside))
        };
        assert_eq!(row_outline(&selected), outlined, "{id}");
        assert_eq!(shapes_radius(&selected), corner_radius(s.layer_radius), "{id}: the fill and outline radius");
        assert!(!row_outline(&plain), "{id}: a plain row has no outline");
        assert_eq!(s.layer_outline != egui::Color32::TRANSPARENT, outlined, "{id}");
        let frame = selected.iter().find_map(|s2| match s2 {
            Shape::Rect(r) if r.stroke.width > 0.0 && r.stroke_kind == egui::StrokeKind::Outside => Some(r.corner_radius),
            _ => None,
        });
        assert_eq!(frame, Some(corner_radius(if outlined { 5.0 } else { 0.0 })), "{id}: the thumbnail frame");
        let text = label_colors(&selected);
        assert_eq!(text.first(), Some(&s.layer_selected_text), "{id}");
    }
}

fn shapes_radius(shapes: &[Shape]) -> egui::CornerRadius {
    shapes.iter().find_map(|s| if let Shape::Rect(r) = s { Some(r.corner_radius) } else { None }).unwrap_or_default()
}

/// A second thumbnail has the first one's size and frame and sits `list.icon` after it with a
/// link glyph between (`list.gap` without one); a click on the glyph asks for the opposite state
/// and the label starts after both.
#[test]
fn a_second_thumbnail_follows_the_first_with_an_optional_link_between() {
    use std::cell::RefCell;
    type State = (bool, RefCell<Vec<Rect>>, Vec<RowEvent<u8>>, Rect);
    let s = style();
    for with_link in [true, false] {
        let mut harness = harness::<State>(Direction::Ltr).build_ui_state(
            move |ui, (linked, painted, events, label): &mut State| {
                painted.get_mut().clear();
                let painted = &*painted;
                let mut row = ListRow::new(1u8, "Layer")
                    .selected(true)
                    .thumb(ThumbSize::Medium, |_, rect| painted.borrow_mut().push(rect))
                    .second_thumb(|_, rect| painted.borrow_mut().push(rect));
                if with_link {
                    row = row.thumb_link(*linked, "Link mask to {row}");
                }
                let shown = row.show(ui);
                *label = shown.label;
                for event in shown.events {
                    if let RowEvent::LinkToggled { linked: on, .. } = event {
                        *linked = on;
                    }
                    events.push(event);
                }
            },
            (false, RefCell::default(), Vec::new(), Rect::NOTHING),
        );
        harness.run();
        let (_, painted, ..) = harness.state();
        let painted = painted.borrow().clone();
        let [first, second] = painted[..] else { panic!("{painted:?}") };
        assert_eq!(first.size(), second.size());
        assert_eq!(first.size(), vec2(s.thumb.md, s.thumb.md));
        let between = if with_link { s.icon_size } else { s.gap };
        assert_eq!(second.left() - first.right(), between);
        assert!(harness.state().3.left() >= second.right(), "the label follows both thumbnails");
        if with_link {
            harness.get_by_label("Link mask to Layer").click();
            harness.run();
            assert_eq!(harness.state().2, vec![RowEvent::LinkToggled { row: 1, linked: true }]);
            assert!(harness.state().0);
        } else {
            assert!(harness.query_by_label("Link mask to Layer").is_none());
        }
    }
}

type Thumbs = (Option<ThumbTarget>, std::cell::RefCell<Vec<Rect>>, Vec<RowEvent<u8>>, usize);

fn thumbs_harness(target: Option<ThumbTarget>, selected: bool) -> Harness<'static, Thumbs> {
    let mut h = harness::<Thumbs>(Direction::Ltr).build_ui_state(
        move |ui, (target, painted, events, clicks): &mut Thumbs| {
            painted.get_mut().clear();
            let painted = &*painted;
            let mut row = ListRow::new(1u8, "Layer")
                .selected(selected)
                .thumb(ThumbSize::Medium, |_, rect| painted.borrow_mut().push(rect))
                .second_thumb(|_, rect| painted.borrow_mut().push(rect))
                .thumb_link(true, "Link mask to {row}");
            if let Some(target) = *target {
                row = row.thumb_target(target);
            }
            let shown = row.show(ui);
            *clicks += usize::from(shown.response.clicked());
            events.extend(shown.events);
        },
        (target, std::cell::RefCell::default(), Vec::new(), 0),
    );
    h.run();
    h
}

fn click_at(h: &mut Harness<'_, Thumbs>, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    h.event(button(pos, true, Modifiers::NONE));
    h.event(button(pos, false, Modifiers::NONE));
    h.run();
}

/// Advances the clock so the next press is a separate click.
fn later(h: &mut Harness<'_, Thumbs>) {
    let now = h.ctx.input(|i| i.time);
    h.input_mut().time = Some(now + 1.0);
}

/// The end points of the 1.5-wide bracket segments in `color` the last frame painted.
fn bracket_segments(h: &Harness<'_, Thumbs>, color: egui::Color32) -> Vec<[Pos2; 2]> {
    fn collect(shape: &Shape, color: egui::Color32, out: &mut Vec<[Pos2; 2]>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, color, out)),
            Shape::LineSegment { points, stroke } if stroke.color == color && (stroke.width - 1.5).abs() < 1e-3 => out.push(*points),
            _ => {}
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|s| collect(&s.shape, color, &mut out));
    out
}

/// The target brackets are eight 1.5-wide segments 6 long at the corners of the targeted
/// thumbnail grown by 3; nothing is marked without a target or on a row that is not selected.
#[test]
fn the_targeted_thumbnail_has_corner_brackets() {
    let color = style().selected_text;
    for target in [ThumbTarget::First, ThumbTarget::Second] {
        let h = thumbs_harness(Some(target), true);
        let [first, second] = h.state().1.borrow()[..] else { panic!("two thumbnails") };
        let marked = if target == ThumbTarget::First { first } else { second }.expand(3.0);
        let segments = bracket_segments(&h, color);
        assert_eq!(segments.len(), 8, "{target:?}");
        let corners = [marked.left_top(), marked.right_top(), marked.right_bottom(), marked.left_bottom()];
        for [a, b] in segments {
            assert!(corners.contains(&a) && ((a - b).length() - 6.0).abs() < 1e-3, "{target:?}: {a:?} {b:?}");
        }
    }
    assert!(bracket_segments(&thumbs_harness(None, true), color).is_empty(), "no target, no brackets");
    assert!(bracket_segments(&thumbs_harness(Some(ThumbTarget::Second), false), color).is_empty(), "a row that is not selected");
}

/// A click on a thumbnail reports it and the row is clicked too; the link glyph between the
/// thumbnails and the label report no thumbnail.
#[test]
fn a_click_on_a_thumbnail_reports_which_and_still_clicks_the_row() {
    let mut h = thumbs_harness(Some(ThumbTarget::First), true);
    let [first, second] = h.state().1.borrow()[..] else { panic!("two thumbnails") };
    click_at(&mut h, second.center());
    assert_eq!(h.state().2, vec![RowEvent::ThumbClicked { row: 1, which: ThumbTarget::Second }]);
    assert_eq!(h.state().3, 1, "the row selects as for any click");
    later(&mut h);
    click_at(&mut h, first.center());
    assert_eq!(h.state().2.last(), Some(&RowEvent::ThumbClicked { row: 1, which: ThumbTarget::First }));
    assert_eq!(h.state().3, 2);

    let before = h.state().2.len();
    later(&mut h);
    click_at(&mut h, pos2(second.right() + 200.0, second.center().y));
    assert_eq!(h.state().2.len(), before, "a click beside the thumbnails reports none");
    assert_eq!(h.state().3, 3);
}

/// The link glyph shows the focus ring when it has the keyboard focus.
#[test]
fn the_link_glyph_shows_a_focus_ring() {
    let ctx = context();
    let mut ring = None;
    ctx.run_ui(egui::RawInput::default(), |ui| ring = Some(crate::primitives::FocusRing::style(&Tokens::of(ui)).stroke.color))
        .drop_without_applying_deltas();
    let ring = ring.unwrap_or_default();
    let mut h = thumbs_harness(Some(ThumbTarget::First), true);
    let focus_rects =
        |h: &Harness<'_, Thumbs>| h.output().shapes.iter().filter(|s| matches!(&s.shape, Shape::Rect(r) if r.stroke.color == ring)).count();
    assert_eq!(focus_rects(&h), 0);
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(focus_rects(&h), 1, "Tab reaches the glyph and rings it");
}

#[test]
fn a_tinted_toggle_cell_is_filled_and_keeps_its_icon() {
    let ctx = context();
    let tint = craft_theme::MenuHue::Red.color();
    let row = |tinted: bool| {
        shapes(&ctx, |ui| {
            panel(ui, 240.0, |ui| {
                let eye = ToggleColumn::eye("Visibility {row}", true);
                let _ = ListRow::new(1u8, "Layer").toggle(if tinted { eye.tint(tint) } else { eye }).show(ui);
            })
        })
    };
    let (plain, tinted) = (row(false), row(true));
    let mut expected = None;
    ctx.run_ui(egui::RawInput::default(), |ui| expected = Some(crate::menu::row::tint_colors(&Tokens::of(ui), tint.into())))
        .drop_without_applying_deltas();
    let expected = expected.expect("tint");
    let fill = crate::theme::ToEgui::to_egui(expected.fill);
    assert!(fills(&plain).iter().all(|r| r.fill != fill));
    assert_eq!(fills(&tinted).iter().filter(|r| r.fill == fill).count(), 1, "the cell is filled with the tint");
    assert_eq!(tinted.len(), plain.len() + 1, "the eye is still painted over the fill");
}

/// Which toggle cells had a secondary click, accumulated over frames.
#[derive(Default)]
struct Cells {
    secondary: [bool; 2],
    toggles: usize,
}

#[test]
fn a_toggle_cell_reports_its_own_response() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, cells: &mut Cells| {
            panel(ui, 240.0, |ui| {
                let out = ListRow::new(1u8, "Layer 1")
                    .toggle(ToggleColumn::eye("Visibility {row}", true))
                    .toggle(ToggleColumn::lock("Lock {row}", false))
                    .show(ui);
                for (seen, response) in cells.secondary.iter_mut().zip(&out.toggles) {
                    *seen |= response.secondary_clicked();
                }
                cells.toggles += out.events.iter().filter(|e| matches!(e, RowEvent::Toggle(_))).count();
            })
        },
        Cells::default(),
    );
    harness.run();
    let at = harness.get_by_label("Visibility Layer 1").rect().center();
    harness.event(Event::PointerMoved(at));
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        harness.run();
    }
    assert_eq!((harness.state().secondary, harness.state().toggles), ([true, false], 0), "the eye's secondary click, and no toggle");
}

/// Two renamable rows; `start` opens the rename of the other one by a command of the host.
struct Commands {
    labels: [String; 2],
    renaming: Option<usize>,
    start: Option<usize>,
}

#[test]
fn the_host_commits_an_open_rename_when_it_starts_another() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, state: &mut Commands| {
            panel(ui, 240.0, |ui| {
                if let Some(next) = state.start.take()
                    && let Some(open) = state.renaming.replace(next)
                    && let Some(text) = ListRow::<usize>::take_rename_draft(ui, &open)
                {
                    state.labels[open] = text;
                }
                for at in 0..2 {
                    let out = ListRow::new(at, state.labels[at].clone()).renaming(state.renaming == Some(at)).show(ui);
                    for event in out.events {
                        if let RowEvent::RenameEnded { .. } = event {
                            state.renaming = None;
                        }
                    }
                }
            })
        },
        Commands { labels: ["Layer 1".into(), "Layer 2".into()], renaming: Some(0), start: None },
    );
    harness.run();
    harness.event(Event::Text("Sky".to_owned()));
    harness.run();
    harness.state_mut().start = Some(1);
    harness.run();
    harness.run();
    let state = harness.state();
    assert_eq!((state.labels[0].as_str(), state.renaming), ("Sky", Some(1)), "the open rename was committed");
    assert_eq!(harness.query_all_by_role(Role::TextInput).count(), 1);
    assert_eq!(harness.get_by_role(Role::TextInput).accesskit_node().label().as_deref(), Some("Layer 2"));
}
