//! `Remember::pending`: one submission per key, a result that lands on the next pass, one repaint.

use crate::contract::kit::PlainStyle;
use craft_behavior::Sense;
use craft_host::{AppInfo, Host, ManualJobs};
use craft_ui::contract::{Common, Component, Cx, Job, Pending, Remember as _, Role, Show as _};
use craft_ui::style::StyleCx;
use craft_ui::testing::{Frames, trial};
use egui::Response;
use std::cell::RefCell;
use std::sync::Arc;

/// Shows the pending result of a job that reports half done, then returns 7.
#[derive(Component)]
struct Thumbnail<'a> {
    #[common]
    common: Common,
    seen: &'a RefCell<Vec<Pending<u32, String>>>,
}

impl Component for Thumbnail<'_> {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ThumbnailStyleKey) -> PlainStyle {
        PlainStyle::min(egui::vec2(40.0, 20.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let result = cx.pending("thumb", || {
            Job::new("thumbnail", |progress, _| {
                progress.report(0.5);
                Ok::<u32, String>(7)
            })
        });
        self.seen.borrow_mut().push(result.clone());
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_pending_result_arrives_on_the_next_pass_and_repaints_once() {
    let jobs = Arc::new(ManualJobs::new());
    let host = Arc::new(Host::builder(AppInfo { id: "trial", name: "Trial", version: "0" }).jobs(jobs.clone()).build());
    let seen = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    frames.host(host);
    let mut show = |ui: &mut egui::Ui| {
        Thumbnail { common: Common::new(), seen: &seen }.show(ui);
    };
    frames.idle(3, &mut show);
    assert_eq!(jobs.waiting(), 1, "submitted once across frames");
    assert!(seen.borrow().iter().all(|p| matches!(p, Pending::Loading { progress: None, .. })), "loading until the job runs");
    assert!(!frames.repaint_requested(), "a loading result asks for nothing");
    jobs.run_next();
    assert!(frames.repaint_requested(), "the landed result asks for a pass");
    seen.borrow_mut().clear();
    frames.frame(Vec::new(), &mut show);
    assert_eq!(seen.borrow().last(), Some(&Pending::Ready(7)), "the next pass shows the result");
    frames.frame(Vec::new(), &mut show);
    assert!(!frames.repaint_requested(), "after egui's own follow-up pass nothing asks for more");
    frames.idle(2, &mut show);
    assert_eq!(jobs.waiting(), 0, "a landed result is not submitted again");
    assert_eq!(seen.borrow().last(), Some(&Pending::Ready(7)), "it stays");
}
