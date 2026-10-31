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
//! ScreenColorPicker::tick(ctx);     // once per pass, before shortcuts: Esc belongs to the picker
//! // … the app's UI, where a picker's button may start a pick …
//! ScreenColorPicker::show(ctx);     // once per pass, after the UI: the capture windows
//! # }
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{Context, Id, Response, Ui};

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
    }
}

/// The loupe's grid: 11 by 11 screen pixels round the pointer, the centre one outlined black
/// outside white.
pub const LOUPE_PIXELS: usize = 11;

/// The loupe's zoom in physical pixels per screen pixel: starts at 12, the wheel steps it by 2
/// between 4 and 24.
pub const LOUPE_ZOOM: (usize, usize, usize) = (4, 12, 24);

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

impl ScreenColorPicker {
    /// Asks for a pick for `owner`; ignored while one runs.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:128 `button` (`state.request = Some(id)`).
    pub fn open(ctx: &Context, owner: Id) {
        todo!()
    }

    /// Polls the capture and the pick. Call once per pass before the app's shortcuts. The
    /// editing target is the app's (`target`, any hash of document, layer and top dialog): when
    /// it changes, a running pick is cancelled and an untaken result dropped.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:144 `tick` (on `craft_platform::screen::capture`
    // in place of the `Services::screen_pick` closure).
    pub fn tick(ctx: &Context, target: u64) {
        todo!()
    }

    /// Shows the capture windows (immediate viewports). Call once per pass after the app's UI.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:282 `show`, :263 `viewport_builder`, :327
    // `frame`, :378 `loupe`.
    pub fn show(ctx: &Context) {
        todo!()
    }

    /// The capture windows are up.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:239 `showing`.
    pub fn showing(ctx: &Context) -> bool {
        todo!()
    }

    /// A pick was asked for or runs.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:243 `busy`.
    pub fn busy(ctx: &Context) -> bool {
        todo!()
    }

    /// Cancels the pick (or the request not yet started); no result.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:248 `cancel`.
    pub fn cancel(ctx: &Context) {
        todo!()
    }

    /// `owner`'s picked colour, once.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:122 `take`.
    pub fn take(ctx: &Context, owner: Id) -> Option<Srgba> {
        todo!()
    }

    /// The last pick's notice (denied, unsupported, failed), once, for the status bar.
    // new: PhotoCraft's service closure reported errors to the app itself.
    pub fn take_notice(ctx: &Context) -> Option<String> {
        todo!()
    }

    /// The eyedropper button for `owner` ("Pick screen color", tooltip "Sample a pixel anywhere
    /// on the screen"): a press asks for a pick; returns `owner`'s result when one arrives. An
    /// `IconButton` (`Icon::Eyedropper`) in the Full picker and the compact popover alike.
    // pc: crates/ui-egui/src/dialogs/screen_picker.rs:128 `button`.
    pub fn button(ui: &mut Ui, owner: Id) -> Option<Srgba> {
        todo!()
    }

    /// Photoshop's gesture inside the app's own windows: while a press that began on `button`
    /// is held and dragged, the pixel under the pointer in the app's last frame is sampled live
    /// (through `ViewportCommand::Screenshot`) and returned each pass; a release outside the
    /// app's windows starts the capture view at once instead.
    // new: Q-S4C-5 (both behaviours behind one picker).
    pub fn hold_drag(ui: &Ui, button: &Response, owner: Id) -> Option<Srgba> {
        todo!()
    }
}
