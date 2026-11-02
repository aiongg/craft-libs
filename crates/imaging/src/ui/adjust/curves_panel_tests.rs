//! Tests of the Curves panel and its ⌘-click helper (task AD-2).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::curves_panel::overlays_for;
use super::{CurvesMode, CurvesPanel, ToneChannel, ToneChannels, ToneCurves, ToneSpace};
use craft_theme::Srgba;
use craft_ui::composites::{Curve, Sampler};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

struct Host {
    curves: ToneCurves,
    modes: Vec<CurvesMode>,
    events: Vec<EditEvent<Curve>>,
    id: Option<egui::Id>,
}

fn rig(curves: ToneCurves) -> Harness<'static, Host> {
    let state = Host { curves, modes: Vec::new(), events: Vec::new(), id: None };
    let mut h = Harness::builder().with_size([480.0, 720.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let out = CurvesPanel::new("curves-panel", &host.curves, ToneChannels::of(ToneSpace::Rgb, false), &numbers).show(ui);
            host.id = Some(CurvesPanel::id_in(ui, "curves-panel"));
            host.modes.extend(out.mode);
            host.events.extend(out.edit.event);
        },
        state,
    );
    h.run_steps(3);
    h
}

fn rgb() -> ToneChannels {
    ToneChannels::of(ToneSpace::Rgb, false)
}

fn monotone() -> Box<Sampler> {
    Box::new(|points: &[[f32; 2]]| Curve::monotone(points, 255.0))
}

fn grey(v: f32) -> Srgba {
    Srgba { r: v, g: v, b: v, a: 1.0 }
}

fn drawn() -> ToneCurves {
    let mut map: Vec<f32> = (0..256).map(|k| k as f32).collect();
    map[128] = 135.0;
    ToneCurves { curves: vec![Curve::Map(map); 4], mode: CurvesMode::Pencil }
}

// A click on the pencil icon in points mode reports `mode: Some(Pencil)`; the current mode's icon reports nothing.
#[test]
fn curves_panel_mode_icons_switch_between_points_and_pencil() {
    let mut h = rig(ToneCurves::identity(&rgb()));
    h.get_by_role_and_label(Role::Button, "Edit points to modify the curve").click();
    h.run_steps(3);
    assert!(h.state().modes.is_empty(), "the current mode's icon reports nothing");
    h.get_by_role_and_label(Role::Button, "Draw to modify the curve").click();
    h.run_steps(3);
    assert_eq!(h.state().modes, [CurvesMode::Pencil]);
}

// Smooth is disabled in points mode; in pencil mode a click reports one `Set` of the radius-3 moving average of the shown map.
#[test]
fn curves_panel_smooth_is_disabled_in_points_mode_and_sets_in_pencil_mode() {
    let h = rig(ToneCurves::identity(&rgb()));
    assert!(h.get_by_role_and_label(Role::Button, "Smooth").accesskit_node().is_disabled());

    let mut h = rig(drawn());
    h.get_by_role_and_label(Role::Button, "Smooth").click();
    h.run_steps(3);
    let events = &h.state().events;
    assert_eq!(events.len(), 1, "one Set: {events:?}");
    let Some(Curve::Map(map)) = events[0].value() else { panic!("Smooth sets a map: {events:?}") };
    assert_eq!(map.len(), 256);
    assert_eq!((map[0], map[255]), (0.0, 255.0), "the ends keep their values");
    assert_eq!(map[1], 1.0, "the window shrinks to radius 1 at level 1");
    assert_eq!(map[128], 129.0, "(125 + 126 + 127 + 135 + 129 + 130 + 131) / 7");
    assert_eq!(map[131], 132.0, "(135 + 129 + 130 + 131 + 132 + 133 + 134) / 7");
}

// On the RGB composite the editor gets the three other curves as overlays; on Red, and for an isolated channel, none.
#[test]
fn curves_panel_overlays_other_channels_on_the_composite_only() {
    let curves = ToneCurves::identity(&rgb()).curves;
    assert_eq!(overlays_for(&rgb(), 0, &curves).len(), 4, "the composite shows every channel's curve");
    assert!(overlays_for(&rgb(), 1, &curves).is_empty());
    let isolated = ToneChannels::isolated(ToneChannel::Named { name: "Alpha 1".into(), color: craft_ui::composites::ChannelColor::Neutral });
    assert!(overlays_for(&isolated, 0, &curves[..1]).is_empty());
}

// A sample of grey 0.5 on the composite adds a point at input 128 (Rec. 709 luma), output the curve's there rounded, and returns its index.
#[test]
fn curves_add_point_from_sample_inserts_on_the_curve_at_the_sampled_tone() {
    let mut curves = ToneCurves::identity(&rgb());
    curves.curves[0] = Curve::smooth(vec![[0.0, 0.0], [64.0, 40.0], [255.0, 255.0]]);
    let sampler = monotone();
    let want = sampler(&[[0.0, 0.0], [64.0, 40.0], [255.0, 255.0]])[128].round();
    let at = CurvesPanel::add_point_from_sample(&mut curves, &rgb(), 0, grey(0.5), false, &sampler);
    assert_eq!(at, Some(2));
    let Some(points) = curves.curves[0].points() else { panic!("still a point curve") };
    assert_eq!(points, [[0.0, 0.0], [64.0, 40.0], [128.0, want], [255.0, 255.0]]);
    assert_eq!(curves.curves[1], Curve::identity(255.0), "the other channels are untouched");
}

// With `every_channel` on RGB a sample adds one point to Red, Green and Blue each, at that channel's value.
#[test]
fn curves_add_point_from_sample_with_shift_adds_one_per_rgb_channel() {
    let mut curves = ToneCurves::identity(&rgb());
    let sampler = monotone();
    let sample = Srgba { r: 0.2, g: 0.4, b: 0.6, a: 1.0 };
    let at = CurvesPanel::add_point_from_sample(&mut curves, &rgb(), 2, sample, true, &sampler);
    assert_eq!(at, Some(1), "the shown channel's point");
    for (index, input) in [(1, 51.0), (2, 102.0), (3, 153.0)] {
        let Some(points) = curves.curves[index].points() else { panic!("a point curve") };
        assert_eq!(points.len(), 3, "channel {index}");
        assert_eq!(points[1], [input, input], "channel {index}");
    }
    assert_eq!(curves.curves[0], Curve::identity(255.0), "the composite is untouched");
}

// In pencil mode, for Lab or CMYK, and for an isolated channel the helper returns `None` and the value is unchanged.
#[test]
fn curves_add_point_from_sample_does_nothing_in_pencil_mode() {
    let sampler = monotone();
    let mut pencil = drawn();
    let before = pencil.clone();
    assert_eq!(CurvesPanel::add_point_from_sample(&mut pencil, &rgb(), 0, grey(0.5), false, &sampler), None);
    assert_eq!(pencil, before);

    for channels in [ToneChannels::of(ToneSpace::Lab, false), ToneChannels::of(ToneSpace::Cmyk, false)] {
        let mut curves = ToneCurves::identity(&channels);
        let before = curves.clone();
        assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &channels, 0, grey(0.5), false, &sampler), None);
        assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &channels, 1, grey(0.5), true, &sampler), None);
        assert_eq!(curves, before);
    }

    let isolated = ToneChannels::isolated(ToneChannel::Named { name: "Alpha 1".into(), color: craft_ui::composites::ChannelColor::Neutral });
    let mut curves = ToneCurves::identity(&isolated);
    let before = curves.clone();
    assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &isolated, 0, grey(0.5), false, &sampler), None);
    assert_eq!(curves, before);
}

// An isolated colour channel takes the point at its own value; only a mask or saved channel (`Named`) refuses.
#[test]
fn curves_add_point_from_sample_works_on_an_isolated_colour_channel() {
    let sampler = monotone();
    let isolated = ToneChannels::isolated(ToneChannel::Red);
    let mut curves = ToneCurves::identity(&isolated);
    let sample = Srgba { r: 0.2, g: 0.4, b: 0.6, a: 1.0 };
    assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &isolated, 0, sample, true, &sampler), Some(1));
    assert_eq!(curves.curves[0].points().map(<[_]>::len), Some(3));
}

// A point added from a sample and selected through `select_point` is the editor's selected point.
#[test]
fn curves_select_point_selects_the_added_point_in_the_editor() {
    let mut h = rig(ToneCurves::identity(&rgb()));
    let sampler = monotone();
    let at = CurvesPanel::add_point_from_sample(&mut h.state_mut().curves, &rgb(), 0, grey(0.5), false, &sampler).expect("a point");
    let id = h.state().id.expect("drawn");
    CurvesPanel::select_point(&h.ctx, id, at);
    h.run_steps(3);
    let selected: Vec<bool> =
        h.get_all_by(|n| n.label().is_some_and(|l| l.starts_with("Point "))).map(|n| n.accesskit_node().is_selected() == Some(true)).collect();
    assert_eq!(selected, [false, true, false], "the new middle point is selected");
}
