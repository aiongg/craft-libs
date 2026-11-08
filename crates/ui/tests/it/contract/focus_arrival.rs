//! `Focus::focus_arrived`: the part focus moved onto in this frame.

use super::kit::PlainStyle;
use craft_behavior::{Input, NavShape, PartKey, PartRef, Roving, Sense};
use craft_ui::contract::{Common, Component, Cx, Focus as _, MeasureCx, PartsCx, Props as _, Role, Show as _};
use craft_ui::runtime::focus::KeyUse;
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Rect, Response, Ui, Vec2};
use std::cell::RefCell;

const LEFT: PartKey = PartKey::new("left");
const RIGHT: PartKey = PartKey::new("right");

/// Two focusable parts in one roving group; every pass records what `focus_arrived` reads.
#[derive(Component)]
struct Pair<'a> {
    #[common]
    common: Common,
    arrivals: &'a RefCell<Vec<Option<PartRef>>>,
}

impl Component for Pair<'_> {
    const ROLE: Role = Role::RADIO_GROUP;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer);
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &craft_ui::style::StyleCx<'_>, _: PairStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let half = |from: f32| Rect::from_min_size(rect.min + Vec2::new(from, 0.0), Vec2::new(40.0, 20.0));
        cx.part(LEFT, half(0.0)).sense(Sense::CLICK | Sense::FOCUS).focusable().selected();
        cx.part(RIGHT, half(40.0)).sense(Sense::CLICK | Sense::FOCUS).focusable();
        cx.roving(Roving::new(NavShape::Flat));
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.arrivals.borrow_mut().push(cx.focus_arrived());
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Tab into a part group reports the arrival part in that frame only; an arrow within the group reports the new
/// part; a component that focus left reports none.
#[trial]
fn focus_arrival_names_the_part_focus_moved_onto_in_the_frame_it_did() {
    let (first, second) = (RefCell::new(Vec::new()), RefCell::new(Vec::new()));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Pair { common: Common::new(), arrivals: &first }.id_salt("first").show(ui);
        Pair { common: Common::new(), arrivals: &second }.id_salt("second").show(ui);
    };
    let mut frame = |frames: &mut Frames, key: Option<(Key, Modifiers)>| {
        first.borrow_mut().clear();
        second.borrow_mut().clear();
        frames.frame(key.map(|(key, modifiers)| key_press(key, modifiers)).into_iter().collect(), &mut show);
        (first.borrow().clone(), second.borrow().clone())
    };
    let only = |seen: &[Option<PartRef>], part: Option<PartRef>| !seen.is_empty() && seen.iter().all(|arrived| *arrived == part);
    for _ in 0..3 {
        frame(&mut frames, None);
    }
    let (first_seen, second_seen) = frame(&mut frames, Some((Key::Tab, Modifiers::NONE)));
    assert!(only(&first_seen, Some(PartRef::from(LEFT))), "Tab into the group arrives at its selected part: {first_seen:?}");
    assert!(only(&second_seen, None), "the other component sees no arrival: {second_seen:?}");
    let (first_seen, _) = frame(&mut frames, None);
    assert!(only(&first_seen, None), "the next frame reports none: {first_seen:?}");
    let (first_seen, _) = frame(&mut frames, Some((Key::ArrowRight, Modifiers::NONE)));
    assert!(only(&first_seen, Some(PartRef::from(RIGHT))), "an arrow within the group reports the new part: {first_seen:?}");
    let (first_seen, second_seen) = frame(&mut frames, Some((Key::Tab, Modifiers::NONE)));
    assert!(only(&first_seen, None), "the component focus left reports none: {first_seen:?}");
    assert!(only(&second_seen, Some(PartRef::from(LEFT))), "the next one reports its arrival part: {second_seen:?}");
}
