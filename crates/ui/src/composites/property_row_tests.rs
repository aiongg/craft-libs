use egui::accesskit::Role;
use egui::text_selection::LabelSelectionState;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::property_row::{InspectorArgs, show_inspector};
use super::*;
use crate::controls::{IconButton, NumberField};
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};
use craft_theme::ControlSize;

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

#[allow(clippy::expect_used)]
fn tokens_style() -> PropertyStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(PropertyRow::style(&Tokens::of(ui), ControlSize::Md))).drop_without_applying_deltas();
    style.expect("style")
}

/// A panel `width` wide at the top left.
fn panel(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        add(ui);
    });
}

fn number<'a>(id: &'static str, label: &'static str, value: f64, numbers: &'a LocaleNumbers) -> NumberField<'a> {
    NumberField::new(id, label, value, numbers)
}

fn spin<'a, State>(harness: &'a Harness<'_, State>, label: &'a str) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some(label))
}

fn text<'a, State>(harness: &'a Harness<'_, State>, value: &'a str) -> Option<Node<'a>> {
    harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value))
}

#[track_caller]
#[allow(clippy::panic)]
fn rect<State>(harness: &Harness<'_, State>, value: &str) -> Rect {
    text(harness, value).unwrap_or_else(|| panic!("no label {value}")).rect()
}

fn glyphs(ui: &mut Ui) {
    let numbers = LocaleNumbers::from_tag("en");
    panel(ui, 260.0, |ui| {
        PropertyGroup::new("g").show(ui, |ui| {
            PropertyRow::glyph("W").show(ui, |ui, _| number("a", "Width", 1.0, &numbers).show(ui));
            PropertyRow::glyph("i").show(ui, |ui, _| number("b", "Inset", 2.0, &numbers).show(ui));
        });
    });
}

#[test]
fn glyph_labels_share_a_centre_with_the_widest_against_the_control() {
    let harness = harness(Direction::Ltr).build_ui(glyphs);
    let (wide, narrow) = (rect(&harness, "W"), rect(&harness, "i"));
    let (a, b) = (spin(&harness, "Width").rect(), spin(&harness, "Inset").rect());
    let style = tokens_style();
    assert!(wide.width() > narrow.width());
    assert!((wide.center().x - narrow.center().x).abs() < 0.6, "{wide:?} {narrow:?}");
    assert!((a.left() - b.left()).abs() < 0.5, "controls start together: {a:?} {b:?}");
    assert!(a.left() - wide.right() - style.label_gap < 1.5, "the widest is flush with the control: {wide:?} {a:?}");
    assert!(wide.left() >= 0.0 && narrow.left() > wide.left());
}

#[test]
fn right_to_left_mirrors_the_label_column() {
    let harness = harness(Direction::Rtl).build_ui(glyphs);
    let (wide, narrow) = (rect(&harness, "W"), rect(&harness, "i"));
    let a = spin(&harness, "Width").rect();
    assert!((wide.center().x - narrow.center().x).abs() < 0.6);
    assert!(a.right() < wide.left() && a.left() < 260.0 - 100.0, "{wide:?} {a:?}");
    assert!((a.right() - spin(&harness, "Inset").rect().right()).abs() < 0.5);
    assert!(wide.right() > 200.0, "the labels are at the right: {wide:?}");
}

#[test]
fn the_control_fills_the_row_and_keeps_its_minimum() {
    let style = tokens_style();
    for width in [260.0, 30.0] {
        let row = std::cell::Cell::new(Rect::NOTHING);
        let harness = harness(Direction::Ltr).build_ui(|ui| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, width, |ui| {
                let shown = PropertyRow::word("Opacity").redundant().show(ui, |ui, _| number("o", "Opacity", 50.0, &numbers).show(ui));
                row.set(shown.response.rect);
            });
        });
        let field = spin(&harness, "Opacity").rect();
        assert!(field.width() >= style.min_control - 0.5, "{field:?}");
        if width > 200.0 {
            assert!((field.right() - row.get().right()).abs() < 0.5, "fills the row: {field:?} {:?}", row.get());
            let label = rect(&harness, "Opacity");
            assert!((field.left() - row.get().left() - style.word_width - style.label_gap).abs() < 0.5, "after the word column: {label:?} {field:?}");
        } else {
            assert!((field.left() - row.get().left()).abs() < 0.5, "no label: {field:?}");
        }
    }
}

#[test]
fn a_pair_splits_the_width_equally_around_the_link_slot() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        let mut linked = true;
        panel(ui, 300.0, |ui| {
            let _ = PropertyRow::pair(PropertyLabel::glyph("W"), PropertyLabel::glyph("H")).show_linked(
                ui,
                |ui, _| number("w", "Width", 1.0, &numbers).show(ui),
                |ui| ui.add(IconButton::new(Icon::Lock, "Keep proportions").size(ControlSize::Sm).toggle(&mut linked)),
                |ui, _| number("h", "Height", 2.0, &numbers).show(ui),
            );
        });
    });
    let style = tokens_style();
    let (w, h) = (spin(&harness, "Width").rect(), spin(&harness, "Height").rect());
    let link = harness.get_by_label("Keep proportions").rect();
    assert!((w.width() - h.width()).abs() < 0.5, "{w:?} {h:?}");
    assert!((link.width() - style.link_width).abs() < 0.5, "{link:?}");
    assert!((link.left() - w.right() - style.column_gap).abs() < 0.5, "{w:?} {link:?}");
    let label_h = rect(&harness, "H");
    assert!(
        label_h.left() >= link.right() + style.column_gap && (h.left() - label_h.right() - style.label_gap).abs() < 1.5,
        "{link:?} {label_h:?} {h:?}"
    );
    assert!((h.right() - w.left() - 300.0).abs() < 30.0);
}

#[test]
fn a_pair_with_a_reserved_link_slot_has_the_columns_of_a_linked_pair() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 300.0, |ui| {
            let _ = PropertyRow::pair(PropertyLabel::glyph("W"), PropertyLabel::glyph("H")).show_linked(
                ui,
                |ui, _| number("w", "Width", 1.0, &numbers).show(ui),
                |ui| ui.label("link"),
                |ui, _| number("h", "Height", 2.0, &numbers).show(ui),
            );
            let _ = PropertyRow::pair(PropertyLabel::glyph("X"), PropertyLabel::glyph("Y")).reserve_link_slot().show(
                ui,
                |ui, _| number("x", "X position", 1.0, &numbers).show(ui),
                |ui, _| number("y", "Y position", 2.0, &numbers).show(ui),
            );
        });
    });
    for (above, below) in [("Width", "X position"), ("Height", "Y position")] {
        let (a, b) = (spin(&harness, above).rect(), spin(&harness, below).rect());
        assert!((a.left() - b.left()).abs() < 0.5 && (a.right() - b.right()).abs() < 0.5, "{above} {a:?} {below} {b:?}");
    }
}

#[test]
fn a_pair_without_a_link_is_split_by_the_column_gap() {
    let style = tokens_style();
    for dir in [Direction::Ltr, Direction::Rtl] {
        let row = std::cell::Cell::new(Rect::NOTHING);
        let harness = harness(dir).build_ui(|ui| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, 300.0, |ui| {
                let shown = PropertyRow::pair(PropertyLabel::glyph("X"), PropertyLabel::glyph("Y")).show(
                    ui,
                    |ui, _| number("x", "X position", 1.0, &numbers).show(ui),
                    |ui, _| number("y", "Y position", 2.0, &numbers).show(ui),
                );
                row.set(shown.response.rect);
            });
        });
        let row = row.get();
        let (x, y) = (spin(&harness, "X position").rect(), spin(&harness, "Y position").rect());
        let (label_x, label_y) = (rect(&harness, "X"), rect(&harness, "Y"));
        let column = (row.width() - style.column_gap) / 2.0;
        assert!((x.width() - y.width()).abs() < 0.5, "{x:?} {y:?}");
        if dir == Direction::Ltr {
            assert!((x.right() - row.left() - column).abs() < 0.5, "the first column is half the row less the gap: {x:?} {row:?}");
            assert!((label_y.left() - x.right()) >= style.column_gap - 0.5, "{x:?} {label_y:?}");
            assert!((y.right() - row.right()).abs() < 0.5, "{y:?} {row:?}");
            let second = row.left() + column + style.column_gap;
            assert!(label_y.left() >= second - 0.5 && label_x.left() >= row.left() - 0.5, "{label_y:?} starts the second column at {second}");
        } else {
            assert!((row.right() - x.left() - column).abs() < 0.5, "{x:?} {row:?}");
            assert!((y.left() - row.left()).abs() < 0.5, "{y:?} {row:?}");
            assert!(x.left() - label_y.right() >= style.column_gap - 0.5, "{x:?} {label_y:?}");
        }
    }
}

#[test]
fn an_essential_label_truncates_and_keeps_its_full_name() {
    let style = tokens_style();
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 100.0, |ui| {
            PropertyRow::word("Opacity of the layer").show(ui, |ui, _| number("o", "Opacity", 50.0, &numbers).show(ui));
        });
    });
    let field = harness.get_by(|n| n.role() == Role::SpinButton).rect();
    assert!(field.width() >= style.min_control - 0.5, "{field:?}");
    let label = rect(&harness, "Opacity of the layer");
    assert!(label.width() < 80.0, "{label:?}");
    let node = harness.get_by(|n| n.role() == Role::SpinButton);
    assert_eq!(node.accesskit_node().label().as_deref(), Some("Opacity of the layer"), "the word label names the control, uncut");
    a11y::assert_labelled(&harness);
}

#[test]
fn a_redundant_label_hides_when_narrow_and_the_control_keeps_its_name() {
    let row = |width: f32| {
        harness(Direction::Ltr).build_ui(move |ui| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, width, |ui| {
                PropertyRow::word("Opacity").redundant().show(ui, |ui, _| number("o", "Opacity", 50.0, &numbers).show(ui));
            });
        })
    };
    let wide = row(260.0);
    assert!(text(&wide, "Opacity").is_some());
    let narrow = row(90.0);
    assert!(text(&narrow, "Opacity").is_none(), "hidden");
    let field = spin(&narrow, "Opacity");
    assert!(field.accesskit_node().data().labelled_by().is_empty());
    assert!(field.rect().width() >= tokens_style().min_control, "{:?}", field.rect());
    a11y::assert_labelled(&narrow);
}

#[test]
fn a_hidden_redundant_label_leaves_the_control_its_own_name() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 110.0, |ui| {
            let _ = PropertyRow::word("Size").redundant().show(ui, |ui, _| number("s", "Brush size", 5.0, &numbers).show(ui));
        });
    });
    assert!(text(&harness, "Size").is_none(), "hidden");
    let node = harness.get_by(|n| n.role() == Role::SpinButton).accesskit_node();
    assert_eq!(node.label().as_deref(), Some("Brush size"));
    assert!(node.data().labelled_by().is_empty());
}

#[test]
fn a_word_label_names_its_control_and_a_glyph_label_does_not() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 260.0, |ui| {
            PropertyGroup::new("g").show(ui, |ui| {
                PropertyRow::glyph("W").show(ui, |ui, _| number("w", "Width", 1.0, &numbers).show(ui));
                PropertyRow::word("Opacity").show(ui, |ui, _| number("o", "Layer opacity", 2.0, &numbers).show(ui));
            });
        });
    });
    let spins: Vec<_> = harness.get_all_by(|n| n.role() == Role::SpinButton).map(|n| n.accesskit_node()).collect();
    let [glyph, word] = spins.as_slice() else { panic!("two spin buttons: {spins:?}") };
    assert_eq!(glyph.label().as_deref(), Some("Width"), "a glyph row keeps the control's name");
    assert!(glyph.data().labelled_by().is_empty());
    let opacity = text(&harness, "Opacity").expect("label").accesskit_node().locate().0;
    assert_eq!(word.label().as_deref(), Some("Opacity"), "a word row is named by its label");
    assert_eq!(word.data().labelled_by(), [opacity]);
    a11y::assert_labelled(&harness);
}

#[test]
fn a_scrubbing_label_changes_the_value() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, value: &mut f64| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, 260.0, |ui| {
                let edit = PropertyRow::word("Opacity")
                    .scrubs()
                    .show(ui, |ui, row| {
                        let field = number("o", "Opacity", *value, &numbers);
                        match row.scrub() {
                            Some(label) => field.scrub_by(label),
                            None => field,
                        }
                        .show(ui)
                    })
                    .inner;
                if let Some(EditEvent::Update { value: v, .. } | EditEvent::Commit { value: v, .. } | EditEvent::Set { value: v, .. }) = edit.event {
                    *value = v;
                }
            });
        },
        50.0,
    );
    let at = rect(&harness, "Opacity").center();
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let selection = |harness: &Harness<'_, f64>| harness.ctx.with_plugin(|s: &mut LabelSelectionState| s.has_selection()).unwrap_or(false);
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.event(button(at, true));
    harness.run();
    for dx in [10.0, 30.0] {
        harness.event(Event::PointerMoved(at + vec2(dx, 0.0)));
        harness.run();
        assert!(!selection(&harness), "the drag selects no label text");
    }
    harness.event(button(at + vec2(30.0, 0.0), false));
    harness.run();
    assert_ne!(*harness.state(), 50.0, "dragging the label scrubbed the field");
    assert!(!selection(&harness));
}

#[test]
fn the_inspector_story_names_every_control_at_every_width() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for (width, mixed) in [(60.0, false), (150.0, true), (260.0, false)] {
            let mut args = InspectorArgs { mixed, width };
            let harness = harness(dir).build_ui(move |ui| show_inspector(ui, &mut args));
            a11y::assert_labelled(&harness);
        }
    }
}

#[test]
fn the_first_frame_already_shows_the_shared_centre() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let frame = |ctx: &Context| {
        let mut labels = Vec::new();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            labels.clear();
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, 260.0, |ui| {
                PropertyGroup::new("g").show(ui, |ui| {
                    // The narrow label first, so that the first pass draws it at its own centre.
                    for (glyph, id) in [("i", "b"), ("W", "a")] {
                        PropertyRow::glyph(glyph).show(ui, |ui, row| {
                            labels.push(row.response().map_or(Rect::NOTHING, |r| r.rect));
                            number(id, glyph, 1.0, &numbers).show(ui)
                        });
                    }
                });
            });
        });
        let passes = output.platform_output.num_completed_passes;
        output.drop_without_applying_deltas();
        (passes, labels)
    };
    let (passes, labels) = frame(&ctx);
    assert_eq!(passes, 2, "the pass that measured the labels is discarded");
    let [narrow, wide] = labels.as_slice() else { panic!("two labels: {labels:?}") };
    assert!(
        narrow.width() < wide.width() && (narrow.center().x - wide.center().x).abs() < 0.6,
        "the shown pass shares the centre: {narrow:?} {wide:?}"
    );
    assert_eq!(frame(&ctx).0, 1, "an unchanged column lays out once");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "two PropertyGroups share one id")]
fn two_groups_of_one_id_under_one_parent_fail_in_debug() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        for _ in 0..2 {
            PropertyGroup::new("same").show(ui, |ui| {
                PropertyRow::glyph("X").show(ui, |ui, _| number("x", "X", 1.0, &numbers).show(ui));
            });
        }
    });
}

#[test]
fn a_group_overrides_the_column_widths() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 260.0, |ui| {
            PropertyGroup::new("g").word_width(120.0).show(ui, |ui| {
                PropertyRow::word("Angle").show(ui, |ui, _| number("a", "Angle", 1.0, &numbers).show(ui));
            });
        });
    });
    let style = tokens_style();
    let (label, field) = (rect(&harness, "Angle"), spin(&harness, "Angle").rect());
    assert!((field.left() - label.left() - 120.0 - style.label_gap).abs() < 1.5, "{label:?} {field:?}");
}

/// PhotoCraft's panel rows are `ROW_GAP` apart (`props_layout.rs:12-30`; docks zero egui's item
/// spacing and the rows add the gap themselves).
#[test]
fn the_rows_of_a_group_are_a_row_gap_apart() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 260.0, |ui| {
            PropertyGroup::new("g").show(ui, |ui| {
                PropertyRow::word("Opacity").show(ui, |ui, _| number("o", "Opacity", 1.0, &numbers).show(ui));
                PropertyRow::word("Angle").show(ui, |ui, _| number("a", "Angle", 1.0, &numbers).show(ui));
            });
        });
    });
    let gap = spin(&harness, "Angle").rect().top() - spin(&harness, "Opacity").rect().bottom();
    assert!((gap - harness_row_gap()).abs() < 0.5, "{gap}");
}

fn harness_row_gap() -> f32 {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut gap = 0.0;
    ctx.run_ui(egui::RawInput::default(), |ui| gap = Tokens::of(ui).space.gap.row).drop_without_applying_deltas();
    gap
}
