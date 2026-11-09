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
fn add_to_swatches_keeps_the_dialog_open() {
    let mut rig = rig(ColorPickerDialog::foreground(FOREGROUND).web_colors_remembered(false));
    rig.click("Add to Swatches");
    assert!(rig.result().is_none(), "the dialog stays open");
    let swatch = rig.app.borrow_mut().dialog.take_add_to_swatches().expect("the colour is kept for the app");
    assert_eq!(swatch.color, FOREGROUND);
    assert!(rig.app.borrow_mut().dialog.take_add_to_swatches().is_none(), "it is taken once");
}

#[trial]
fn a_canvas_sample_sets_the_new_colour_while_on_top() {
    let sampled = Srgba::rgb(0.9, 0.1, 0.1);
    let mut rig = rig(ColorPickerDialog::foreground(FOREGROUND).web_colors_remembered(false));
    let request = rig.app.borrow_mut().dialog.requests().take_eyedropper().expect("the dialog on top asks the host to arm its dropper");
    assert_eq!(request.key, "foreground");
    rig.app.borrow_mut().dialog.requests().answer_eyedropper("foreground", sampled);
    rig.fx.settle();
    assert_eq!(rig.app.borrow().dialog.current().map(|picked| picked.color), Some(sampled), "the sample is the new colour");
    assert_eq!(rig.app.borrow().dialog.opening().map(|picked| picked.color), Some(FOREGROUND), "the compare swatch keeps the opening colour");
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

/// A request from a Mixed swatch carries no colour: the dialog opens with empty channel fields and OK disabled, a typed
/// value sets the colour, fills the other fields and enables OK, and OK answers with that colour.
#[trial]
fn a_request_without_a_colour_opens_empty_until_the_first_pick() {
    let request = craft_ui::runtime::requests::ColorRequest::new("fills", None).title("Fills");
    let mut rig = rig(ColorPickerDialog::from_request(request).web_colors_remembered(false));
    let values = |rig: &Rig| -> Vec<String> {
        let tree = rig.fx.tree();
        tree.keys().into_iter().filter_map(|key| tree.value_of(key).map(str::to_owned)).collect()
    };
    let ok = rig.fx.by_label("OK").expect("OK").node;
    assert!(rig.fx.tree().is_disabled_key(ok), "OK waits for a colour");
    assert!(rig.app.borrow().dialog.current().is_none(), "no colour yet");
    assert!(values(&rig).iter().any(|text| text == "Mixed"), "the new swatch shows Mixed: {:?}", values(&rig));
    assert!(!values(&rig).iter().any(|text| text == "000000"), "the hex field is empty, not black");

    let hex = rig.fx.by_label("Hex color").expect("the hex field");
    rig.fx.click(&hex);
    rig.fx.key(Key::A, Modifiers::COMMAND);
    rig.fx.type_text("336699");
    rig.fx.key(Key::Tab, Modifiers::NONE);
    rig.fx.settle();
    assert!(values(&rig).iter().any(|text| text == "51"), "the other fields show the colour: {:?}", values(&rig));
    let ok = rig.fx.by_label("OK").expect("OK").node;
    assert!(!rig.fx.tree().is_disabled_key(ok), "OK takes the colour");
    rig.click("OK");
    match rig.result() {
        Some(DialogResult::Ok(ColorPickerResult { key, picked, .. })) => {
            assert_eq!((key.as_str(), picked.color.to_rgba().0[..3].to_vec()), ("fills", vec![0x33, 0x66, 0x99]));
        }
        other => panic!("OK answers the typed colour: {other:?}"),
    }
}

