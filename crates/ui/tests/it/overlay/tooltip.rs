//! Tooltips: shown after a delay, hidden on a press.

use super::kit::{Leaf, Seen};
use craft_behavior::IconId;
use craft_ui::buttons::IconButton;
use craft_ui::contract::tooltip::DELAY;
use craft_ui::contract::{Bind, Props as _, Role, Show as _};
use craft_ui::runtime::overlay::{OpenFilter, any_open};
use craft_ui::testing::pointer::{moved, press};
use craft_ui::testing::{Fixture, Frames, trial};
use craft_ui::value_drag::Slider;
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const TARGET: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(180.0, 130.0) };

#[trial]
fn a_tooltip_shows_after_its_delay_and_hides_on_press() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        Leaf::new(&seen, Vec2::new(80.0, 30.0)).id_salt("target").tooltip("Hint").show_at(ui, TARGET);
    };
    frames.idle(3, &mut show);
    let at = TARGET.center();
    frames.frame(vec![moved(at)], &mut show);
    frames.idle(10, &mut show);
    assert!(!any_open(&frames.ctx, OpenFilter::Any), "the tooltip waits for its delay");
    frames.advance(DELAY);
    frames.idle(3, &mut show);
    assert!(any_open(&frames.ctx, OpenFilter::Any), "the tooltip shows after its delay");
    frames.frame(vec![press(at)], &mut show);
    frames.idle(30, &mut show);
    assert!(!any_open(&frames.ctx, OpenFilter::Any), "a press hides the tooltip");
}

#[trial]
fn a_shown_tooltip_is_a_node_named_by_its_text_and_describes_its_control() {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        Leaf::new(&shown, Vec2::new(80.0, 30.0)).id_salt("target").tooltip("Hides the layer").show_at(ui, TARGET);
    });
    fixture.settle();
    fixture.frame(vec![moved(TARGET.center())]);
    fixture.advance(DELAY);
    fixture.idle(3);
    let tips = fixture.by_role(Role::TOOLTIP);
    assert_eq!(tips.len(), 1, "one tooltip node is shown");
    assert!(tips.first().and_then(|tip| fixture.tree().name_of(tip.node)).is_some_and(|name| !name.is_empty()), "the tip text names the node");
    let control = seen.borrow().id;
    let tip_nodes: Vec<_> = tips.iter().map(|tip| tip.node).collect();
    assert_eq!(control.map(|id| fixture.tree().described_by(id)), Some(tip_nodes), "the control is described by the tip");
}

/// Whether resting the pointer on the shown component past the delay opens a tooltip.
fn tooltip_opens(mut show_component: impl FnMut(&mut Ui)) -> bool {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_component(ui);
    frames.idle(3, &mut show);
    frames.frame(vec![moved(TARGET.center())], &mut show);
    frames.advance(DELAY);
    frames.idle(3, &mut show);
    any_open(&frames.ctx, OpenFilter::Any)
}

#[trial]
fn an_icon_only_face_shows_its_label_as_tooltip_and_a_slider_does_not() {
    let icon = tooltip_opens(|ui| {
        IconButton::new(IconId::new("close")).label("Close").show_at(ui, TARGET);
    });
    assert!(icon, "an icon button shows its label as tooltip");
    let mut value = 40.0;
    let slider = tooltip_opens(|ui| {
        Slider::new(Bind::bound(&mut value), 0.0..=100.0).label("Opacity").show_at(ui, TARGET);
    });
    assert!(!slider, "a slider that paints no text shows no label tooltip");
}
