//! Notice tests: live region per tone, plain notes, the icon and the story.

use egui::Context;
use egui::accesskit::{Live, Role};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::testing::a11y;
use crate::theme::{Base, install};

fn harness<'a>(dir: Direction, show: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a, ()> {
    let builder: HarnessBuilder<()> = Harness::builder().with_size([400.0, 300.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    });
    builder.build_ui(show)
}

fn region<'a, S>(harness: &'a Harness<'_, S>) -> egui_kittest::Node<'a> {
    harness.get_by(|n| matches!(n.role(), Role::Status | Role::Alert | Role::Note))
}

#[test]
fn the_region_is_status_or_alert_with_its_live_setting() {
    for (tone, role, live) in [
        (Tone::Info, Role::Status, Live::Polite),
        (Tone::Good, Role::Status, Live::Polite),
        (Tone::Warn, Role::Status, Live::Polite),
        (Tone::Neutral, Role::Status, Live::Polite),
        (Tone::Bad, Role::Alert, Live::Assertive),
    ] {
        let mut harness = harness(Direction::Ltr, move |ui| {
            Notice::new("The message").title("The title").tone(tone).show(ui);
        });
        harness.run();
        let node = region(&harness);
        assert_eq!((node.accesskit_node().role(), node.accesskit_node().live()), (role, live), "{tone:?}");
        assert_eq!(node.accesskit_node().label().as_deref(), Some("The title, The message"), "{tone:?}");
        let text = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("The message"));
        assert_eq!(text.accesskit_node().live(), Live::Off, "the text inside is silent");
    }
}

#[test]
fn announce_off_is_a_plain_note() {
    let mut harness = harness(Direction::Ltr, |ui| {
        Notice::new("Applies at the next launch.").tone(Tone::Warn).announce(false).show(ui);
    });
    harness.run();
    let node = region(&harness);
    assert_eq!(node.accesskit_node().role(), Role::Note);
    assert_eq!(node.accesskit_node().live(), Live::Off);
}

#[test]
fn the_icon_has_no_node() {
    let nodes = |tone| {
        let mut harness = harness(Direction::Ltr, move |ui| {
            Notice::new("Message").tone(tone).show(ui);
        });
        harness.run();
        harness.root().children_recursive().count()
    };
    assert_eq!(nodes(Tone::Warn), nodes(Tone::Neutral));
}

#[test]
fn the_text_starts_after_the_icon_and_neutral_starts_at_the_edge() {
    let start = |tone, dir| {
        let mut harness = harness(dir, move |ui| {
            Notice::new("Message").tone(tone).show(ui);
        });
        harness.run();
        let rect = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Message")).rect();
        (rect.left(), rect.right())
    };
    let (neutral, _) = start(Tone::Neutral, Direction::Ltr);
    let (warn, _) = start(Tone::Warn, Direction::Ltr);
    assert!(warn > neutral + 10.0, "{warn} {neutral}");
    let (_, rtl_warn) = start(Tone::Warn, Direction::Rtl);
    let (_, rtl_neutral) = start(Tone::Neutral, Direction::Rtl);
    assert!(rtl_warn < rtl_neutral - 10.0, "{rtl_warn} {rtl_neutral}");
}

#[test]
fn a_widget_appearing_before_a_notice_keeps_its_region() {
    let mut harness =
        Harness::builder().with_size([400.0, 300.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, shown: &mut bool| {
                if *shown {
                    ui.label("Appears");
                }
                Notice::new("Saved").tone(Tone::Good).id_salt("saved").show(ui);
            },
            false,
        );
    harness.run();
    let id = region(&harness).accesskit_node().id();
    *harness.state_mut() = true;
    harness.run();
    assert!(harness.query_by_label("Appears").is_some());
    assert_eq!(region(&harness).accesskit_node().id(), id);
}

#[test]
fn the_story_is_labelled_in_both_directions() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut args = (notice::stories()[0].args)();
        let mut harness = harness(dir, move |ui| notice::show_tones(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
    }
}

/// The colour of each text shape a notice paints, in order.
fn text_colors(title: Option<&str>) -> Vec<egui::Color32> {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut colors = Vec::new();
    let mut output = ctx.run_ui(
        egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0))), ..Default::default() },
        |ui| {
            let mut notice = Notice::new("The message").tone(Tone::Warn);
            if let Some(title) = title {
                notice = notice.title(title);
            }
            notice.show(ui);
        },
    );
    output.textures_delta.clear();
    for clipped in output.shapes {
        if let egui::Shape::Text(t) = clipped.shape {
            colors.push(t.galley.job.sections[0].format.color);
        }
    }
    colors
}

#[test]
fn a_message_under_a_title_is_muted_and_a_lone_message_has_the_tone_colour() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let style = Notice::style(&crate::theme::current(&ctx), Tone::Warn);
    assert_ne!(style.fg, style.body);
    assert_eq!(text_colors(Some("The title")), [style.fg, style.body]);
    assert_eq!(text_colors(None), [style.fg]);
}
