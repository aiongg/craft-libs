//! Options bar tests: heights, the trailing slot, scrolling and focus reveal, cells, groups,
//! names and mirroring.

use std::cell::Cell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui::{Context, Rect, Ui, Vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::options_bar::stories;
use super::*;
use crate::controls::{CheckState, Checkbox, NumberField};
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::LocaleNumbers;
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn harness<State>(dir: Direction, width: f32) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([width, 120.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

#[allow(clippy::expect_used)]
fn style(narrow: bool) -> OptionsBarStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(OptionsBar::style(&Tokens::of(ui), narrow))).drop_without_applying_deltas();
    style.expect("style")
}

/// A bar with a few buttons in the body and, optionally, commit and cancel.
fn bar(ui: &mut Ui, bar_rect: &Cell<Rect>, narrow: bool, session: bool, body_buttons: usize) {
    let response = OptionsBar::new("test").narrow(narrow).show(ui, |bar| {
        bar.leading(|ui| {
            OptionsBar::tool_icon(ui, Icon::Brush, "Brush");
        });
        bar.body(|ui| {
            for i in 0..body_buttons {
                ui.add(crate::controls::Button::new(format!("Option {i}")));
            }
        });
        if session {
            bar.trailing(|ui| {
                OptionsBar::commit_cancel(ui, "Commit transform", "Cancel transform");
            });
        }
    });
    bar_rect.set(response.rect);
}

#[test]
fn the_narrow_bar_is_the_shorter_one_and_nothing_else_changes() {
    let (wide, narrow) = (style(false), style(true));
    assert!(narrow.height < wide.height);
    assert_eq!((narrow.pad, narrow.gap, narrow.fields, narrow.button), (wide.pad, wide.gap, wide.fields, wide.button));
    for (is_narrow, expected) in [(false, wide.height), (true, narrow.height)] {
        let rect = Rc::new(Cell::new(Rect::NOTHING));
        let seen = rect.clone();
        let _harness = harness(Direction::Ltr, 400.0).build_ui(move |ui| bar(ui, &seen, is_narrow, false, 1));
        assert_eq!(rect.get().height(), expected);
    }
}

/// The trailing buttons stay at the reading end however wide the body is, and the body scrolls.
#[test]
fn the_trailing_slot_stays_at_the_end_when_the_body_is_wider_than_the_bar() {
    let style = style(false);
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let seen = rect.clone();
    let mut harness = harness(Direction::Ltr, 300.0).build_ui(move |ui| bar(ui, &seen, false, true, 12));
    harness.run();
    let bar = rect.get();
    let commit = harness.get_by_label("Commit transform").rect();
    let cancel = harness.get_by_label("Cancel transform").rect();
    assert!((bar.right() - style.pad - commit.right()).abs() < 1.0, "commit at the end: {commit:?} in {bar:?}");
    assert!(cancel.right() < commit.left(), "cancel before commit: {cancel:?} {commit:?}");
    let last = harness.get_by_label("Option 11").rect();
    assert!(last.left() > bar.right(), "the body runs past the bar: {last:?}");
    let first = harness.get_by_label("Option 0").rect();
    assert!(first.right() < cancel.left() + 1.0 || first.left() < cancel.left(), "the body scrolls under its own area");
}

/// Tab to a control that is out of view scrolls it into the bar.
#[test]
fn focusing_a_control_out_of_view_scrolls_it_into_view() {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let seen = rect.clone();
    let mut harness = harness(Direction::Ltr, 300.0).build_ui(move |ui| bar(ui, &seen, false, false, 12));
    harness.run();
    assert!(harness.get_by_label("Option 11").rect().left() > rect.get().right());
    harness.get_by_label("Option 11").focus();
    harness.run();
    let option = harness.get_by_label("Option 11").rect();
    assert!(rect.get().contains_rect(option), "{option:?} inside {:?}", rect.get());
}

#[test]
fn commit_and_cancel_have_names_and_report_clicks() {
    let result = Rc::new(Cell::new(CommitCancel::default()));
    let seen = result.clone();
    let mut harness = harness(Direction::Ltr, 400.0).build_ui(move |ui| {
        OptionsBar::new("test").show(ui, |bar| {
            let seen = seen.clone();
            bar.trailing(move |ui| {
                let now = OptionsBar::commit_cancel(ui, "Commit transform", "Cancel transform");
                if now != CommitCancel::default() {
                    seen.set(now);
                }
            });
        });
    });
    harness.run();
    harness.get_by_label("Commit transform").click();
    harness.run();
    assert_eq!(result.get(), CommitCancel { commit: true, cancel: false });
    harness.get_by_label("Cancel transform").click();
    harness.run();
    assert_eq!(result.get(), CommitCancel { commit: false, cancel: true });
}

#[test]
fn a_cell_gives_its_control_the_class_width_and_a_caption_that_is_not_selectable() {
    let style = style(false);
    let widths = Rc::new(Cell::new([0.0_f32; 3]));
    let seen = widths.clone();
    let mut harness = harness(Direction::Ltr, 600.0).build_ui(move |ui| {
        OptionsBar::new("test").show(ui, |bar| {
            let seen = seen.clone();
            bar.body(move |ui| {
                let numbers = LocaleNumbers::from_tag("en-US");
                let mut out = seen.get();
                for (i, class) in [CellWidth::Sm, CellWidth::Md, CellWidth::Lg].into_iter().enumerate() {
                    OptionsBar::cell(ui, &format!("Field {i}:"), class, |ui, _| {
                        out[i] = NumberField::new(("field", i), format!("Field {i}"), 5.0, &numbers).show(ui).response.rect.width();
                    });
                }
                seen.set(out);
            });
        });
    });
    harness.run();
    assert_eq!(widths.get(), [style.fields[0], style.fields[1], style.fields[2]]);
    let caption = harness.get_by_label("Field 0:");
    assert_eq!(caption.accesskit_node().role(), Role::Label);
    assert!(!caption.accesskit_node().data().supports_action(egui::accesskit::Action::Focus), "the caption is not a Tab stop");
}

/// Dragging a caption scrubs the field after it, and the gesture ends as one commit.
#[test]
fn dragging_a_caption_scrubs_its_field() {
    let value = Rc::new(Cell::new(10.0_f64));
    let commits = Rc::new(Cell::new(0));
    let (shown, counted) = (value.clone(), commits.clone());
    let mut harness = harness(Direction::Ltr, 400.0).build_ui(move |ui| {
        OptionsBar::new("test").show(ui, |bar| {
            let (shown, counted) = (shown.clone(), counted.clone());
            bar.body(move |ui| {
                let numbers = LocaleNumbers::from_tag("en-US");
                OptionsBar::cell(ui, "Size:", CellWidth::Md, |ui, caption| {
                    let edit = NumberField::new("size", "Size", shown.get(), &numbers).range(0.0..=1000.0).scrub_by(caption).show(ui);
                    if let Some(event) = &edit.event {
                        if let Some(v) = event.value() {
                            shown.set(*v);
                        }
                        if matches!(event, EditEvent::Commit { .. }) {
                            counted.set(counted.get() + 1);
                        }
                    }
                });
            });
        });
    });
    harness.run();
    let start = harness.get_by_label("Size:").rect().center();
    harness.hover_at(start);
    harness.drag_at(start);
    harness.run();
    harness.hover_at(start + Vec2::new(20.0, 0.0));
    harness.run();
    harness.hover_at(start + Vec2::new(40.0, 0.0));
    harness.run();
    harness.drop_at(start + Vec2::new(40.0, 0.0));
    harness.run();
    assert!(value.get() > 10.0, "scrubbed to {}", value.get());
    assert_eq!(commits.get(), 1);
}

#[test]
fn a_disabled_group_disables_its_controls_and_keeps_the_rest() {
    let mut harness = harness(Direction::Ltr, 400.0).build_ui(|ui| {
        let (mut a, mut b) = (CheckState::Unchecked, CheckState::Unchecked);
        OptionsBar::new("test").show(ui, |bar| {
            bar.body(move |ui| {
                let _ = Checkbox::new("Free", &mut a).show(ui);
                OptionsBar::divider(ui);
                OptionsBar::group(ui, false, |ui| {
                    let _ = Checkbox::new("Grouped", &mut b).show(ui);
                });
            });
        });
    });
    harness.run();
    assert!(!harness.get_by_label("Free").accesskit_node().is_disabled());
    assert!(harness.get_by_label("Grouped").accesskit_node().is_disabled());
}

#[test]
fn the_bar_is_a_named_toolbar() {
    let mut harness = harness(Direction::Ltr, 400.0).build_ui(|ui| {
        OptionsBar::new("test").label("Brush options").show(ui, |bar| {
            bar.leading(|ui| {
                OptionsBar::tool_icon(ui, Icon::Brush, "Brush");
            });
        });
    });
    harness.run();
    let toolbar = harness.get_by_role(Role::Toolbar);
    assert_eq!(toolbar.accesskit_node().label().as_deref(), Some("Brush options"));
}

#[test]
fn the_leading_slot_is_at_the_right_edge_when_mirrored() {
    let style = style(false);
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let seen = rect.clone();
    let mut harness = harness(Direction::Rtl, 400.0).build_ui(move |ui| bar(ui, &seen, false, true, 1));
    harness.run();
    let (bar, brush) = (rect.get(), harness.get_by_label("Brush").rect());
    assert!((bar.right() - style.pad - brush.right()).abs() < 1.0, "leading at the right: {brush:?} in {bar:?}");
    let commit = harness.get_by_label("Commit transform").rect();
    assert!((commit.left() - (bar.left() + style.pad)).abs() < 1.0, "trailing at the left: {commit:?} in {bar:?}");
}

#[test]
fn the_story_names_every_control() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let story = stories().remove(0);
        let mut args = Args::new()
            .with_bool("narrow", false)
            .with_bool("session", true)
            .with_bool("disabled", false)
            .with_number("width", 1200.0, 160.0, 1200.0);
        let show = story.show;
        let mut harness = harness(dir, 800.0).build_ui(move |ui| show(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
    }
}
