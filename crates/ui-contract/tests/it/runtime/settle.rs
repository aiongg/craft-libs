//! `runtime::settle`: each signal turns the app busy, then back to settled.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use craft_behavior::{MotionMode, PointerButton, Sense};
use craft_host::{AppInfo, Host, ManualJobs, Wake};
use craft_i18n::Han;
use craft_ui::contract::{Animate as _, Common, Component, Cx, Job, Remember as _, Role, Show as _};
use craft_ui::fonts::{FontCandidates, OnDemandFonts};
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::motion::spec;
use craft_ui::runtime::settle::{self, Looping};
use craft_ui::runtime::windows::WindowHost;
use craft_ui::style::StyleCx;
use craft_ui::testing::{Frames, trial};
use craft_ui_test_kit::contract::PlainStyle;
use egui::{Response, Ui, ViewportId};

/// What the probe asks of the motion service or the job queue each pass.
#[derive(Clone, Copy)]
enum Act {
    Still,
    Tween(f32),
    Loop,
    SteppedLoop,
    Blink,
    WakeAt(f64),
    Job,
}

#[derive(Component)]
struct Probe<'a> {
    #[common]
    common: Common,
    act: Act,
    shown: &'a Cell<f32>,
}

impl Component for Probe<'_> {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ProbeStyleKey) -> PlainStyle {
        PlainStyle::min(egui::vec2(40.0, 20.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        match self.act {
            Act::Still => {}
            Act::Tween(target) => self.shown.set(cx.anim("value", target, spec::MOVE)),
            Act::Loop => _ = cx.phase(1.0, None),
            Act::SteppedLoop => _ = cx.phase(1.0, Some(0.25)),
            Act::Blink => _ = cx.blink(0.0),
            Act::WakeAt(at) => {
                if cx.now() < at {
                    cx.wake_at(at);
                }
            }
            Act::Job => _ = cx.pending("job", || Job::new(|_, _| Ok::<u32, String>(7))),
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn show(act: Act, shown: &Cell<f32>) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        Probe { common: Common::new(), act, shown }.show(ui);
    }
}

fn state(frames: &Frames) -> settle::SettleState {
    settle::state(&frames.ctx)
}

/// The tween's target and the one-shot wake's delay.
const TARGET: f32 = 1.0;
const DELAY: f64 = 1.0;

#[trial]
fn a_tween_is_busy_until_it_reaches_its_target() {
    let (mut frames, shown) = (Frames::new(), Cell::new(0.0));
    frames.settle(show(Act::Tween(0.0), &shown));
    assert!(state(&frames).is_settled(), "at rest before the change: {:?}", state(&frames));
    frames.frame(Vec::new(), show(Act::Tween(TARGET), &shown));
    assert!(state(&frames).animating && !state(&frames).at_rest(), "a retargeted tween is busy");
    assert!(shown.get() < TARGET, "under way");
    frames.settle(show(Act::Tween(TARGET), &shown));
    assert!(state(&frames).is_settled(), "settled after the tween: {:?}", state(&frames));
    assert!(shown.get() == TARGET, "a settle ends with the tween at its target");
}

#[trial]
fn loops_are_at_rest_but_not_settled_and_gone_without_motion() {
    let shown = Cell::new(0.0);
    for (act, looping, repaints) in
        [(Act::Loop, Looping::Loop, true), (Act::SteppedLoop, Looping::Periodic, false), (Act::Blink, Looping::Periodic, false)]
    {
        let mut frames = Frames::new();
        frames.idle(3, show(act, &shown));
        let now = state(&frames);
        assert_eq!(now.looping, Some(looping));
        assert_eq!(now.repaint, repaints, "only an unstepped loop asks for every pass: {now:?}");
        assert!(now.at_rest() && !now.is_settled(), "{now:?}");
        frames.root(RootEnv::default().motion(MotionMode::Off));
        frames.settle(show(act, &shown));
        assert!(state(&frames).is_settled(), "no loop under Off: {:?}", state(&frames));
    }
}

#[trial]
fn a_one_shot_wake_is_at_rest_until_it_fires() {
    let (mut frames, shown) = (Frames::new(), Cell::new(0.0));
    frames.frame(Vec::new(), show(Act::Still, &shown));
    let at = frames.ctx.input(|input| input.time) + DELAY;
    frames.settle(show(Act::WakeAt(at), &shown));
    let now = state(&frames);
    assert!(now.wake_in.is_some_and(|wake| wake > 0.0 && wake <= DELAY), "{now:?}");
    assert!(now.at_rest() && !now.is_settled(), "{now:?}");
    frames.advance(DELAY);
    frames.settle(show(Act::WakeAt(at), &shown));
    assert!(state(&frames).is_settled(), "settled once the wake fired: {:?}", state(&frames));
}

#[trial]
fn a_rerun_the_frame_did_not_run_is_busy_until_a_frame_runs_it() {
    let mut frames = Frames::new();
    frames.idle(2, |_| {});
    frames.frame(Vec::new(), |ui| craft_ui::runtime::frame::discard_pass(ui.ctx(), "settle trial: every pass"));
    assert!(state(&frames).rerun, "the last pass asked for a rerun");
    frames.settle(|_| {});
    assert!(state(&frames).is_settled(), "{:?}", state(&frames));
}

#[trial]
fn a_background_result_is_busy_until_a_pass_shows_it() {
    let jobs = Arc::new(ManualJobs::new());
    let host = Arc::new(Host::builder(AppInfo { id: "trial", name: "Trial", version: "0" }).jobs(jobs.clone()).build());
    let (mut frames, shown) = (Frames::new(), Cell::new(0.0));
    frames.host(host);
    frames.idle(2, show(Act::Job, &shown));
    assert_eq!(state(&frames).jobs, 1, "submitted and waiting");
    jobs.run_next();
    assert_eq!(state(&frames).jobs, 1, "landed but not shown");
    frames.settle(show(Act::Job, &shown));
    assert!(state(&frames).is_settled(), "{:?}", state(&frames));
}

/// Candidates for no script, whose background lookup runs until `done`.
#[derive(Default)]
struct Lookup {
    running: AtomicBool,
    wake: Mutex<Option<Wake>>,
}

impl FontCandidates for Lookup {
    fn files(&self, _: &str, _: Option<Han>) -> Vec<std::path::PathBuf> {
        Vec::new()
    }

    fn covers(&self, _: &[u8], _: char) -> Option<u32> {
        None
    }

    fn pending(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn wake_when_ready(&self, wake: Wake) {
        *self.wake.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(wake);
    }
}

#[trial]
fn fonts_are_busy_while_a_lookup_runs() {
    let lookup = Arc::new(Lookup::default());
    lookup.running.store(true, Ordering::SeqCst);
    let mut frames = Frames::new();
    OnDemandFonts::new(lookup.clone(), None).install(&frames.ctx);
    let mut han = |ui: &mut Ui| {
        let color = ui.visuals().text_color();
        ui.painter().text(egui::pos2(4.0, 4.0), egui::Align2::LEFT_TOP, "漢", egui::FontId::default(), color);
    };
    frames.idle(3, &mut han);
    assert!(state(&frames).fonts, "waits for the lookup: {:?}", state(&frames));
    lookup.running.store(false, Ordering::SeqCst);
    let wake = lookup.wake.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
    wake.expect("the loader asked to be woken").wake();
    frames.settle(&mut han);
    assert!(state(&frames).is_settled(), "settled once the script ran out of candidates: {:?}", state(&frames));
}

#[trial]
fn a_repaint_request_is_busy_until_a_pass_ran_after_it() {
    let mut frames = Frames::new();
    frames.settle(|_| {});
    assert!(!state(&frames).repaint);
    frames.ctx.request_repaint();
    assert!(state(&frames).repaint, "asked between frames");
    frames.settle(|_| {});
    assert!(state(&frames).is_settled(), "{:?}", state(&frames));
    frames.frame(Vec::new(), |ui| ui.ctx().request_repaint());
    assert!(state(&frames).repaint, "asked inside a pass");
    frames.settle(|_| {});
    assert!(state(&frames).is_settled(), "{:?}", state(&frames));
}

#[trial]
fn injected_input_is_busy_until_its_passes_consumed_it() {
    let mut frames = Frames::new();
    frames.settle(|_| {});
    WindowHost::of(&frames.ctx).inject_click(ViewportId::ROOT, egui::pos2(10.0, 10.0), PointerButton::Primary, 1);
    assert!(state(&frames).input, "queued");
    frames.settle(|_| {});
    assert!(state(&frames).is_settled(), "{:?}", state(&frames));
}
