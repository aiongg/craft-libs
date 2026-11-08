//! `Focus::focus_arrived_by_key`: the arrival of focus without a press.

use crate::contract::kit::PlainStyle;
use craft_behavior::{Input, NavShape, PartKey, PartRef, Roving, Sense};
use craft_ui::contract::{Common, Component, Cx, Focus as _, MeasureCx, PartsCx, Props as _, Role, Show as _};
use craft_ui::runtime::focus::KeyUse;
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Frames, trial};
use egui::{Event, Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::RefCell;

const LEFT: PartKey = PartKey::new("left");
const RIGHT: PartKey = PartKey::new("right");

/// Two focusable parts in one roving group; every pass records what the arrival forms read.
#[derive(Component)]
struct Pair<'a> {
    #[common]
    common: Common,
    seen: &'a RefCell<Vec<(Option<PartRef>, Option<PartRef>)>>,
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
        self.seen.borrow_mut().push((cx.focus_arrived(), cx.focus_arrived_by_key()));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Tab into the group is a key arrival; a press on a part moves focus onto it but is no key arrival.
#[trial]
fn a_tab_arrival_is_a_key_arrival_and_a_press_arrival_is_not() {
    let seen = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let out = Pair { common: Common::new(), seen: &seen }.id_salt("pair").show(ui);
        ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new("pair-rect"), out.rect));
    };
    for _ in 0..3 {
        frames.frame(vec![], &mut show);
    }
    seen.borrow_mut().clear();
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    let tab = seen.borrow().clone();
    assert!(
        !tab.is_empty() && tab.iter().all(|(any, by_key)| *any == Some(PartRef::from(LEFT)) && by_key == any),
        "Tab arrives at the selected part by key: {tab:?}"
    );
    let rect: Rect = frames.ctx.data(|data| data.get_temp(egui::Id::new("pair-rect"))).expect("shown");
    let at: Pos2 = Pos2::new(rect.max.x - 5.0, rect.center().y);
    seen.borrow_mut().clear();
    frames.frame(vec![Event::PointerMoved(at), craft_ui::testing::focus::primary_button(at, true)], &mut show);
    let press = seen.borrow().clone();
    assert!(!press.is_empty() && press.iter().all(|(_, by_key)| by_key.is_none()), "a press reports no key arrival: {press:?}");
    assert!(press.iter().any(|(any, _)| *any == Some(PartRef::from(RIGHT))), "the press still moved focus onto the part: {press:?}");
}
