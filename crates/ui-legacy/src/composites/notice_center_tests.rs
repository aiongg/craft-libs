//! The notice center.

use egui::accesskit::Role;
use egui::{Context, Rect, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

const BODY: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(640.0, 400.0) };

struct App {
    center: NoticeCenter,
    ends: Vec<NoticeEnd>,
}

fn harness(center: NoticeCenter) -> Harness<'static, App> {
    Harness::builder()
        .with_size(BODY.size())
        .with_step_dt(0.1)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app| {
                let ends = app.center.show(ui.ctx(), BODY);
                app.ends.extend(ends);
            },
            App { center, ends: Vec::new() },
        )
}

#[test]
// A fourth notice drops the oldest one without a dismiss key and reports NoticeEnd::Dropped.
fn posting_past_the_cap_drops_the_oldest_unsilenceable() {
    let mut center = NoticeCenter::new();
    let keyed = center.post("Keyed", vec![], Tone::Neutral, Some("shortcuts".into()), &[]).id.unwrap();
    let plain = center.post("Plain", vec![], Tone::Neutral, None, &[]).id.unwrap();
    center.post("Third", vec![], Tone::Neutral, None, &[]).id.unwrap();
    let fourth = center.post("Fourth", vec![], Tone::Neutral, None, &[]);
    assert_eq!(fourth.dropped, [NoticeEnd::Dropped(plain)], "post reports the drop in its own pass");
    let ids: Vec<u64> = center.notices().iter().map(|n| n.id).collect();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&keyed) && !ids.contains(&plain), "the keyed notice stays, the oldest plain one goes");
    let mut harness = harness(center);
    harness.step();
    assert!(harness.state().ends.is_empty(), "show does not report it again: {:?}", harness.state().ends);
}

#[test]
// post returns None when the dismiss key is in silenced; Don't Show Again reports Silenced with the key.
fn silenced_notices_are_not_posted() {
    let mut center = NoticeCenter::new().auto_hide(None);
    let silenced = ["shortcuts".to_owned()];
    assert_eq!(center.post("Hidden", vec![], Tone::Neutral, Some("shortcuts".into()), &silenced).id, None);
    let id = center.post("Shown", vec!["A line".into()], Tone::Neutral, Some("other".into()), &silenced).id.unwrap();
    let mut harness = harness(center);
    harness.run();
    harness.get_by_role_and_label(Role::Link, "Don't Show Again").click();
    harness.run();
    assert_eq!(harness.state().ends, [NoticeEnd::Silenced { id, key: "other".into() }]);
    assert!(harness.state().center.notices().is_empty());
}
