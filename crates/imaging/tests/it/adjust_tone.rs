//! The Levels editor and panel, the Curves panel's point helper and the tone pickers, driven through the public
//! widgets as a host would show them.

use craft_imaging::ui::adjust::{CurvesPanel, Levels, LevelsEditor, ToneChannels, ToneCurves, ToneSpace};
use craft_imaging::ui::curve::Curve;
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::testing::{Fixture, Target, trial};
use egui::vec2;
use std::cell::RefCell;
use std::rc::Rc;

fn grey(v: f32) -> Srgba {
    Srgba { r: v, g: v, b: v, a: 1.0 }
}

fn rgb() -> ToneChannels {
    ToneChannels::of(ToneSpace::Rgb, false)
}

fn monotone(points: &[[f32; 2]]) -> Vec<f32> {
    Curve::monotone(points, 255.0)
}

/// The thumbs of a shown editor, top row first and left to right within a row.
fn thumbs(fixture: &Fixture) -> Vec<Target> {
    let mut targets = fixture.by_role(Role::SLIDER);
    targets.sort_by(|a, b| a.center().y.total_cmp(&b.center().y).then(a.center().x.total_cmp(&b.center().x)));
    targets
}

/// An editor over `start`, with the levels it holds after the gestures shared out.
fn editor(start: Levels) -> (Fixture, Rc<RefCell<Levels>>) {
    let levels = Rc::new(RefCell::new(start));
    let shared = levels.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut value = *shared.borrow();
        LevelsEditor::new("Levels", Bind::bound(&mut value)).show(ui);
        *shared.borrow_mut() = value;
    });
    fixture.room(vec2(480.0, 640.0));
    fixture.settle();
    (fixture, levels)
}

/// The input ends keep their gap: the black thumb dragged far past the white one stops two steps short of it.
#[trial]
fn input_ends_keep_their_gap_and_output_thumbs_cross() {
    let (mut fixture, levels) = editor(Levels::new(10.0, 1.0, 200.0, 100.0, 200.0));
    let at = thumbs(&fixture);
    assert_eq!(at.len(), 5, "three input thumbs and two output thumbs");
    fixture.drag(&at[0], vec2(2000.0, 0.0));
    fixture.idle(2);
    assert_eq!(levels.borrow().in_black, 198.0, "two steps short of the white end");

    let at = thumbs(&fixture);
    fixture.drag(&at[4], vec2(-2000.0, 0.0));
    fixture.idle(2);
    let after = *levels.borrow();
    assert!(after.out_white < after.out_black, "the output white thumb passed the black one: {after:?}");
}

/// The gamma thumb sits between the black and white thumbs at `0.5^gamma` of the way.
#[trial]
fn the_gamma_thumb_follows_black_and_white_through_the_half_power() {
    for (gamma, share) in [(1.0, 0.5), (2.0, 0.25)] {
        let (fixture, _) = editor(Levels::new(50.0, gamma, 250.0, 0.0, 255.0));
        let at = thumbs(&fixture);
        let drawn = (at[1].center().x - at[0].center().x) / (at[2].center().x - at[0].center().x);
        assert!((drawn - share).abs() < 0.01, "gamma {gamma} sits at {drawn}, not {share}");
    }
}

/// Auto clips a tenth of a percent at both ends, back to gamma 1, outputs kept.
#[trial]
fn auto_follows_the_clip_rule_at_both_ends() {
    let start = Levels::new(3.0, 0.5, 100.0, 7.0, 9.0);
    let empty = start.auto(&[0; 256]);
    assert_eq!((empty.in_black, empty.in_white, empty.gamma), (253.0, 255.0, 1.0));
    assert_eq!((empty.out_black, empty.out_white), (7.0, 9.0), "the outputs stay");

    let mut counts = [0u32; 256];
    counts[0] = 20;
    counts[1] = 1;
    counts[255] = 12;
    counts[254] = 8;
    counts[253] = 1;
    counts[128] = 20000 - 42;
    let auto = start.auto(&counts);
    assert_eq!((auto.in_black, auto.in_white), (1.0, 253.0));
}

/// A sample of grey 0.5 on the composite adds a point at input 128 (Rec. 709 luma), output the curve's there rounded.
#[trial]
fn a_sample_adds_a_point_on_the_curve_at_the_sampled_tone() {
    let mut curves = ToneCurves::identity(&rgb());
    curves.curves[0] = Curve::smooth(vec![[0.0, 0.0], [64.0, 40.0], [255.0, 255.0]]);
    let want = monotone(&[[0.0, 0.0], [64.0, 40.0], [255.0, 255.0]])[128].round();
    let at = CurvesPanel::add_point_from_sample(&mut curves, &rgb(), 0, grey(0.5), false, &monotone);
    assert_eq!(at, Some(2));
    let Some(points) = curves.curves[0].points() else { panic!("still a point curve") };
    assert_eq!(points, [[0.0, 0.0], [64.0, 40.0], [128.0, want], [255.0, 255.0]]);
    assert_eq!(curves.curves[1], Curve::identity(255.0), "the other channels are untouched");
}

/// With every channel asked for, a sample adds one point to Red, Green and Blue each, at that channel's value.
#[trial]
fn a_sample_with_shift_adds_one_point_per_rgb_channel() {
    let mut curves = ToneCurves::identity(&rgb());
    let sample = Srgba { r: 0.2, g: 0.4, b: 0.6, a: 1.0 };
    let at = CurvesPanel::add_point_from_sample(&mut curves, &rgb(), 2, sample, true, &monotone);
    assert_eq!(at, Some(1), "the shown channel's point");
    for (index, input) in [(1, 51.0), (2, 102.0), (3, 153.0)] {
        let Some(points) = curves.curves[index].points() else { panic!("a point curve") };
        assert_eq!(points.len(), 3, "channel {index}");
        assert_eq!(points[1], [input, input], "channel {index}");
    }
    assert_eq!(curves.curves[0], Curve::identity(255.0), "the composite is untouched");
}

/// The output at the sampled input is the curve's as drawn: each run between corner points is sampled on its own.
#[trial]
fn a_sample_reads_each_corner_run_on_its_own() {
    // A sampler that draws the chord from a run's first to its last point.
    let chord = |points: &[[f32; 2]]| {
        let (a, b) = (points[0], points[points.len() - 1]);
        (0..=255).map(|x| if x as f32 <= a[0] { a[1] } else { a[1] + (b[1] - a[1]) * ((x as f32 - a[0]) / (b[0] - a[0])).min(1.0) }).collect()
    };
    let mut curves = ToneCurves::identity(&rgb());
    curves.curves[0] = Curve::from_parts(vec![[0.0, 0.0], [100.0, 200.0], [255.0, 255.0]], vec![false, true, false]);
    let at = CurvesPanel::add_point_from_sample(&mut curves, &rgb(), 0, grey(0.5), false, &chord);
    assert_eq!(at, Some(2));
    let Some(points) = curves.curves[0].points() else { panic!("still a point curve") };
    assert_eq!(points[2], [128.0, 210.0], "the chord of the run from the corner at 100, not of the whole curve");
}
