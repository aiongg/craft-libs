//! Coach marks.

use egui::{Context, Key, Rect, Vec2, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::tour::card_position;
use super::*;
use crate::theme::{Base, install};

struct Host {
    index: usize,
    events: Vec<TourEvent>,
}

fn steps() -> Vec<TourStep> {
    ["Layers", "Tools", "Options"]
        .into_iter()
        .map(|name| TourStep { id: name.to_lowercase(), title: name.into(), body: format!("About {name}."), target: None })
        .collect()
}

fn host(index: usize) -> Harness<'static, Host> {
    let app = |ui: &mut egui::Ui, host: &mut Host| {
        if let Some(event) = Tour::new("tour").show(ui.ctx(), &steps(), host.index) {
            host.events.push(event);
        }
    };
    let mut h = Harness::<Host>::builder()
        .with_size([800.0, 600.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(app, Host { index, events: vec![] });
    h.run();
    h
}

#[test]
fn tour_buttons_and_keys_move_steps() {
    let mut h = host(1);
    h.get_by_label("Next").click();
    h.run();
    assert_eq!(h.state().events, [TourEvent::Next]);

    let mut h = host(1);
    h.get_by_label("Back").click();
    h.run();
    assert_eq!(h.state().events, [TourEvent::Back]);

    let mut h = host(1);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().events, [TourEvent::Next], "Enter is Next");

    let mut h = host(2);
    assert!(h.query_by_label("Next").is_none(), "the last step says Done");
    h.get_by_label("Done").click();
    h.run();
    assert_eq!(h.state().events, [TourEvent::Finished]);

    let mut h = host(0);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().events, [TourEvent::Skipped]);

    // The close button and the Skip Tour button both skip.
    for which in 0..2 {
        let mut h = host(0);
        let button = h.get_all_by_role_and_label(egui::accesskit::Role::Button, "Skip Tour").nth(which).expect("two buttons");
        button.click();
        h.run();
        assert_eq!(h.state().events, [TourEvent::Skipped], "button {which}");
    }
}

#[test]
fn card_sits_beside_the_target_or_centred() {
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
    let card = Vec2::new(340.0, 180.0);
    let target = |min: (f32, f32), max: (f32, f32)| Some(Rect::from_min_max(pos2(min.0, min.1), pos2(max.0, max.1)));
    let at = |t| card_position(t, card, screen, 8.0);

    let right = at(target((100.0, 200.0), (200.0, 240.0)));
    assert!(right.x > 200.0 && (right.y - 130.0).abs() < 0.5, "right of the target, centred on it: {right:?}");

    let left = at(target((650.0, 200.0), (780.0, 240.0)));
    assert!(left.x + card.x < 650.0, "left when the right has no room: {left:?}");

    let below = at(target((10.0, 20.0), (790.0, 60.0)));
    assert!(below.y > 60.0, "below when neither side has room: {below:?}");

    let above = at(target((10.0, 540.0), (790.0, 580.0)));
    assert!(above.y + card.y < 540.0, "above when below has no room: {above:?}");

    let covering = at(target((10.0, 10.0), (790.0, 590.0)));
    assert_eq!(covering, pos2(800.0 - 8.0 - card.x, 600.0 - 8.0 - card.y), "bottom right when nothing fits");

    assert_eq!(at(None), pos2(230.0, 210.0), "centred without a target");
}
