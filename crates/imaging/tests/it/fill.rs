//! Content-Aware Fill: what its own logic adds to the full-window parts, the tools and the controls.

use std::cell::RefCell;
use std::rc::Rc;

use egui::{Color32, ColorImage, Key, Modifiers, Pos2, vec2};
use serde_json::json;

use craft_behavior::edit_history::Step;
use craft_behavior::geometry::selection_op::SelectionOperation;
use craft_behavior::kurbo::Rect;
use craft_behavior::{CancelReason, EditHistory, PixelSize};
use craft_imaging::ui::fill::{
    ContentAwareFillWorkspace, FillAreaEdit, FillCommand, FillEvent, FillLimit, FillLimits, FillOpen, FillOutput, FillOutputTo, FillOverlay,
    FillPreviewPane, FillPreviewRequest, FillReset, FillSession, FillSettings, FillSource, FillStrokeEvent, PreviewFraming, RotationAdaptation,
    SamplingMode, SamplingPlane, SamplingStroke,
};
use craft_imaging::ui::select_mask::EditMode;
use craft_imaging::ui::wide_angle::PreviewDone;
use craft_theme::Srgba;
use craft_ui::canvas::{NavigationOptions, PixelRect, RasterSource};
use craft_ui::dialog::FullWindowView;
use craft_ui::primitives::Revision;
use craft_ui::runtime::access::Role;
use craft_ui::runtime::pending::{Job, Pending};
use craft_ui::testing::{Fixture, pointer, trial};

const NONE: Modifiers = Modifiers::NONE;
const GENEROUS: FillLimits = FillLimits::new(1000, 200_000);
/// The layer the workspace opens on, in document pixels.
const LAYER: PixelSize = PixelSize::new(400, 300);
/// The room the workspace is shown in.
const ROOM: egui::Vec2 = vec2(1400.0, 900.0);
/// How far from the window's centre the framed view's centre may be, in document pixels.
const CENTRE_TOLERANCE: f64 = 1.0;

/// The level of the flat grey picture.
const GREY: u8 = 128;

/// A flat grey picture.
struct Grey;

impl RasterSource for Grey {
    fn size(&self) -> PixelSize {
        LAYER
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_gray(GREY));
    }
}

/// The app's side: a picture and no planes; its preview job fails.
struct Source;

impl FillSource for Source {
    fn image(&self) -> &dyn RasterSource {
        &Grey
    }

    fn sampling(&self) -> Option<&SamplingPlane> {
        None
    }

    fn fill_area(&self) -> Option<&SamplingPlane> {
        None
    }

    fn sampling_window(&self) -> Option<Rect> {
        None
    }

    fn preview(&self) -> Option<&dyn RasterSource> {
        None
    }

    fn preview_job(&self, _: &FillPreviewRequest) -> Job<PreviewDone, String> {
        Job::new(|_, _| Err("no engine in this test".to_owned()))
    }
}

/// The workspace shown in a fixture, with every pass's output kept.
struct Rig {
    fixture: Fixture,
    work: Rc<RefCell<ContentAwareFillWorkspace>>,
    outputs: Rc<RefCell<Vec<FillOutput>>>,
}

impl Rig {
    fn new(settings: FillSettings, limits: FillLimits) -> Rig {
        let open = FillOpen::new([LAYER.width, LAYER.height], "Layer 1", limits).settings(settings);
        let work = Rc::new(RefCell::new(ContentAwareFillWorkspace::new(open)));
        let outputs = Rc::new(RefCell::new(Vec::new()));
        let (shown, kept) = (Rc::clone(&work), Rc::clone(&outputs));
        let mut fixture = Fixture::new(move |ui| {
            let output = shown.borrow_mut().show(ui, &Source);
            kept.borrow_mut().push(output);
        });
        fixture.room(ROOM).settle();
        let rig = Rig { fixture, work, outputs };
        rig.take();
        rig
    }

    fn standard() -> Rig {
        Rig::new(FillSettings::default(), GENEROUS)
    }

    /// The middle of the main canvas.
    fn canvas(&self) -> Pos2 {
        self.fixture.by_role(Role::CANVAS).into_iter().next().expect("the workspace shows a canvas").center()
    }

    /// The outputs since the last call.
    fn take(&self) -> Vec<FillOutput> {
        std::mem::take(&mut *self.outputs.borrow_mut())
    }

    /// The events since the last call.
    fn events(&self) -> Vec<FillEvent> {
        self.take().into_iter().flat_map(|output| output.events).collect()
    }

    /// Presses at `from`, moves through `through` and releases at the last point, with `modifiers` held.
    fn stroke(&mut self, from: Pos2, through: &[Pos2], modifiers: Modifiers) {
        self.fixture.hold(modifiers);
        self.fixture.frame(vec![pointer::moved(from)]);
        self.fixture.frame(vec![pointer::press(from)]);
        for at in through {
            self.fixture.frame(vec![pointer::moved(*at)]);
        }
        self.fixture.frame(vec![pointer::release(through.last().copied().unwrap_or(from))]);
        self.fixture.hold(NONE);
        self.fixture.idle(1);
    }

    /// A stroke along a line to the right of the canvas centre in `steps` moves.
    fn line(&mut self, steps: usize) {
        let through: Vec<Pos2> = (1..=steps).map(|i| self.canvas() + vec2(12.0 * i as f32, 0.0)).collect();
        self.stroke(self.canvas(), &through, NONE);
    }

    /// Picks the Lasso in the tool strip.
    fn lasso(&mut self) {
        let tool = self.fixture.by_label("Lasso Tool").expect("the strip has the Lasso");
        self.fixture.click(&tool);
    }

    /// A lasso round a square.
    fn square(&mut self, modifiers: Modifiers) {
        let corners =
            [self.canvas() + vec2(80.0, 0.0), self.canvas() + vec2(80.0, 80.0), self.canvas() + vec2(0.0, 80.0), self.canvas() + vec2(0.0, 1.0)];
        self.stroke(self.canvas(), &corners, modifiers);
    }
}

/// The stroke a pass committed, if one did.
fn committed(events: &[FillEvent]) -> Option<&SamplingStroke> {
    events.iter().find_map(|event| match event {
        FillEvent::Stroke(FillStrokeEvent::Commit(stroke)) => Some(stroke),
        _ => None,
    })
}

fn stroke_of(points: &[[f64; 2]]) -> SamplingStroke {
    SamplingStroke::new(EditMode::Add, 20.0, points.to_vec())
}

fn session_with_work() -> FillSession {
    let mut session = FillSession::default();
    session.set_sampling(SamplingMode::Custom);
    session.add_stroke(stroke_of(&[[1.0, 1.0], [2.0, 2.0]]));
    assert_eq!(session.add_fill_edit(FillAreaEdit::grow(true, 3.0), &GENEROUS), Ok(()));
    session
}

#[trial]
fn fill_reset_scopes_sampling_fill_and_all() {
    let mut changed = FillSettings::default();
    (changed.sample_all_layers, changed.opacity, changed.scale, changed.mirror) = (true, 10.0, true, true);

    // Sampling: the strokes and the option; the fill edits stay.
    let (mut session, mut settings) = (session_with_work(), changed.clone());
    settings.sampling = SamplingMode::Custom;
    assert!(session.reset(FillReset::Sampling, &mut settings));
    assert_eq!((session.sampling, session.strokes.len(), session.fill_edits.len()), (SamplingMode::Auto, 0, 1));
    assert!(!settings.sample_all_layers && settings.sampling == SamplingMode::Auto);
    assert!(settings.scale, "the fill settings stay");

    // Fill: only the settings, and the session reports no change.
    let (mut session, mut settings) = (session_with_work(), changed.clone());
    assert!(!session.reset(FillReset::Fill, &mut settings));
    assert_eq!(session, session_with_work());
    assert!(!settings.scale && !settings.mirror && settings.sample_all_layers && (settings.opacity - 10.0).abs() < f64::EPSILON);

    // Fill area: only the lasso edits.
    let (mut session, mut settings) = (session_with_work(), changed.clone());
    assert!(session.reset(FillReset::FillArea, &mut settings));
    assert_eq!((session.strokes.len(), session.fill_edits.len()), (1, 0));
    assert!(!session.reset(FillReset::FillArea, &mut settings), "nothing left to reset");

    // All: the session and every setting; one undo step.
    let (mut session, mut settings) = (session_with_work(), changed);
    let mut history = EditHistory::new(&session);
    let before = session.clone();
    assert!(session.reset(FillReset::All, &mut settings));
    history.record(&craft_behavior::EditEvent::set(craft_behavior::Mixed::Same(before), session.clone(), "Reset".into(), None));
    assert_eq!(session, FillSession::default());
    assert_eq!(settings, FillSettings::default());
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(session, session_with_work());

    // A new option with the same value and no strokes changes nothing.
    let mut session = FillSession::default();
    assert!(!session.set_sampling(SamplingMode::Auto));
    assert!(session.set_sampling(SamplingMode::Rectangular));
}

#[trial]
fn fill_limits_refuse_at_the_brush() {
    let limits = FillLimits::new(2, 5);
    let mut session = FillSession::default();
    session.add_stroke(stroke_of(&[[0.0, 0.0]]));
    assert_eq!(session.may_begin_stroke(&limits), Ok(()));
    session.add_stroke(stroke_of(&[[0.0, 0.0], [1.0, 1.0]]));
    assert_eq!(session.may_begin_stroke(&limits), Err(FillLimit::Strokes(2)), "two strokes are the limit");

    // Points are counted over strokes and lasso edits; a total equal to the limit passes an edit.
    let limits = FillLimits::new(10, 5);
    let mut session = FillSession::default();
    session.add_stroke(stroke_of(&[[0.0, 0.0], [1.0, 1.0]]));
    let triangle = || {
        let Some(edit) = FillAreaEdit::lasso(SelectionOperation::Add, vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]]) else {
            panic!("three points are a lasso")
        };
        edit
    };
    assert_eq!(session.add_fill_edit(triangle(), &limits), Ok(()));
    assert_eq!(session.points_used(None), 5);
    assert_eq!(session.may_begin_stroke(&limits), Err(FillLimit::Vertices(5)));
    assert_eq!(session.add_fill_edit(triangle(), &limits), Err(FillLimit::Vertices(5)));
    assert_eq!(session.fill_edits.len(), 1, "a refused edit records nothing");

    // The brush: no stroke begins at the stroke limit.
    let mut rig = Rig::new(FillSettings::default(), FillLimits::new(1, 100));
    rig.line(2);
    assert!(committed(&rig.events()).is_some(), "the first stroke fits");
    rig.line(2);
    let events = rig.events();
    assert!(events.contains(&FillEvent::Refused(FillLimit::Strokes(1))), "{events:?}");
    assert!(committed(&events).is_none());

    // A stroke at the points limit stops taking points; the message comes once.
    let mut rig = Rig::new(FillSettings::default(), FillLimits::new(10, 3));
    rig.line(6);
    let events = rig.events();
    let refusals = events.iter().filter(|event| matches!(event, FillEvent::Refused(FillLimit::Vertices(3)))).count();
    assert_eq!(refusals, 1, "{events:?}");
    assert_eq!(committed(&events).map(|stroke| stroke.points.len()), Some(3));
}

#[trial]
fn fill_settings_and_command_serialise_as_photocraft() {
    let defaults = FillSettings::default();
    assert_eq!(serde_json::from_value::<FillSettings>(json!({})).ok(), Some(defaults.clone()));
    let Ok(round) = serde_json::to_value(&defaults) else { panic!("settings serialise") };
    assert_eq!(serde_json::from_value::<FillSettings>(round).ok(), Some(defaults.clone()));
    let Ok(partial) = serde_json::from_value::<FillSettings>(
        json!({"opacity": 30.0, "colorAdaptation": "veryHigh", "rotation": "full", "output": "duplicate", "color": [1, 2, 3]}),
    ) else {
        panic!("a partial object reads")
    };
    assert_eq!((partial.opacity, partial.color, partial.show_overlay), (30.0, [1, 2, 3], true));
    assert_eq!(partial.output, FillOutputTo::DuplicateLayer);

    // No strokes: no `samplingStrokes` key; the layer is the app's.
    let mut session = FillSession::default();
    assert_eq!(
        serde_json::to_value(FillCommand::new(&session, &defaults)).ok(),
        Some(json!({
            "sampling": "auto", "sampleAllLayers": false, "colorAdaptation": "default", "rotationAdaptation": "none",
            "scale": false, "mirror": false, "output": "new", "fillEdits": [],
        }))
    );
    session.set_sampling(SamplingMode::Custom);
    session.add_stroke(SamplingStroke::new(EditMode::Subtract, 12.0, vec![[1.0, 2.0]]));
    assert_eq!(session.add_fill_edit(FillAreaEdit::grow(false, 5.0), &GENEROUS), Ok(()));
    let mut settings = defaults;
    (settings.sample_all_layers, settings.scale, settings.rotation) = (true, true, RotationAdaptation::Medium);
    assert_eq!(
        serde_json::to_value(FillCommand::new(&session, &settings)).ok(),
        Some(json!({
            "sampling": "custom", "sampleAllLayers": true, "colorAdaptation": "default", "rotationAdaptation": "medium",
            "scale": true, "mirror": false, "output": "new", "fillEdits": [{"op": "contract", "by": 5.0}],
            "samplingStrokes": [{"mode": "subtract", "size": 12.0, "points": [[1.0, 2.0]]}],
        }))
    );
}

#[trial]
fn fill_rebase_clears_history_and_a_refused_edit_is_taken_back() {
    let mut rig = Rig::standard();
    rig.line(3);
    rig.take();
    rig.work.borrow_mut().rebase();
    assert!(!rig.work.borrow_mut().step(Step::Undo), "the history is cleared");
    assert_eq!(rig.work.borrow().session().strokes.len(), 1, "the strokes stay");

    // A lasso edit the app refuses is taken back, and nothing is left to redo of it.
    rig.lasso();
    rig.square(NONE);
    rig.take();
    assert_eq!(rig.work.borrow().session().fill_edits.len(), 1);
    rig.work.borrow_mut().refuse_fill_edit("the fill area is empty".to_owned());
    assert!(rig.work.borrow().session().fill_edits.is_empty());
    assert_eq!(rig.work.borrow().session().strokes.len(), 1);
    assert!(!rig.work.borrow_mut().step(Step::Redo), "the refused edit leaves no redo step");
}

#[trial]
fn fill_stroke_updates_carry_only_the_new_points() {
    let mut rig = Rig::standard();
    rig.line(3);
    let events = rig.events();
    let updates: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            FillEvent::Stroke(FillStrokeEvent::Update(added)) => Some(added.points.len()),
            _ => None,
        })
        .collect();
    assert!(!updates.is_empty() && updates.iter().all(|len| *len == 1), "{updates:?}");
    assert_eq!(committed(&events).map(|stroke| stroke.points.len()), Some(updates.len() + 1), "the commit carries the whole stroke");
}

#[trial]
fn fill_letter_mid_stroke_cancels_the_stroke_and_ends_busy() {
    let mut rig = Rig::standard();
    rig.fixture.frame(vec![pointer::moved(rig.canvas())]);
    rig.fixture.frame(vec![pointer::press(rig.canvas())]);
    rig.fixture.frame(vec![pointer::moved(rig.canvas() + vec2(20.0, 0.0))]);
    rig.take();
    // The dialog takes the letter wherever focus is; the stroke is the open gesture it cancels.
    rig.fixture.key(Key::L, NONE);
    let events = rig.events();
    assert!(events.contains(&FillEvent::Stroke(FillStrokeEvent::Cancel(CancelReason::Removed))), "{events:?}");
    assert!(rig.work.borrow().session().strokes.is_empty(), "the history records nothing for it");
    rig.fixture.frame(vec![pointer::release(rig.canvas() + vec2(20.0, 0.0))]);
    assert!(committed(&rig.events()).is_none());
}

#[trial]
fn fill_undo_mid_stroke_cancels_the_stroke() {
    let mut rig = Rig::standard();
    rig.fixture.frame(vec![pointer::moved(rig.canvas())]);
    rig.fixture.frame(vec![pointer::press(rig.canvas())]);
    rig.fixture.frame(vec![pointer::moved(rig.canvas() + vec2(20.0, 0.0))]);
    rig.take();
    rig.work.borrow_mut().step(Step::Undo);
    rig.fixture.idle(1);
    let events = rig.events();
    assert!(events.contains(&FillEvent::Stroke(FillStrokeEvent::Cancel(CancelReason::UndoDuringGesture))), "{events:?}");
}

#[trial]
fn fill_settings_changes_are_reported_by_the_pass_that_sees_them() {
    let mut rig = Rig::standard();
    rig.work.borrow_mut().set_color("fill.overlayColor", Srgba::from(Color32::from_rgb(10, 20, 30)));
    rig.fixture.idle(1);
    let outputs = rig.take();
    assert_eq!(outputs.iter().filter(|output| output.settings_changed).count(), 1, "the colour counts once");
    assert_eq!(rig.work.borrow().settings().color, [10, 20, 30]);
    // A key the workspace did not ask for changes nothing.
    rig.work.borrow_mut().set_color("other", Srgba::from(Color32::from_gray(u8::MAX)));
    rig.fixture.idle(1);
    assert!(rig.take().iter().all(|output| !output.settings_changed));
}

#[trial]
fn fill_overlay_tints_the_sampling_area_or_the_excluded_area() {
    // A 4 by 1 plane of one-pixel cells: sampled, sampled, not sampled, sampled.
    let sampling = SamplingPlane::new([0, 0], [4, 1], 1, vec![true, true, false, true]);
    // The fill area covers the second cell.
    let fill_area = SamplingPlane::new([0, 0], [4, 1], 1, vec![false, true, false, false]);
    let document = PixelSize::new(6, 1);
    let tinted = |excluded: bool| -> Vec<bool> {
        let overlay = FillOverlay::new(&sampling, Some(&fill_area), document).excluded(excluded);
        let mut out = ColorImage::filled([6, 1], Color32::from_white_alpha(0));
        overlay.read(PixelRect::of_size(document), 1, &mut out);
        out.pixels.iter().map(|pixel| pixel.a() > 0).collect()
    };
    // Sampling Area: the sampled cells the fill area does not cover; nothing outside the plane.
    assert_eq!(tinted(false), [true, false, false, true, false, false]);
    // Excluded Area: the cells inside the plane that are not sampled and not filled.
    assert_eq!(tinted(true), [false, false, true, false, false, false]);
}

#[trial]
fn fill_preview_panel_has_its_own_view_framed_on_the_sampling_window() {
    let view = Rc::new(RefCell::new(FullWindowView::new()));
    let framing = Rc::new(RefCell::new(PreviewFraming::default()));
    let frame = Rect::new(100.0, 50.0, 200.0, 100.0);
    let (shown, framed) = (Rc::clone(&view), Rc::clone(&framing));
    let mut fixture = Fixture::new(move |ui| {
        let status: Pending<PreviewDone, String> = Pending::default();
        FillPreviewPane::new(&mut shown.borrow_mut(), &mut framed.borrow_mut(), &status, &Grey)
            .frame(Some(frame))
            .navigation(NavigationOptions::default())
            .show(ui);
    });
    fixture.room(vec2(400.0, 400.0)).settle();
    let held = view.borrow().view();
    assert!(held.center.distance(frame.center()) < CENTRE_TOLERANCE, "centred on the window: {:?}", held.center);
    assert!(!view.borrow().is_fit(), "the view is the framing, not the fit of the whole picture");
}
