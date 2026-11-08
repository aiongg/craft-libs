//! Document faces: one registry per context, a budget of new faces per frame, eviction past the cap, and registering
//! again after the definitions are replaced.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use craft_behavior::Metrics;
use craft_ui::contract::{Common, Component, Cx, FacesCx as _, MeasureCx, Props as _, Role, Show as _};
use craft_ui::fonts::{self, DOCUMENT_FACES_PER_PASS, FaceBytes, FaceState, MAX_DOCUMENT_FACES};
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::{Frames, trial};
use egui::{Response, Vec2};

const FONT: &[u8] = include_bytes!("../../fonts/NotoSans-test.ttf");

/// The frames a face may take to become ready.
const FRAMES_TO_READY: usize = 20;

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PlainStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What the probes asked for: how often a face was loaded and the state each key was last given.
#[derive(Clone, Default)]
struct Tally {
    loads: Arc<AtomicUsize>,
    states: Arc<Mutex<HashMap<String, FaceState>>>,
}

impl Tally {
    fn loads(&self) -> usize {
        self.loads.load(Ordering::Relaxed)
    }

    fn state(&self, key: &str) -> FaceState {
        self.states.lock().expect("the states").get(key).cloned().unwrap_or_else(|| panic!("{key} was asked for"))
    }
}

/// An owner of document faces: it asks for its keys while it is measured.
#[derive(Component)]
struct Owner {
    #[common]
    common: Common,
    keys: Vec<String>,
    loads_fail: bool,
    tally: Tally,
}

impl Component for Owner {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: OwnerStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        for key in &self.keys {
            let state = cx.document_face(key, || {
                self.tally.loads.fetch_add(1, Ordering::Relaxed);
                (!self.loads_fail).then(|| FaceBytes::new(Arc::from(FONT), 0))
            });
            self.tally.states.lock().expect("the states").insert(key.clone(), state);
        }
        Vec2::splat(1.0)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn keys(range: std::ops::Range<usize>) -> Vec<String> {
    range.map(|n| format!("k{n}")).collect()
}

/// One frame in which each `(salt, keys)` owner asks for its keys.
fn frame(frames: &mut Frames, tally: &Tally, owners: &[(&'static str, Vec<String>)], loads_fail: bool) {
    frames.frame(vec![], |ui| {
        for (salt, keys) in owners {
            Owner { common: Common::new(), keys: keys.clone(), loads_fail, tally: tally.clone() }.id_salt(*salt).show(ui);
        }
    });
}

/// Frames until every key of `owners` is ready.
fn until_ready(frames: &mut Frames, tally: &Tally, owners: &[(&'static str, Vec<String>)]) {
    for _ in 0..FRAMES_TO_READY {
        frame(frames, tally, owners, false);
        if owners.iter().flat_map(|(_, keys)| keys).all(|key| matches!(tally.state(key), FaceState::Ready(_))) {
            return;
        }
    }
    panic!("the faces did not become ready in {FRAMES_TO_READY} frames");
}

/// Past the cap only the faces the previous frame did not draw are taken out; a screen of more faces than the cap keeps
/// them all.
#[trial]
fn only_faces_off_screen_are_taken_out() {
    let (mut frames, tally) = (Frames::new(), Tally::default());
    let screen = |range| [("owner", keys(range))];
    until_ready(&mut frames, &tally, &screen(0..MAX_DOCUMENT_FACES));
    until_ready(&mut frames, &tally, &screen(0..MAX_DOCUMENT_FACES + 10));
    assert_eq!(tally.loads(), MAX_DOCUMENT_FACES + 10, "a screen over the cap keeps every face");

    // Scrolled on: the ten new faces load, and the twenty the screen left are taken out.
    until_ready(&mut frames, &tally, &screen(20..MAX_DOCUMENT_FACES + 20));
    assert_eq!(tally.loads(), MAX_DOCUMENT_FACES + 20);

    frame(&mut frames, &tally, &[("owner", vec!["k30".to_owned(), "k5".to_owned()])], false);
    assert!(matches!(tally.state("k30"), FaceState::Ready(_)), "a face still on screen stays");
    assert_eq!(tally.state("k5"), FaceState::Loading, "a face that left the screen loads again");
    assert_eq!(tally.loads(), MAX_DOCUMENT_FACES + 21);
}

/// Owners share one registry and one budget: a frame adds at most the budget's faces however many owners ask, and a key
/// two owners ask for loads once.
#[trial]
fn one_registry_serves_every_owner_and_a_frame_begins_once() {
    let (mut frames, tally) = (Frames::new(), Tally::default());
    let owners = [("a", keys(0..4)), ("b", keys(2..8))];
    frame(&mut frames, &tally, &owners, false);
    assert_eq!(tally.loads(), DOCUMENT_FACES_PER_PASS, "the budget counts both owners");
    until_ready(&mut frames, &tally, &owners);
    assert_eq!(tally.loads(), 8, "the keys both owners ask for load once");
}

#[trial]
fn a_new_face_past_the_frames_budget_waits_without_loading() {
    let (mut frames, tally) = (Frames::new(), Tally::default());
    let owners = [("owner", keys(0..DOCUMENT_FACES_PER_PASS + 1))];
    frame(&mut frames, &tally, &owners, false);
    let last = format!("k{DOCUMENT_FACES_PER_PASS}");
    assert_eq!(tally.state(&last), FaceState::Loading);
    assert_eq!(tally.loads(), DOCUMENT_FACES_PER_PASS, "`load` is not called past the budget");
    assert!(frames.repaint_requested(), "a face that waits asks for another pass");
    frame(&mut frames, &tally, &owners, false);
    assert_eq!(tally.loads(), DOCUMENT_FACES_PER_PASS + 1, "the next frame has a budget again");
}

#[trial]
fn a_face_that_cannot_load_is_unavailable_and_not_tried_again() {
    let (mut frames, tally) = (Frames::new(), Tally::default());
    let owners = [("owner", keys(0..1))];
    frame(&mut frames, &tally, &owners, true);
    frame(&mut frames, &tally, &owners, false);
    assert_eq!(tally.state("k0"), FaceState::Unavailable);
    assert_eq!(tally.loads(), 1);
}

/// A stack install sets the whole definitions and drops the faces added one by one: the registry notices and adds
/// them again, within the budget.
#[trial]
fn faces_are_registered_again_after_the_definitions_are_replaced() {
    let (mut frames, tally) = (Frames::new(), Tally::default());
    let owners = [("owner", keys(0..3))];
    until_ready(&mut frames, &tally, &owners);
    assert_eq!(tally.loads(), 3);

    fonts::load(&frames.ctx, craft_ui::fonts::deflated_default_definitions());
    frame(&mut frames, &tally, &owners, false);
    assert!(keys(0..3).iter().all(|key| tally.state(key) == FaceState::Loading), "the replaced faces wait for their registration");
    assert_eq!(tally.loads(), 6, "each face loads again");
    until_ready(&mut frames, &tally, &owners);
    assert_eq!(tally.loads(), 6);
}
