//! The screen colour picker (F04.01): a one-shot sample of any pixel on any monitor. A press on
//! the picker's eyedropper button captures every monitor ([`craft_platform::screen::capture`]),
//! shows each capture in a full-screen window over its monitor with a magnifying loupe, and a
//! click picks the pixel under the pointer, converted from the monitor's profile to sRGB.
//!
//! The capture view works the same on X11, Wayland (the portal asks once), macOS and Windows,
//! where live sampling outside the app's windows is not possible everywhere. Inside the app's own
//! windows a press on the button and a drag samples live ([`ScreenColorPicker::hold_drag`]),
//! Photoshop's gesture.
//!
//! One pick runs at a time, owned by the widget that asked ([`Id`]); its result is taken only by
//! that widget, even when its popup closed meanwhile.
//!
//! ```no_run
//! use craft_color::ui::ScreenColorPicker;
//! # fn app(ctx: &egui::Context) {
//! ScreenColorPicker::tick(ctx, 0); // once per pass, before shortcuts: Esc belongs to the picker
//! ScreenColorPicker::set_monitor(ctx, &craft_cms::Builtin::Srgb.profile()); // the main window's monitor profile
//! // … the app's UI, where a picker's button may start a pick …
//! ScreenColorPicker::show(ctx);     // once per pass, after the UI: the capture windows
//! # }
//! ```

use std::sync::{Arc, Mutex, MutexGuard};

use craft_cms::{Builtin, Intent, Profile, Transform};
use craft_host::Wake;
use craft_platform::screen::{self, Capture, CaptureHandle, ScreenError, ScreenImage};
use craft_theme::{Srgba, TypeRole};
use craft_ui::composites::{StatusLine, Tone};
use craft_ui::controls::IconButton;
use craft_ui::icons::Icon;
use craft_ui::localized::SharedLabels;
use craft_ui::primitives::Gap;
use craft_ui::theme::{ToEgui, current};
use egui::{
    Color32, Context, Id, Key, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextureOptions, Ui, ViewportBuilder, ViewportCommand, ViewportId,
    vec2,
};

craft_ui::words! {
    /// The screen picker's words, English by default.
    #[non_exhaustive]
    pub struct ScreenPickerLabels {
        button: "screenPicker.button" = "Pick screen color";
        tooltip: "screenPicker.tooltip" = "Sample a pixel anywhere on the screen";
        /// The capture windows' title (not shown: the windows have no decorations); `{app}` is
        /// the app's name.
        window_title: "screenPicker.windowTitle" = "{app} — Screen Color Picker";
        click_to_pick: "screenPicker.clickToPick" = "Click to pick · Esc to cancel";
        scroll_to_zoom: "screenPicker.scrollToZoom" = "Scroll to zoom";
        denied: "screenPicker.denied" = "Screen capture was not allowed. Allow screen recording for this app in the system settings.";
        unsupported: "screenPicker.unsupported" = "Screen capture is not available here.";
        invalid_capture: "screenPicker.invalidCapture" = "The screen capture is not valid.";
        texture_limit: "screenPicker.textureLimit" = "This display is larger than the graphics card can show.";
        not_rgb_profile: "screenPicker.notRgbProfile" = "The screen profile must describe RGB pixels.";
        nothing_captured: "screenPicker.nothingCaptured" = "No screen was captured.";
        /// The capture failed for a reason the platform gives; `{reason}` is its text.
        failed: "screenPicker.failed" = "Screen capture failed: {reason}";
    }
}

/// The loupe's grid: 11 by 11 screen pixels round the pointer, the centre one outlined black
/// outside white.
pub const LOUPE_PIXELS: usize = 11;

/// The loupe's zoom in physical pixels per screen pixel: starts at 12, the wheel steps it by 2
/// between 4 and 24.
pub const LOUPE_ZOOM: (usize, usize, usize) = (4, 12, 24);

/// How far a wheel notch changes the loupe's zoom.
const ZOOM_STEP: u32 = 2;

/// The screen colour picker: app-wide state in egui memory, driven by associated functions.
///
/// - [`button`](Self::button) or [`open`](Self::open) asks for a pick for an owner; the next
///   [`tick`](Self::tick) starts the capture. The button is disabled while a pick runs and where
///   the platform has no capture ([`ScreenError::Unsupported`](craft_platform::screen::ScreenError)).
/// - Each monitor's capture shows in an undecorated, always-on-top window of its size and place,
///   made visible only after its first frame is painted. The pointer is a crosshair; the loupe
///   follows it, 24 points down-right, kept on the monitor; under the grid the hex of the
///   pixel (uppercase, `Mono` role) with the zoom ("#FF8800   12×"), then "Click to pick · Esc
///   to cancel" and "Scroll to zoom". The wheel zooms ([`LOUPE_ZOOM`]).
/// - Every way out works: a click or a press-and-drag release picks the pixel under it (or
///   cancels with no pixel there); Esc, a right-click, closing a window, or the editing target
///   changing (another document, layer or dialog) cancels.
/// - The pick is the pixel converted from the monitor's ICC profile to sRGB (craft-cms,
///   relative colorimetric), or the raw pixel when the monitor has no profile.
/// - A denied capture (Wayland portal, macOS permission) ends the pick with no colour and a
///   [`denied`](ScreenPickerLabels::denied) notice for the app's status bar
///   ([`take_notice`](Self::take_notice)).
pub struct ScreenColorPicker;

/// The loupe's offset from the pointer, in points, down and right.
const POINTER_OFFSET: f32 = 24.0;

/// A capture's result as the picker receives it.
pub type CaptureResult = Result<Capture, ScreenError>;

/// What a pick resolves to: the colour, `Ok(None)` for a cancel, or a notice for the status bar.
type Answer = Result<Option<Srgba>, String>;

/// Where the captures come from: the platform capture ([`CaptureHandle`]), or a fake one in
/// tests and stories ([`StillCapture`]).
pub trait CaptureSource: Send {
    /// The capture's result once it has finished; `None` while it runs.
    fn poll(&self) -> Option<CaptureResult>;
}

impl CaptureSource for CaptureHandle {
    fn poll(&self) -> Option<CaptureResult> {
        self.try_take()
    }
}

/// A capture that has already finished with a fixed result: what a story or an app's test gives
/// [`ScreenColorPicker::tick_with`] in place of the screen.
pub struct StillCapture(Mutex<Option<CaptureResult>>);

impl StillCapture {
    /// A capture that answers with `result` on the first poll.
    pub fn new(result: CaptureResult) -> StillCapture {
        StillCapture(Mutex::new(Some(result)))
    }

    /// A capture that answers with one picked colour, as the Wayland portal does.
    pub fn color(color: Srgba) -> StillCapture {
        StillCapture::new(Ok(Capture::Color([color.r, color.g, color.b])))
    }
}

impl CaptureSource for StillCapture {
    fn poll(&self) -> Option<CaptureResult> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

/// Why a capture cannot be shown.
enum SnapshotError {
    Invalid,
    TextureLimit,
    NotRgb,
    /// The colour engine's text for a profile it cannot read or link.
    Profile(String),
}

impl SnapshotError {
    fn notice(self, labels: &ScreenPickerLabels) -> String {
        match self {
            SnapshotError::Invalid => labels.invalid_capture.clone(),
            SnapshotError::TextureLimit => labels.texture_limit.clone(),
            SnapshotError::NotRgb => labels.not_rgb_profile.clone(),
            SnapshotError::Profile(reason) => failed(labels, &reason),
        }
    }
}

fn failed(labels: &ScreenPickerLabels, reason: &str) -> String {
    craft_i18n::fmt_plain(&labels.failed, &[("reason", reason)])
}

/// One monitor's capture as shown: its pixels, their texture and the conversion to sRGB.
#[derive(Clone)]
pub(super) struct Snapshot {
    image: ScreenImage,
    texture: egui::TextureHandle,
    conversion: Option<Arc<Transform>>,
}

impl Snapshot {
    /// Checks the geometry the platform promised and uploads the texture.
    fn new(ctx: &Context, image: ScreenImage) -> Result<Snapshot, SnapshotError> {
        let [w, h] = image.size.map(|n| n as usize);
        let pixels = w.checked_mul(h).filter(|&n| n > 0 && n <= screen::MAX_PIXELS);
        if pixels.and_then(|n| n.checked_mul(4)) != Some(image.rgba.len()) || !image.scale.is_finite() || image.scale <= 0.0 {
            return Err(SnapshotError::Invalid);
        }
        if w.max(h) > ctx.input(|i| i.max_texture_side) {
            return Err(SnapshotError::TextureLimit);
        }
        let conversion = match &image.icc_profile {
            None => None,
            Some(bytes) => {
                let source = Profile::parse(bytes).map_err(|e| SnapshotError::Profile(e.to_string()))?;
                if source.color_space != craft_cms::ColorSpace::Rgb {
                    return Err(SnapshotError::NotRgb);
                }
                Some(Arc::new(
                    Transform::new(&source, Builtin::Srgb.profile(), Intent::RelativeColorimetric, true)
                        .map_err(|e| SnapshotError::Profile(e.to_string()))?,
                ))
            }
        };
        let texture =
            ctx.load_texture("craft-color-screen-capture", egui::ColorImage::from_rgba_unmultiplied([w, h], &image.rgba), TextureOptions::NEAREST);
        Ok(Snapshot { image, texture, conversion })
    }

    fn size(&self) -> [usize; 2] {
        self.image.size.map(|n| n as usize)
    }

    /// The screen pixel under `pos` when the capture fills `rect`.
    fn at(&self, rect: Rect, pos: Pos2) -> Option<(usize, usize)> {
        if !pos.is_finite() || !rect.contains(pos) || rect.size().min_elem() <= 0.0 {
            return None;
        }
        let [w, h] = self.size();
        let p = (pos - rect.min) / rect.size();
        Some((((p.x * w as f32) as usize).min(w - 1), ((p.y * h as f32) as usize).min(h - 1)))
    }

    /// The raw pixel, `None` outside the capture.
    fn pixel(&self, x: usize, y: usize) -> Option<Color32> {
        let [w, h] = self.size();
        if x >= w || y >= h {
            return None;
        }
        let at = (y * w + x) * 4;
        let p = self.image.rgba.get(at..at + 3)?;
        Some(Color32::from_rgb(p[0], p[1], p[2]))
    }

    /// The pixel as sRGB: through the monitor's profile, or raw without one.
    fn sample(&self, x: usize, y: usize) -> Result<Srgba, String> {
        let p = self.pixel(x, y).ok_or("The pixel is outside the capture")?;
        let mut rgb = [p.r(), p.g(), p.b()].map(|v| f32::from(v) / 255.0);
        if let Some(t) = &self.conversion {
            t.apply(&mut rgb, 3).map_err(|e| e.to_string())?;
        }
        Ok(Srgba::from_rgb_array(rgb.map(|v| v.clamp(0.0, 1.0))))
    }
}

/// The picker's state, in egui memory behind a lock so the capture windows' callbacks reach it.
#[derive(Default)]
pub(super) struct Runtime {
    /// The owner that asked; the next `tick` starts the capture.
    request: Option<Id>,
    /// The owner of the running pick.
    owner: Option<Id>,
    source: Option<Box<dyn CaptureSource>>,
    pub(super) images: Vec<Snapshot>,
    /// How the running pick ends, set by the capture windows, `tick` and `cancel`.
    answer: Option<Answer>,
    /// The pick waiting for its owner.
    completed: Option<(Id, Srgba)>,
    notice: Option<String>,
    unsupported: bool,
    zoom: u32,
    generation: u64,
    target: Option<u64>,
    /// The widget whose press is being tracked by `hold_drag`, with the app's last frame.
    hold: Option<Hold>,
    /// The main window's monitor profile (monitor to sRGB), set by `set_monitor`; the app's
    /// frame holds monitor values.
    monitor: Option<(u64, Option<Arc<Transform>>)>,
}

struct Hold {
    owner: Id,
    dragged: bool,
    frame: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
}

type Shared = Arc<Mutex<Runtime>>;

pub(super) fn runtime(ctx: &Context) -> Shared {
    let id = Id::unique("craft-color-screen-picker");
    ctx.data_mut(|d| d.get_temp_mut_or_insert_with::<Shared>(id, Shared::default).clone())
}

pub(super) fn lock(shared: &Shared) -> MutexGuard<'_, Runtime> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

impl ScreenColorPicker {
    /// Asks for a pick for `owner`; ignored while one runs.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:128 `button` (`state.request = Some(id)`).
    pub fn open(ctx: &Context, owner: Id) {
        let shared = runtime(ctx);
        let mut s = lock(&shared);
        if s.owner.is_none() && s.request.is_none() {
            s.request = Some(owner);
            ctx.request_repaint();
        }
    }

    /// Polls the capture and the pick. Call once per pass before the app's shortcuts. The
    /// editing target is the app's (`target`, any hash of document, layer and top dialog): when
    /// it changes, a running pick is cancelled and an untaken result dropped.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:144 `tick` (on `craft_platform::screen::capture`
    // in place of the `Services::screen_pick` closure).
    pub fn tick(ctx: &Context, target: u64) {
        Self::tick_with(ctx, target, &|| {
            let repaint = ctx.clone();
            Box::new(screen::capture(ctx.native_pixels_per_point().unwrap_or(1.0), Wake::new(move || repaint.request_repaint())))
        });
    }

    /// [`tick`](Self::tick) with `start` making the capture, for a story or a test that has no
    /// screen to capture.
    pub fn tick_with(ctx: &Context, target: u64, start: &dyn Fn() -> Box<dyn CaptureSource>) {
        let shared = runtime(ctx);
        let mut s = lock(&shared);
        if s.completed.is_some() && s.target != Some(target) {
            s.completed = None;
        }
        if let Some(owner) = s.request.take() {
            s.completed = None;
            s.notice = None;
            s.source = Some(start());
            s.owner = Some(owner);
            s.target = Some(target);
            s.zoom = LOUPE_ZOOM.1 as u32;
            s.generation = s.generation.wrapping_add(1);
        }
        if s.owner.is_none() {
            return;
        }
        if s.target != Some(target)
            || ctx.input(|i| i.viewport().close_requested())
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
        {
            s.answer = Some(Ok(None));
        }
        if s.answer.is_none()
            && let Some(result) = s.source.as_ref().and_then(|source| source.poll())
        {
            s.source = None;
            match result {
                Ok(Capture::Color([r, g, b])) => s.answer = Some(Ok(Some(Srgba::rgb(r, g, b)))),
                Ok(Capture::Images(images)) => match images.into_iter().map(|image| Snapshot::new(ctx, image)).collect::<Result<Vec<_>, _>>() {
                    Ok(images) if !images.is_empty() => s.images = images,
                    Ok(_) => s.answer = Some(Err(ScreenPickerLabels::shared_ctx(ctx).nothing_captured.clone())),
                    Err(e) => s.answer = Some(Err(e.notice(&ScreenPickerLabels::shared_ctx(ctx)))),
                },
                Err(ScreenError::Cancelled) => s.answer = Some(Ok(None)),
                Err(ScreenError::Denied) => s.answer = Some(Err(ScreenPickerLabels::shared_ctx(ctx).denied.clone())),
                Err(ScreenError::Unsupported) => {
                    s.unsupported = true;
                    s.answer = Some(Err(ScreenPickerLabels::shared_ctx(ctx).unsupported.clone()));
                }
                Err(ScreenError::Failed(reason)) => s.answer = Some(Err(failed(&ScreenPickerLabels::shared_ctx(ctx), &reason))),
            }
        }
        if let Some(answer) = s.answer.take() {
            let owner = s.owner.take();
            s.source = None;
            for n in 0..s.images.len() {
                ctx.send_viewport_cmd_to(viewport(s.generation, n), ViewportCommand::Close);
            }
            s.images.clear();
            match answer {
                Ok(Some(color)) => s.completed = owner.map(|owner| (owner, color)),
                Ok(None) => {}
                Err(notice) => s.notice = Some(notice),
            }
        }
    }

    /// Shows the capture windows (immediate viewports). Call once per pass after the app's UI.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:282 `show`, :263 `viewport_builder`, :327
    // `frame`, :378 `loupe`.
    pub fn show(ctx: &Context) {
        let shared = runtime(ctx);
        let (images, generation) = {
            let s = lock(&shared);
            (s.images.clone(), s.generation)
        };
        let labels = ScreenPickerLabels::shared_ctx(ctx);
        let app = ctx.input(|i| i.raw.viewports.get(&ViewportId::ROOT).and_then(|v| v.title.clone())).unwrap_or_default();
        let title = craft_i18n::fmt(&labels.window_title, &[("app", app.as_str())]);
        for (n, snapshot) in images.into_iter().enumerate() {
            let builder = builder(ctx, &snapshot.image, &title);
            let shared = shared.clone();
            // Immediate, so it paints inside the main window's frame and the pick is answered in
            // the same pass.
            ctx.show_viewport_immediate(viewport(generation, n), builder, move |ui, _| {
                {
                    let s = lock(&shared);
                    if s.generation != generation || s.owner.is_none() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                        return;
                    }
                }
                frame(ui, &shared, &snapshot);
                // Visible only once its first frame is painted, so the window's empty first frame
                // never covers the desktop.
                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(true));
                // Raised and focused once, after that first frame: a monitor-sized window created
                // always-on-top can fail swapchain creation on Windows, and a window shown later
                // is not always given the keyboard, so Esc would not reach it.
                let raised = Id::unique(("craft-color-screen-picker-raised", ui.ctx().viewport_id()));
                if !ui.ctx().data_mut(|d| std::mem::replace(d.get_temp_mut_or_default::<bool>(raised), true)) {
                    ui.ctx().send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                    ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                }
            });
        }
    }

    /// The capture windows are up.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:239 `showing`.
    pub fn showing(ctx: &Context) -> bool {
        !lock(&runtime(ctx)).images.is_empty()
    }

    /// A pick was asked for or runs.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:243 `busy`.
    pub fn busy(ctx: &Context) -> bool {
        let shared = runtime(ctx);
        let s = lock(&shared);
        s.owner.is_some() || s.request.is_some()
    }

    /// Cancels the pick (or the request not yet started); no result.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:248 `cancel`.
    pub fn cancel(ctx: &Context) {
        let shared = runtime(ctx);
        let mut s = lock(&shared);
        if s.owner.is_some() {
            s.answer = Some(Ok(None));
        } else {
            s.request = None;
        }
        drop(s);
        ctx.request_repaint();
    }

    /// `owner`'s picked colour, once.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:122 `take`.
    pub fn take(ctx: &Context, owner: Id) -> Option<Srgba> {
        let shared = runtime(ctx);
        let mut s = lock(&shared);
        if s.completed.as_ref().is_some_and(|(id, _)| *id == owner) { s.completed.take().map(|(_, color)| color) } else { None }
    }

    /// The last pick's notice (denied, unsupported, failed), once, for the status bar.
    // new: PhotoCraft's service closure reported errors to the app itself.
    pub fn take_notice(ctx: &Context) -> Option<StatusLine> {
        lock(&runtime(ctx)).notice.take().map(|text| StatusLine { text, tone: Tone::Bad })
    }

    /// The monitor profile the app's own frame is shown through: the main window's
    /// ([`DisplayCaches::monitor_for`](crate::display::DisplayCaches::monitor_for)). Call it each pass;
    /// [`hold_drag`](Self::hold_drag) converts the sampled frame back to sRGB through it. Without
    /// a call the frame counts as sRGB.
    pub fn set_monitor(ctx: &Context, monitor: &Profile) {
        let shared = runtime(ctx);
        let mut s = lock(&shared);
        let hash = monitor.content_hash();
        if s.monitor.as_ref().is_some_and(|(known, _)| *known == hash) {
            return;
        }
        let back = if monitor.same_colors(Builtin::Srgb.profile()) {
            None
        } else {
            Transform::new(monitor, Builtin::Srgb.profile(), Intent::RelativeColorimetric, true).ok().map(Arc::new)
        };
        s.monitor = Some((hash, back));
    }

    /// Whether the platform can capture the screen: not on wasm, and not after a capture came
    /// back unsupported. The dialog and the paint chip show their eyedropper button only when it
    /// is.
    pub fn available(ctx: &Context) -> bool {
        !cfg!(target_arch = "wasm32") && !lock(&runtime(ctx)).unsupported
    }

    /// The eyedropper button for `owner` ("Pick screen color", tooltip "Sample a pixel anywhere
    /// on the screen"): a press asks for a pick; returns `owner`'s result when one arrives. An
    /// `IconButton` (`Icon::Eyedropper`) in the Full picker and the compact popover alike.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:128 `button`.
    pub fn button(ui: &mut Ui, owner: Id) -> Option<Srgba> {
        let ctx = ui.ctx().clone();
        let labels = ScreenPickerLabels::shared(ui);
        let result = Self::take(&ctx, owner);
        let available = Self::available(&ctx);
        let enabled = available && !Self::busy(&ctx);
        let response = ui.add(IconButton::new(Icon::Eyedropper, labels.button.as_str()).show_tooltip(false).enabled(enabled));
        let response =
            if available { response.on_hover_text(labels.tooltip.as_str()) } else { response.on_disabled_hover_text(labels.unsupported.as_str()) };
        if enabled && response.clicked() {
            Self::open(&ctx, owner);
        }
        result
    }

    /// Photoshop's gesture inside the app's own windows: while a press that began on `button`
    /// is held and dragged, the pixel under the pointer in the app's last frame is sampled live
    /// (through `ViewportCommand::Screenshot`) and returned each pass, converted from the
    /// monitor's profile ([`set_monitor`](Self::set_monitor)) to sRGB; a release outside the
    /// app's windows starts the capture view at once instead.
    // new: Q-S4C-5 (both behaviours behind one picker).
    pub fn hold_drag(ui: &Ui, button: &Response, owner: Id) -> Option<Srgba> {
        let ctx = ui.ctx();
        let shared = runtime(ctx);
        let (down, dragging, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.is_decidedly_dragging(), i.pointer.latest_pos()));
        let inside = pos.filter(|p| ctx.content_rect().contains(*p));
        let mut s = lock(&shared);
        if s.owner.is_some() || s.request.is_some() {
            s.hold = None;
            return None;
        }
        if s.hold.is_none() && button.is_pointer_button_down_on() && down {
            s.hold = Some(Hold { owner, dragged: false, frame: Arc::default() });
        }
        let hold = s.hold.as_mut().filter(|h| h.owner == owner)?;
        hold.dragged |= dragging;
        let frame = hold.frame.clone();
        let dragged = hold.dragged;
        let back = s.monitor.as_ref().and_then(|(_, back)| back.clone());
        let sample = || {
            let image = frame.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
            sample_frame(&image, ctx.content_rect(), inside?, back.as_deref())
        };
        if down {
            if dragged {
                let (sink, repaint) = (frame.clone(), ctx.clone());
                ctx.request_screenshot(move |image| {
                    *sink.lock().unwrap_or_else(|e| e.into_inner()) = Some(image);
                    repaint.request_repaint();
                });
                ctx.request_repaint();
            }
            return if dragged { sample() } else { None };
        }
        s.hold = None;
        drop(s);
        if !dragged {
            return None;
        }
        match inside {
            Some(_) => sample(),
            None => {
                Self::open(ctx, owner);
                None
            }
        }
    }
}

/// The colour of the app's frame under `pos`, when the frame covers `rect`; the frame holds
/// monitor values, `back` takes them to sRGB.
fn sample_frame(image: &egui::ColorImage, rect: Rect, pos: Pos2, back: Option<&Transform>) -> Option<Srgba> {
    let [w, h] = image.size;
    let p = (pos - rect.min) / rect.size();
    if w == 0 || h == 0 || !(0.0..1.0).contains(&p.x) || !(0.0..1.0).contains(&p.y) {
        return None;
    }
    let c = image.pixels.get(((p.y * h as f32) as usize) * w + (p.x * w as f32) as usize)?;
    let mut rgb = [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0);
    if let Some(back) = back {
        back.apply(&mut rgb, 3).ok()?;
    }
    Some(Srgba::from_rgb_array(rgb.map(|v| v.clamp(0.0, 1.0))))
}

fn viewport(generation: u64, n: usize) -> ViewportId {
    ViewportId::from_hash_of(("craft-color-screen-picker", generation, n))
}

/// A capture window: undecorated, over its monitor, hidden until its first frame is painted.
fn builder(ctx: &Context, image: &ScreenImage, title: &str) -> ViewportBuilder {
    let scale = image.scale.max(f32::EPSILON) * ctx.zoom_factor();
    ViewportBuilder::default()
        .with_title(title)
        .with_decorations(false)
        .with_resizable(false)
        .with_position(egui::pos2(image.origin[0] as f32, image.origin[1] as f32) / scale)
        .with_inner_size(vec2(image.size[0] as f32, image.size[1] as f32) / scale)
        .with_visible(false)
        .with_clamp_size_to_monitor_size(false)
}

/// One pass of a capture window over its snapshot. The window covers the monitor, so every way
/// out works: Esc, closing it, a right-click, and a left click or a press-and-drag release, which
/// picks the pixel under it (or cancels when there is none).
pub(super) fn frame(ui: &mut Ui, shared: &Shared, snapshot: &Snapshot) {
    let ctx = ui.ctx().clone();
    let answer = |a: Answer| {
        lock(shared).answer = Some(a);
        ctx.request_repaint_of(ViewportId::ROOT);
    };
    if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(Key::Escape) || i.pointer.secondary_released()) {
        answer(Ok(None));
        return;
    }
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
        let rect = ui.max_rect();
        ui.painter().image(snapshot.texture.id(), rect, Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
        let response = ui.interact(rect, ui.scope_id().with("sample"), Sense::click_and_drag());
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        let released = response.clicked() || response.drag_stopped();
        let Some((pos, (x, y))) = ui.ctx().pointer_latest_pos().and_then(|pos| Some((pos, snapshot.at(rect, pos)?))) else {
            if released {
                answer(Ok(None));
            }
            return;
        };
        let zoom = {
            let (zoom_min, zoom_max) = (LOUPE_ZOOM.0 as u32, LOUPE_ZOOM.2 as u32);
            let wheel: f32 = ui.input(|i| {
                i.raw
                    .events
                    .iter()
                    .map(|e| match e {
                        egui::Event::MouseWheel { delta, .. } => delta.y,
                        _ => 0.0,
                    })
                    .sum()
            });
            let mut s = lock(shared);
            if wheel > 0.0 {
                s.zoom = (s.zoom + ZOOM_STEP).min(zoom_max);
            } else if wheel < 0.0 {
                s.zoom = s.zoom.saturating_sub(ZOOM_STEP).max(zoom_min);
            }
            s.zoom
        };
        let picked = snapshot.sample(x, y);
        loupe(ui, snapshot, rect, pos, (x, y), zoom, picked.as_ref().ok().copied());
        if released {
            answer(picked.map(Some).map_err(|reason| failed(&ScreenPickerLabels::shared(ui), &reason)));
        }
    });
}

/// The loupe beside the pointer: the grid of screen pixels round `pixel`, `zoom` physical pixels
/// to each, then the hex with the zoom and the two hints.
fn loupe(ui: &Ui, snapshot: &Snapshot, screen: Rect, pos: Pos2, pixel: (usize, usize), zoom: u32, picked: Option<Srgba>) {
    let tokens = current(ui.ctx());
    let cells = LOUPE_PIXELS;
    let labels = ScreenPickerLabels::shared(ui);
    let cell = zoom as f32 / ui.ctx().pixels_per_point();
    let pad = Gap::Inline.points(&tokens);
    let text = tokens.color.text.normal.to_egui();
    let p = ui.painter();
    let (x, y) = pixel;
    let hex = match picked {
        Some(color) => color.to_hex(),
        None => snapshot.pixel(x, y).map(|c| format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())).unwrap_or_default(),
    };
    let readout = p.layout_no_wrap(format!("{}   {zoom}×", hex.to_uppercase()), tokens.font(TypeRole::Mono), text);
    let hint = p.layout_no_wrap(labels.click_to_pick.clone(), tokens.font(TypeRole::Caption), text);
    let wheel = p.layout_no_wrap(labels.scroll_to_zoom.clone(), tokens.font(TypeRole::Caption), text);
    let grid = cells as f32 * cell;
    let width = (grid + 2.0 * pad).max(readout.size().x.max(hint.size().x).max(wheel.size().x) + 2.0 * pad);
    let height = pad + grid + pad + readout.size().y + hint.size().y + wheel.size().y + pad;
    let desired = pos + vec2(POINTER_OFFSET, POINTER_OFFSET);
    let at = egui::pos2(desired.x.min(screen.right() - width).max(screen.left()), desired.y.min(screen.bottom() - height).max(screen.top()));
    let area = Rect::from_min_size(at, vec2(width, height));
    let radius = tokens.radius.popup;
    p.rect_filled(area, radius, tokens.color.surface.raised.to_egui());
    p.rect_stroke(area, radius, Stroke::new(1.0, tokens.color.line.border.to_egui()), StrokeKind::Inside);
    let origin = at + vec2((width - grid) / 2.0, pad);
    let half = cells / 2;
    let beyond = tokens.color.surface.canvas.to_egui();
    for row in 0..cells {
        for col in 0..cells {
            let color = (x + col).checked_sub(half).zip((y + row).checked_sub(half)).and_then(|(px, py)| snapshot.pixel(px, py)).unwrap_or(beyond);
            p.rect_filled(Rect::from_min_size(origin + vec2(col as f32, row as f32) * cell, vec2(cell, cell)), 0.0, color);
        }
    }
    let center = Rect::from_min_size(origin + vec2(half as f32, half as f32) * cell, vec2(cell, cell));
    p.rect_stroke(center.expand(1.0), 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    p.rect_stroke(center, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
    let mut footer = at + vec2(pad, pad + grid + pad);
    for galley in [readout, hint, wheel] {
        let step = galley.size().y;
        p.galley(footer, galley, text);
        footer.y += step;
    }
}
