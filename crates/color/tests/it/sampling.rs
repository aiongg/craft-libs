//! The Color Picker dialog, the droppers, the eyedropper, the colour samplers and the screen pick.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, CanvasView, PointerButton};
use craft_cms::convert::{CmykConverter, srgb_to_cmyk_naive};
use craft_color::ui::{
    CaptureSource, ColorPickerDialog, ColorPickerResult, ColorSamplerTool, ColorSamplers, DropperTool, EyedropperOptions, EyedropperTool,
    EyedropperWords, MAX_SAMPLERS, PickedColor, SampleSize, SampleSource, SamplerCache, SamplerError, SamplerReadout, SamplerReadouts, SamplerWords,
    ScreenPick, StillCapture,
};
use craft_keymap::Mods;
use craft_theme::{ModelValues, Srgba};
use craft_ui::canvas::{CanvasSpace, CanvasTool, ToolAction, ToolContext, ToolPointer};
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::runtime::env::Words;
use craft_ui::testing::{Fixture, ToolFixture, trial};
use egui::{Context, Key, Modifiers, Pos2, Rect};

const ROOM: egui::Vec2 = egui::Vec2 { x: 400.0, y: 300.0 };
/// The fixture's room for a dialog.
const DIALOG_ROOM: egui::Vec2 = egui::Vec2 { x: 900.0, y: 700.0 };

/// A canvas whose document points equal its screen points: 400 by 300 at zoom 1, centred on (200, 150).
fn context() -> ToolContext<'static> {
    let rect = Rect::from_min_size(Pos2::ZERO, ROOM);
    let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
    ToolContext::new(rect, Some(CanvasSpace::new(view, 1.0)))
}

fn at(x: f32, y: f32) -> ToolPointer {
    ToolPointer::new(Pos2::new(x, y), PointerButton::Primary, 0.0)
}

fn alt(x: f32, y: f32) -> ToolPointer {
    at(x, y).mods(Mods::ALT)
}

const SIZE: [u32; 2] = [300, 200];

#[trial]
fn points_stop_at_ten() {
    let mut samplers = ColorSamplers::new();
    for i in 0..MAX_SAMPLERS {
        assert_eq!(samplers.add([i as f64 + 0.7, 3.2], [100, 100]), Ok(i));
    }
    assert_eq!(samplers.add([50.0, 50.0], [100, 100]), Err(SamplerError::Full));
    assert_eq!(samplers.points().len(), 10);
    assert_eq!(samplers.points()[0], [0.0, 3.0], "a position floors to its pixel");
    assert_eq!(samplers.add([100.0, 5.0], [100, 100]), Err(SamplerError::Outside));
    assert_eq!(samplers.add([-0.5, 5.0], [100, 100]), Err(SamplerError::Outside));
}

#[trial]
fn points_move_remove_and_hit() {
    let mut samplers = ColorSamplers::new();
    for x in [10.0, 20.0, 30.0] {
        samplers.add([x, 10.0], [100, 100]).unwrap();
    }
    samplers.move_point(1, [22.9, 11.1], [100, 100]).unwrap();
    assert_eq!(samplers.points()[1], [22.0, 11.0]);
    assert_eq!(samplers.hit([21.0, 11.0], 7.0), Some(1));
    assert_eq!(samplers.hit([60.0, 60.0], 7.0), None);
    samplers.move_point(1, [-3.0, 11.0], [100, 100]).unwrap();
    assert_eq!(samplers.points(), [[10.0, 10.0], [30.0, 10.0]], "a point dropped off the canvas is removed");
    samplers.remove(0).unwrap();
    assert_eq!(samplers.points(), [[30.0, 10.0]], "the later points move up one number");
    assert_eq!(samplers.remove(3), Err(SamplerError::NoSuchPoint));
    assert_eq!(samplers.move_point(3, [1.0, 1.0], [100, 100]), Err(SamplerError::NoSuchPoint));
}

#[trial]
fn readouts_use_the_documents_converter() {
    let converter = CmykConverter::coated();
    let blue = Srgba { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    let reading = SamplerReadout::new(2, [4.0, 9.0], blue, converter);
    assert_eq!((reading.number, reading.position, reading.rgb), (3, [4, 9], [0, 0, 255]));
    let managed = converter.from_srgb([0.0, 0.0, 1.0]).map(|v| (v * 100.0).round() as u8);
    assert_eq!(reading.cmyk, managed);
    let naive = srgb_to_cmyk_naive([0.0, 0.0, 1.0]).map(|v| (v * 100.0).round() as u8);
    assert_ne!(reading.cmyk, naive);
    let odd = SamplerReadout::new(0, [0.0, 0.0], Srgba { r: f32::NAN, g: 2.0, b: -1.0, a: 1.0 }, converter);
    assert_eq!(odd.rgb, [0, 255, 0], "channels that are not finite read as 0 and the rest is clamped");
}

#[trial]
fn readouts_are_cached_per_document_and_converter() {
    use std::cell::Cell;
    let mut samplers = ColorSamplers::new();
    samplers.add([1.0, 1.0], [10, 10]).unwrap();
    let reads = Cell::new(0);
    let read = |_: [f64; 2]| {
        reads.set(reads.get() + 1);
        Srgba { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }
    };
    let mut cache = SamplerCache::default();
    let coated = CmykConverter::coated();
    let first = SamplerReadouts::new(&samplers, read).revision(3).document("a").cmyk(coated).get(&mut cache);
    let again = SamplerReadouts::new(&samplers, read).revision(3).document("a").cmyk(coated).get(&mut cache);
    assert!(Arc::ptr_eq(&first, &again) && reads.get() == 1, "unchanged: from the cache");
    SamplerReadouts::new(&samplers, read).revision(3).document("b").cmyk(coated).get(&mut cache);
    assert_eq!(reads.get(), 2, "another document at the same revision reads again");
    let fogra = CmykConverter::new(Arc::new(craft_cms::Builtin::CoatedFogra39.profile().clone())).unwrap();
    let other = SamplerReadouts::new(&samplers, read).revision(3).document("a").cmyk(&fogra).get(&mut cache);
    assert_eq!(reads.get(), 3, "another CMYK working space reads again");
    assert_ne!(other[0].cmyk, first[0].cmyk);
}

fn sampler_tool(points: &[[f64; 2]]) -> ColorSamplerTool<'static> {
    let samplers: &'static mut ColorSamplers = Box::leak(Box::new(ColorSamplers::new()));
    for p in points {
        samplers.add(*p, SIZE).unwrap();
    }
    ColorSamplerTool::new(samplers, SIZE)
}

/// Presses at `from` and releases at `to`.
fn click_drag(tool: &mut impl CanvasTool, from: ToolPointer, to: ToolPointer) {
    let mut cx = context();
    if tool.press(&from, &mut cx) == ToolAction::Claimed {
        tool.release(&to, &mut cx);
    }
}

#[trial]
fn a_point_dragged_off_the_canvas_is_removed() {
    let mut tool = sampler_tool(&[[50.0, 50.0], [100.0, 50.0]]);
    click_drag(&mut tool, at(100.0, 50.0), at(100.3, 50.0));
    assert_eq!(tool.samplers().points(), [[50.0, 50.0], [100.0, 50.0]], "a release within half a pixel does nothing");
    click_drag(&mut tool, at(100.0, 50.0), at(150.6, 80.2));
    assert_eq!(tool.samplers().points(), [[50.0, 50.0], [150.0, 80.0]], "dropped inside it moves, onto the pixel");
    click_drag(&mut tool, at(150.0, 80.0), at(350.0, 80.0));
    assert_eq!(tool.samplers().points(), [[50.0, 50.0]], "dropped off the canvas it is removed");
    click_drag(&mut tool, at(20.0, 20.0), at(20.0, 20.0));
    click_drag(&mut tool, at(320.0, 20.0), at(320.0, 20.0));
    assert_eq!(tool.samplers().points(), [[50.0, 50.0], [20.0, 20.0]], "a click places a point; outside the canvas none");
    assert_eq!(tool.take_error(), None);
}

#[trial]
fn alt_click_removes_and_renumbers() {
    let mut tool = sampler_tool(&[[10.0, 10.0], [60.0, 10.0], [110.0, 10.0]]);
    click_drag(&mut tool, alt(62.0, 12.0), alt(62.0, 12.0));
    assert_eq!(tool.samplers().points(), [[10.0, 10.0], [110.0, 10.0]], "the third point is now number 2");
    click_drag(&mut tool, alt(200.0, 100.0), alt(200.0, 100.0));
    assert_eq!(tool.samplers().points().len(), 2, "Alt-click on empty space does nothing");
    for i in 0..MAX_SAMPLERS {
        click_drag(&mut tool, at(10.0 + 20.0 * i as f32, 150.0), at(10.0 + 20.0 * i as f32, 150.0));
    }
    assert_eq!(tool.samplers().points().len(), MAX_SAMPLERS);
    let toast = tool.take_error().expect("the press past the limit reports");
    let words = Words::english().set::<SamplerWords>();
    assert_eq!(toast.message.as_str(), words.limit.fill(&[("max", "10")]));
    assert!(tool.take_error().is_none());
}

/// A press on empty canvas places a point and the drag carries it: the point ends where the pointer is released.
#[trial]
fn a_press_on_empty_canvas_places_a_point_that_the_drag_carries() {
    let mut tool = sampler_tool(&[]);
    click_drag(&mut tool, at(40.0, 40.0), at(90.6, 70.2));
    assert_eq!(tool.samplers().points(), [[90.0, 70.0]], "one point, at the release");
}

/// The point follows the pointer while it is down; Escape puts it back and a release off the canvas removes it.
#[trial]
fn a_point_follows_the_pointer_during_the_drag() {
    let mut tool = sampler_tool(&[[50.0, 50.0]]);
    let mut cx = context();
    assert_eq!(tool.press(&at(50.0, 50.0), &mut cx), ToolAction::Claimed);
    tool.drag(&at(120.6, 90.2), &mut cx);
    assert_eq!(tool.samplers().points(), [[120.0, 90.0]], "mid-drag the point is under the pointer");
    tool.drag(&at(350.0, 90.0), &mut cx);
    assert_eq!(tool.samplers().points(), [[120.0, 90.0]], "off the canvas it waits where it was");
    tool.cancel(CancelReason::Escape, &mut cx);
    assert_eq!(tool.samplers().points(), [[50.0, 50.0]], "Escape restores the point the press found");

    assert_eq!(tool.press(&at(200.0, 100.0), &mut cx), ToolAction::Claimed);
    tool.drag(&at(210.0, 110.0), &mut cx);
    tool.cancel(CancelReason::Escape, &mut cx);
    assert_eq!(tool.samplers().points(), [[50.0, 50.0], [200.0, 100.0]], "a new point stays at its press");

    assert_eq!(tool.press(&at(200.0, 100.0), &mut cx), ToolAction::Claimed);
    tool.drag(&at(250.0, 100.0), &mut cx);
    tool.release(&at(350.0, 100.0), &mut cx);
    assert_eq!(tool.samplers().points(), [[50.0, 50.0]], "released off the canvas it is removed");
}

#[trial]
fn the_sampler_tool_meets_the_tool_rules() {
    let view = |rect: Rect| CanvasView::new(rect, Point::new(f64::from(SIZE[0]) / 2.0, f64::from(SIZE[1]) / 2.0), 1.0);
    let mut tool = ToolFixture::with_view("Color Sampler", sampler_tool(&[]), view, |_, _| {});
    tool.check();
}

#[trial]
fn sample_sizes_follow_photoshops_list() {
    let words = Words::english().set::<EyedropperWords>();
    let shown: Vec<_> = SampleSize::ALL.iter().map(|s| (s.pixels(), s.label(&words).as_str().to_owned())).collect();
    let want = [
        (1, "Point Sample"),
        (3, "3 by 3 Average"),
        (5, "5 by 5 Average"),
        (11, "11 by 11 Average"),
        (31, "31 by 31 Average"),
        (51, "51 by 51 Average"),
        (101, "101 by 101 Average"),
    ];
    assert_eq!(shown, want.map(|(n, text)| (n, text.to_owned())));
    let sources: Vec<_> = SampleSource::ALL.iter().map(|s| (s.id(), s.label(&words).as_str().to_owned())).collect();
    assert_eq!(
        sources,
        [
            ("current", "Current Layer"),
            ("currentAndBelow", "Current & Below"),
            ("all", "All Layers"),
            ("allNoAdjustments", "All Layers No Adjustments"),
            ("currentAndBelowNoAdjustments", "Current & Below No Adjustments"),
        ]
        .map(|(id, text)| (id, text.to_owned()))
    );
}

#[trial]
fn alt_picks_the_background_and_a_drag_resamples() {
    let red = Srgba::rgb(1.0, 0.0, 0.0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    let mut tool = EyedropperTool::new(EyedropperOptions::default(), move |doc, size, source| {
        log.borrow_mut().push((doc, size, source));
        Some(if doc[0] < 100.0 { red } else { Srgba::rgb(0.0, 0.0, 1.0) })
    });
    let mut cx = context();
    assert_eq!(tool.press(&at(50.0, 40.0), &mut cx), ToolAction::Claimed);
    let pick = tool.take().expect("a pick");
    assert_eq!((pick.color, pick.background), (red, false), "a press picks the foreground");
    tool.drag(&at(150.0, 40.0), &mut cx);
    assert_eq!(tool.take().map(|pick| pick.color), Some(Srgba::rgb(0.0, 0.0, 1.0)), "a drag resamples");
    tool.release(&at(150.0, 40.0), &mut cx);
    tool.cancel(CancelReason::Escape, &mut cx);
    tool.press(&alt(50.0, 40.0), &mut cx);
    let pick = tool.take().expect("a pick");
    assert_eq!((pick.color, pick.background), (red, true), "Alt picks the background");
    assert_eq!(seen.borrow()[0], ([50.0, 40.0], SampleSize::Point, SampleSource::All), "the sample gets the document point and the options");
}

#[trial]
fn the_canvas_tool_samples_for_the_armed_dropper() {
    let green = Srgba::rgb(0.0, 1.0, 0.0);
    let mut tool = DropperTool::new(|doc| (doc[0] >= 0.0).then_some(green));
    let mut cx = context();
    assert_eq!(tool.press(&at(30.0, 30.0), &mut cx), ToolAction::Claimed);
    assert_eq!(tool.take(), Some(green));
    assert_eq!(tool.take(), None, "a sample is taken once");
    tool.drag(&at(60.0, 30.0), &mut cx);
    assert_eq!(tool.take(), Some(green), "a drag updates the dialog while the button is held");
    ToolFixture::new("Dropper", DropperTool::new(move |_| Some(green))).check();
}

struct Rig {
    fx: Fixture,
    app: Rc<RefCell<App>>,
}

struct App {
    dialog: ColorPickerDialog,
    stack: DialogStack,
    result: Option<DialogResult<ColorPickerResult>>,
}

fn rig(dialog: ColorPickerDialog) -> Rig {
    let mut stack = DialogStack::default();
    stack.push(dialog.id(), None);
    let app = Rc::new(RefCell::new(App { dialog, stack, result: None }));
    let shown = Rc::clone(&app);
    let mut fx = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { dialog, stack, result } = &mut *app;
        if result.is_none() {
            *result = dialog.show(ui, stack);
        }
    });
    fx.room(DIALOG_ROOM).settle();
    Rig { fx, app }
}

impl Rig {
    fn click(&mut self, label: &str) {
        let target = self.fx.by_label(label).unwrap_or_else(|| panic!("no `{label}` on screen"));
        self.fx.click(&target);
        self.fx.settle();
    }

    fn result(&self) -> Option<DialogResult<ColorPickerResult>> {
        self.app.borrow_mut().result.take()
    }
}

const FOREGROUND: Srgba = Srgba::rgb(0.2, 0.4, 0.6);

#[trial]
fn ok_answers_the_opening_key() {
    let mut rig = rig(ColorPickerDialog::foreground(FOREGROUND).web_colors_remembered(false));
    rig.click("OK");
    match rig.result() {
        Some(DialogResult::Ok(ColorPickerResult { key, picked, .. })) => assert_eq!((key.as_str(), picked.color), ("foreground", FOREGROUND)),
        other => panic!("OK answers the colour: {other:?}"),
    }
}

#[trial]
fn enter_presses_ok_and_esc_cancel_as_a_dialog_footer_does() {
    let mut rig = rig(ColorPickerDialog::background(FOREGROUND).web_colors_remembered(false));
    rig.fx.key(Key::Escape, Modifiers::NONE);
    rig.fx.settle();
    assert!(matches!(rig.result(), Some(DialogResult::Cancel)), "Esc cancels");
    let mut rig = self::rig(ColorPickerDialog::background(FOREGROUND).web_colors_remembered(false));
    rig.fx.key(Key::Enter, Modifiers::NONE);
    rig.fx.settle();
    assert!(matches!(rig.result(), Some(DialogResult::Ok(ColorPickerResult { key, .. })) if key == "background"), "Enter is OK");
}

#[trial]
fn add_to_swatches_keeps_the_dialog_open() {
    let mut rig = rig(ColorPickerDialog::foreground(FOREGROUND).web_colors_remembered(false));
    rig.click("Add to Swatches");
    assert!(rig.result().is_none(), "the dialog stays open");
    let swatch = rig.app.borrow_mut().dialog.take_add_to_swatches().expect("the colour is kept for the app");
    assert_eq!(swatch.color, FOREGROUND);
    assert!(rig.app.borrow_mut().dialog.take_add_to_swatches().is_none(), "it is taken once");
}

#[trial]
fn a_dialog_opened_from_a_request_answers_its_key() {
    let request = craft_ui::runtime::requests::ColorRequest::new("stroke", FOREGROUND).title("Stroke colour");
    let mut rig = rig(ColorPickerDialog::from_request(request).web_colors_remembered(false));
    assert!(rig.fx.by_label("Stroke colour").is_some(), "the request's title names the dialog");
    rig.click("OK");
    assert!(matches!(rig.result(), Some(DialogResult::Ok(ColorPickerResult { key, .. })) if key == "stroke"));
}

#[trial]
fn a_given_title_fills_its_arg_after_translation() {
    let dialog = ColorPickerDialog::new("vine", "Color Picker ({name})", FOREGROUND).arg("name", "Vine").web_colors_remembered(false);
    let rig = rig(dialog);
    assert!(rig.fx.by_label("Color Picker (Vine)").is_some());
}

#[trial]
fn a_canvas_sample_sets_the_new_colour_while_on_top() {
    let sampled = Srgba::rgb(0.9, 0.1, 0.1);
    let mut rig = rig(ColorPickerDialog::foreground(FOREGROUND).web_colors_remembered(false));
    let request = rig.app.borrow_mut().dialog.requests().take_eyedropper().expect("the dialog on top asks the host to arm its dropper");
    assert_eq!(request.key, "foreground");
    rig.app.borrow_mut().dialog.requests().answer_eyedropper("foreground", sampled);
    rig.fx.settle();
    assert_eq!(rig.app.borrow().dialog.current().color, sampled, "the sample is the new colour");
    assert_eq!(rig.app.borrow().dialog.opening().color, FOREGROUND, "the compare swatch keeps the opening colour");
}

#[trial]
fn a_host_cancel_disarms_the_dropper() {
    let rig = rig(ColorPickerDialog::foreground(FOREGROUND).web_colors_remembered(false));
    let ctx = Context::default();
    rig.app.borrow_mut().dialog.cancelled(&ctx);
    assert!(rig.app.borrow_mut().dialog.requests().take_eyedropper().is_none(), "a cancelled dialog leaves no request");
}

#[trial]
fn set_color_and_confirm_answer_ok_with_the_colour() {
    let mut dialog = ColorPickerDialog::new("fill", "Fill", FOREGROUND).web_colors_remembered(false);
    let set = PickedColor::rgb(Srgba::rgb(0.5, 0.5, 0.0));
    dialog.set_color(set);
    assert_eq!(dialog.current(), set);
    assert_eq!(dialog.opening().color, FOREGROUND, "the opening colour stays");
    let result = dialog.confirm(&Context::default()).expect("confirm answers");
    assert_eq!((result.key.as_str(), result.picked), ("fill", set));
}

#[trial]
fn typed_cmyk_inks_come_back_in_the_result() {
    let inks = ModelValues::new("cmyk", &[10.0, 20.0, 30.0, 40.0]);
    let mut dialog = ColorPickerDialog::new("fill", "Fill", FOREGROUND).web_colors_remembered(false).kept(inks);
    assert_eq!(dialog.opening().model, Some(inks), "the kept model values open the dialog");
    let result = dialog.confirm(&Context::default()).expect("confirm answers");
    assert_eq!(result.picked.model, Some(inks), "the inks the colour was picked in come back");
}

#[trial]
fn the_result_goes_to_its_owner_only() {
    let ctx = Context::default();
    let blue = Srgba::rgb(0.0, 0.0, 1.0);
    let mut pick = ScreenPick::new();
    pick.start("fill");
    assert!(pick.busy());
    pick.tick_with(&ctx, 1, &|| Box::new(StillCapture::color(blue)));
    assert!(!pick.busy(), "a colour capture settles at once");
    assert_eq!(pick.take("stroke"), None, "another owner takes nothing");
    assert_eq!(pick.take("fill"), Some(blue));
    assert_eq!(pick.take("fill"), None, "the colour is taken once");
}

/// A capture that never finishes.
struct Never;

impl CaptureSource for Never {
    fn poll(&self) -> Option<craft_color::ui::CaptureResult> {
        None
    }
}

#[trial]
fn every_way_out_cancels() {
    let ctx = Context::default();
    let mut pick = ScreenPick::new();
    pick.start("fill");
    pick.cancel();
    assert!(!pick.busy(), "a request that has not started is dropped");
    pick.start("fill");
    pick.tick_with(&ctx, 1, &|| Box::new(StillCapture::new(Err(craft_platform::screen::ScreenError::Cancelled))));
    assert!(!pick.busy());
    assert_eq!(pick.take("fill"), None, "a cancelled capture picks nothing");
    pick.start("fill");
    pick.tick_with(&ctx, 1, &|| Box::new(Never));
    assert!(pick.busy(), "a capture that has not finished keeps the pick");
    pick.tick_with(&ctx, 2, &|| Box::new(Never));
    assert!(!pick.busy(), "a pick whose window changed is cancelled");
    pick.start("fill");
    pick.tick_with(&ctx, 2, &|| Box::new(Never));
    pick.cancel();
    pick.tick_with(&ctx, 2, &|| Box::new(Never));
    assert!(!pick.busy(), "cancel ends a running pick");
}

#[trial]
fn a_picked_color_answers_without_capture_windows() {
    let ctx = Context::default();
    let mut pick = ScreenPick::new();
    pick.start("fill");
    pick.tick_with(&ctx, 1, &|| Box::new(StillCapture::color(Srgba::rgb(0.25, 0.5, 0.75))));
    assert_eq!(pick.take("fill"), Some(Srgba::rgb(0.25, 0.5, 0.75)));
    assert!(pick.take_notice(&Words::english().set::<craft_color::ui::ScreenPickerWords>()).is_none());
}
