//! List row tests: state fills, anatomy, truncation and optional items, the toggle sweep, the
//! rename field, AccessKit and mirroring.

use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::row::show_layers;
use super::*;
use crate::direction::{self, Direction};
use crate::edit::{EditEvent, GestureHost, UndoMarks};
use crate::icons::Icon;
use crate::story::Args;
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
fn dividers_belong_to_compact_and_a_dimmed_label_and_detail_take_the_selected_fill_into_account() {
    let ctx = context();
    let s = style();
    let lines = |size| {
        shapes(&ctx, |ui| {
            scope(
                ui,
                |o| o.size = Some(size),
                |ui| {
                    panel(ui, 240.0, |ui| {
                        let _ = ListRow::new(1u8, "Layer").toggle(ToggleColumn::eye("Visibility {row}", true)).show(ui);
                    })
                },
            )
        })
        .iter()
        .filter(|s| matches!(s, Shape::LineSegment { .. }))
        .count()
    };
    assert_eq!(lines(Size::Compact), 2, "one under the row and one at the column's trailing edge");
    assert_eq!(lines(Size::Comfortable), 0);
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
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 150.0, |ui| {
            let _ = ListRow::new(1u8, "A layer with a long name that cannot fit")
                .trailing_item(16.0, |ui| {
                    ui.label("fx");
                })
                .show(ui);
        })
    });
    let row = harness.get_by_role(Role::ListBoxOption);
    assert_eq!(row.accesskit_node().label().as_deref(), Some("A layer with a long name that cannot fit"));
    let text = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("A layer with a long name that cannot fit")).rect();
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
                let args = Args::new()
                    .with_choice("thumbnails", &["None", "Small", "Medium", "Large"], if thumbs == "None" { 0 } else { 3 })
                    .with_bool("detail", true)
                    .with_bool("highlight second row", true)
                    .with_number("width", width, 60.0, 480.0);
                let mut a = args.clone();
                let harness = harness(dir).build_ui(move |ui| show_layers(ui, &mut a));
                a11y::assert_labelled(&harness);
            }
        }
    }
}
