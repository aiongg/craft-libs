//! The Levels editor and panel, the Curves panel's point helper and the tone pickers, driven through the public
//! widgets as a host would show them.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_imaging::ui::adjust::{
    CurvesMode, CurvesPanel, Levels, LevelsEditor, LevelsPanel, ToneChannel, ToneChannels, ToneCurves, TonePick, TonePickTool, TonePicker,
    TonePickerRow, ToneSpace,
};
use craft_imaging::ui::curve::Curve;
use craft_imaging::ui::histogram::ChannelColor;
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::runtime::focus::Initial;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, Target, ToolFixture, trial};
use egui::{Key, Modifiers, Vec2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn identity() -> Levels {
    Levels::identity(0.0..=255.0)
}

fn grey(v: f32) -> Srgba {
    Srgba { r: v, g: v, b: v, a: 1.0 }
}

fn rgb() -> ToneChannels {
    ToneChannels::of(ToneSpace::Rgb, false)
}

fn monotone(points: &[[f32; 2]]) -> Vec<f32> {
    Curve::monotone(points, 255.0)
}

fn drawn() -> ToneCurves {
    let mut map: Vec<f32> = (0..256).map(|k| k as f32).collect();
    map[128] = 135.0;
    ToneCurves::new(vec![Curve::Map(map); 4], CurvesMode::Pencil)
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

/// A double click on a thumb sets its default.
#[trial]
fn a_double_click_resets_a_thumb() {
    let (mut fixture, levels) = editor(Levels::new(40.0, 1.0, 200.0, 0.0, 255.0));
    let at = thumbs(&fixture)[0].center();
    fixture.click_at(at);
    fixture.click_at(at);
    fixture.idle(2);
    assert_eq!(levels.borrow().in_black, 0.0);
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

/// The Auto button sets the shown channel as one change.
#[trial]
fn the_auto_button_sets_the_shown_channel() {
    let mut composite = [0u32; 256];
    composite[0] = 10;
    composite[1] = 1;
    composite[254] = 11;
    composite[255] = 5;
    composite[100] = 9973;
    let start = Levels::new(5.0, 2.0, 200.0, 10.0, 240.0);
    let levels = Rc::new(RefCell::new(vec![start; 4]));
    let shared = levels.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shared.borrow().clone();
        let histograms = [composite, [10; 256], [10; 256], [10; 256]];
        LevelsPanel::new("Levels", Bind::bound(&mut value), rgb()).histograms(&histograms).show(ui);
        *shared.borrow_mut() = value;
    });
    fixture.room(vec2(480.0, 640.0));
    fixture.settle();
    let auto = fixture.by_label("Auto").expect("the Auto button");
    fixture.click(&auto);
    fixture.idle(2);
    assert_eq!(levels.borrow()[0], start.auto(&composite));
    assert_eq!(levels.borrow()[1], start, "the other channels are untouched");
}

/// A grey document's channel list disables the Neutral Gray picker without the context saying so.
#[trial]
fn the_panel_disables_the_gray_picker_for_gray_channels() {
    let armed = Rc::new(Cell::new(None::<TonePicker>));
    let shared = armed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut levels = vec![identity()];
        let mut picker = shared.get();
        LevelsPanel::new("Levels", Bind::bound(&mut levels), ToneChannels::of(ToneSpace::Rgb, true)).pickers(Bind::bound(&mut picker)).show(ui);
        shared.set(picker);
    });
    fixture.room(vec2(480.0, 640.0));
    fixture.settle();
    let gray = fixture.by_label("Set Neutral Gray Point").expect("the gray picker");
    fixture.click(&gray);
    fixture.idle(2);
    assert_eq!(armed.get(), None, "the disabled picker takes no click");
    let black = fixture.by_label("Set Black Point").expect("the black picker");
    fixture.click(&black);
    fixture.idle(2);
    assert_eq!(armed.get(), Some(TonePicker::Black));
}

/// A click arms a picker, a second click on it disarms it, and a picker the host disabled takes no click.
#[trial]
fn the_picker_row_arms_and_disarms() {
    let armed = Rc::new(Cell::new(None::<TonePicker>));
    let shared = armed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut picker = shared.get();
        TonePickerRow::new(Bind::bound(&mut picker)).enabled_picker(TonePicker::Gray, false).show(ui);
        shared.set(picker);
    });
    fixture.settle();
    let black = fixture.by_label("Set Black Point").expect("the black picker");
    fixture.click(&black);
    fixture.idle(2);
    assert_eq!(armed.get(), Some(TonePicker::Black));
    fixture.click(&black);
    fixture.idle(2);
    assert_eq!(armed.get(), None);
    let gray = fixture.by_label("Set Neutral Gray Point").expect("the gray picker");
    fixture.click(&gray);
    fixture.idle(2);
    assert_eq!(armed.get(), None, "the disabled picker takes no click");
}

/// Photoshop's Levels and Curves: Escape ends an armed picker, and a second Escape cancels the dialog.
#[trial]
fn escape_ends_an_armed_picker_before_it_closes_the_dialog() {
    let armed = Rc::new(Cell::new(Some(TonePicker::Black)));
    let open = Rc::new(Cell::new(true));
    let (shown_armed, shown_open) = (armed.clone(), open.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (mut is_open, mut picker) = (shown_open.get(), shown_armed.get());
        Dialog::new("Adjust")
            .opened(Bind::bound(&mut is_open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .show(ui, |ui, _| {
                TonePickerRow::new(Bind::bound(&mut picker)).show(ui);
            });
        shown_open.set(is_open);
        shown_armed.set(picker);
    });
    fixture.settle();
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert_eq!(armed.get(), None, "Escape disarmed the picker");
    assert!(open.get(), "the first Escape leaves the dialog open");
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(!open.get(), "the second Escape closes the dialog");
}

/// In a colour mode without pickers every picker is disabled and the note shows.
#[trial]
fn an_unavailable_row_disables_every_picker_and_says_why() {
    let armed = Rc::new(Cell::new(None::<TonePicker>));
    let shared = armed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut picker = shared.get();
        TonePickerRow::new(Bind::bound(&mut picker)).unavailable().show(ui);
        shared.set(picker);
    });
    fixture.settle();
    assert!(fixture.by_label("Unavailable in this color mode").is_some(), "the note shows");
    for name in ["Set Black Point", "Set Neutral Gray Point", "Set White Point"] {
        let picker = fixture.by_label(name).expect("a picker");
        fixture.click(&picker);
        fixture.idle(2);
    }
    assert_eq!(armed.get(), None);
}

/// A canvas whose view lets the tool map the pointer to a document point.
fn pick_canvas(label: &'static str, tool: TonePickTool<'static>) -> ToolFixture<TonePickTool<'static>> {
    let view = |room: egui::Rect| {
        let centre = room.center();
        CanvasView::new(room, Point::new(f64::from(centre.x), f64::from(centre.y)), 1.0)
    };
    ToolFixture::with_view(label, tool, view, |_, _| {})
}

/// A press on the canvas samples at the document point and `take` yields a pick for the armed picker once.
#[trial]
fn the_pick_tool_reports_the_sample_for_the_armed_picker() {
    let tool = TonePickTool::new(TonePicker::White, |at| Some(Srgba::rgb(at[0] as f32 / 1200.0, 0.0, 0.0)));
    assert_eq!(tool.picker(), TonePicker::White);
    let mut fixture = pick_canvas("Tone pick", tool);
    let centre = fixture.canvas().center();
    fixture.press_at(centre);
    fixture.release_at(centre);
    let pick: TonePick = fixture.tool_mut().take().expect("a press on the image reports a pick");
    assert_eq!(pick.picker, TonePicker::White);
    assert_eq!(pick.color.r, pick.at[0] as f32 / 1200.0);
    assert_eq!(fixture.tool_mut().take(), None, "a pick is taken once");
}

/// Off the image the sample function returns nothing and nothing is reported.
#[trial]
fn the_pick_tool_reports_nothing_off_the_image() {
    let mut fixture = pick_canvas("Tone pick off", TonePickTool::new(TonePicker::Black, |_| None));
    let centre = fixture.canvas().center();
    fixture.press_at(centre);
    fixture.release_at(centre);
    assert_eq!(fixture.tool_mut().take(), None);
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

/// In pencil mode, for Lab or CMYK, and for an isolated mask the helper returns `None` and the value is unchanged.
#[trial]
fn a_sample_does_nothing_in_pencil_mode_or_for_other_spaces() {
    let mut pencil = drawn();
    let before = pencil.clone();
    assert_eq!(CurvesPanel::add_point_from_sample(&mut pencil, &rgb(), 0, grey(0.5), false, &monotone), None);
    assert_eq!(pencil, before);

    for channels in [ToneChannels::of(ToneSpace::Lab, false), ToneChannels::of(ToneSpace::Cmyk, false)] {
        let mut curves = ToneCurves::identity(&channels);
        let before = curves.clone();
        assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &channels, 0, grey(0.5), false, &monotone), None);
        assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &channels, 1, grey(0.5), true, &monotone), None);
        assert_eq!(curves, before);
    }

    let isolated = ToneChannels::isolated(ToneChannel::named("Alpha 1", ChannelColor::Neutral));
    let mut curves = ToneCurves::identity(&isolated);
    let before = curves.clone();
    assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &isolated, 0, grey(0.5), false, &monotone), None);
    assert_eq!(curves, before);
}

/// An isolated colour channel takes the point at its own value; only a mask or saved channel refuses.
#[trial]
fn a_sample_works_on_an_isolated_colour_channel() {
    let isolated = ToneChannels::isolated(ToneChannel::Red);
    let mut curves = ToneCurves::identity(&isolated);
    let sample = Srgba { r: 0.2, g: 0.4, b: 0.6, a: 1.0 };
    assert_eq!(CurvesPanel::add_point_from_sample(&mut curves, &isolated, 0, sample, true, &monotone), Some(1));
    assert_eq!(curves.curves[0].points().map(<[_]>::len), Some(3));
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

/// A click on the pencil icon in points mode reports the mode; the current mode's icon reports nothing.
#[trial]
fn the_mode_icons_switch_between_points_and_pencil() {
    let modes = Rc::new(RefCell::new(Vec::new()));
    let shared = modes.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut curves = ToneCurves::identity(&rgb());
        let out = CurvesPanel::new("Curves", Bind::bound(&mut curves), rgb()).show(ui);
        shared.borrow_mut().extend(out.mode);
    });
    fixture.room(Vec2::new(480.0, 720.0));
    fixture.settle();
    let points = fixture.by_label("Edit points to modify the curve").expect("the points icon");
    fixture.click(&points);
    fixture.idle(2);
    assert!(modes.borrow().is_empty(), "the current mode's icon reports nothing");
    let pencil = fixture.by_label("Draw to modify the curve").expect("the pencil icon");
    fixture.click(&pencil);
    fixture.idle(2);
    assert_eq!(*modes.borrow(), [CurvesMode::Pencil]);
}
