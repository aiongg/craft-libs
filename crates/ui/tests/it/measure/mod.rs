//! A measuring run delivers nothing and consumes nothing: what the real run after it needs is still there.

mod app_window;
mod record;

use crate::body::kit::Slab;
use craft_theme::Srgba;
use craft_ui::contract::{Bind, ContainerProps, Politeness, Props as _, Scroll, Show as _, ShowContainer as _};
use craft_ui::patterns::form_dialogs::{FormDialog, HostRequests};
use craft_ui::runtime::commands::{FireSource, fire};
use craft_ui::runtime::toast::{self, Toast};
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, measuring_run, take_effects, trial};
use egui::{Id, Pos2, Rect, Ui, UiBuilder, Vec2};
use std::cell::{Cell, RefCell};

const LINE: Vec2 = Vec2::new(100.0, 20.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const LINES: usize = 40;

/// A scrolling panel of `LINES` lines; `first_top` is where the first line sits, `id` the panel's.
fn panel(ui: &mut Ui, first_top: &Cell<f32>, id: &Cell<Option<Id>>) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let props = ContainerProps::new().scroll(Scroll::vertical());
        let out = Slab::new(props).id_salt("panel").show(ui, |ui, _| {
            for line in 0..LINES {
                if line == 0 {
                    first_top.set(ui.next_widget_position().y);
                }
                Block::new(LINE).show(ui);
            }
        });
        id.set(Some(out.output.id));
    });
}

/// A reveal queued for a scrolling container is still queued after a measuring run of that container.
#[trial]
fn a_measuring_run_leaves_a_reveal_for_the_real_run() {
    let (top, id) = (Cell::new(0.0), Cell::new(None));
    let mut frames = Frames::new();
    let far = Rect::from_min_size(Pos2::new(ORIGIN.x, ORIGIN.y + 20.0 * 30.0), LINE);
    let measured = |ui: &mut Ui, queue: bool| {
        let ctx = ui.ctx().clone();
        ui.scope_builder(UiBuilder::new().sizing_pass(), |ui| {
            if queue && let Some(owner) = id.get() {
                craft_ui::testing::contract::queue_reveal(&ctx, owner, far);
            }
            measuring_run(&ctx, || panel(ui, &top, &id));
        });
        id.get().and_then(|owner| craft_ui::testing::contract::take_reveal(&ctx, owner))
    };
    frames.idle(3, |ui| {
        measured(ui, false);
    });
    let mut left = None;
    frames.frame(vec![], |ui| left = measured(ui, true));
    assert_eq!(left, Some(far), "the reveal is still queued for the real run");
}

#[trial]
fn a_measuring_run_fires_and_announces_nothing() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        let ctx = ui.ctx().clone();
        measuring_run(&ctx, || {
            assert!(!fire(&ctx, "save", FireSource::Palette));
            craft_ui::contract::announce(&ctx, "Saved", Politeness::Polite);
            let id = toast::post(&ctx, Toast::new("Saved"));
            toast::dismiss(&ctx, id);
        });
        assert_eq!(take_effects(&ctx), ["commands::fire", "announce", "toast::post"]);
        assert!(toast::cards(&ctx).is_empty(), "no toast was posted");
    });
    assert!(craft_ui::runtime::frame::take_output(&frames.ctx).announcements.is_empty(), "nothing was announced");
}

/// A body that runs in a measuring run sees no answer and no opening pass: the real run after it still has both.
#[trial]
fn form_answers_reach_the_real_run() {
    let mut requests = HostRequests::new();
    requests.answer_color("tint", Srgba::rgb(0.0, 0.0, 1.0));
    let answers = requests.take_answers();
    let seen = RefCell::new(Vec::<(bool, usize, bool)>::new());
    let mut held = 0u8;
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        seen.borrow_mut().clear();
        let ctx = ui.ctx().clone();
        for measuring in [true, false] {
            let mut show = |ui: &mut Ui| {
                // The dialog's real show measures its body first, in a sizing pass of its own.
                FormDialog::new("Form", Bind::bound(&mut held)).answers(answers.clone()).show(ui, |ui, form, _| {
                    let first = form.first();
                    seen.borrow_mut().push((measuring || ui.is_sizing_pass(), form.take_answers().len(), first));
                    true
                });
            };
            if measuring { measuring_run(&ctx, || show(ui)) } else { show(ui) }
        }
    });
    let seen = seen.borrow();
    assert!(seen.contains(&(true, 0, false)), "the measuring run saw no answer and no opening pass: {seen:?}");
    assert!(seen.iter().filter(|run| !run.0).all(|run| run.1 == 1 && run.2), "the real run saw the answer and the opening pass: {seen:?}");
    assert!(seen.iter().any(|run| !run.0), "the real run ran: {seen:?}");
}
