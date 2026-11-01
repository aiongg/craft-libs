//! [`Navigator`]: the preview box with the proxy, its pointer, wheel and keys, and the zoom row
//! under it.

use egui::{AsIdSalt, IdSalt, Painter, Rect, Response, Ui};
use kurbo::Size;

use super::labels::NavigatorLabels;
use super::options::NavigatorSettings;
use super::style::NavigatorStyle;
use crate::canvas::textures::CachedTexture;
use crate::canvas::{CanvasView, NavigationOptions};
use crate::localized::LocaleNumbers;
use craft_theme::ControlSize;

/// A paint callback for [`NavigatorPreview::Paint`]: draws the picture into the fitted rect.
pub type NavigatorPaint<'a> = Box<dyn FnOnce(&Painter, Rect) + 'a>;

/// What the preview box shows: the document's picture, made and cached by the app.
pub enum NavigatorPreview<'a> {
    /// A texture from the app's own [`TextureCache`](crate::canvas::textures::TextureCache),
    /// drawn over the checkerboard into the fitted rect ([`preview_rect`](super::preview_rect)).
    /// Damage updates go through
    /// [`put_region`](crate::canvas::textures::TextureCache::put_region), so the id stays.
    Texture(CachedTexture),
    /// The app paints the picture itself into the fitted rect (PhotoCraft's GPU canvas callback);
    /// the checkerboard is not drawn.
    Paint(NavigatorPaint<'a>),
    /// No picture yet (PhotoCraft waits up to 3 passes for its GPU canvas): the checkerboard
    /// alone, with the proxy over it.
    Waiting,
}

/// The Navigator panel body: the preview box sized to the space it gets, the proxy rectangle,
/// and the zoom row (percent field, Zoom Out, slider, Zoom In) under it. It edits the
/// [`CanvasView`] it is given and reports what changed; the app writes the view back to its own
/// view type.
///
/// **What the user sees and does.**
/// - The zoom row is laid out first, at the bottom of the space; the preview box takes the height
///   above it in the same pass (no previous-pass height, lead call L7), at least
///   `navigator.min_preview`. The box is filled with `surface.canvas`; the document is fitted
///   inside with `navigator.inset` on each side ([`preview_rect`](super::preview_rect)), over a
///   checkerboard of `navigator.checker` squares in `Checkerboard`'s colours.
/// - The proxy is [`proxy_quad`](super::proxy_quad) of the view passed in (the animated one while
///   a zoom animation runs), in the [`ProxyColor`](super::ProxyColor) of the settings, 1.5 wide,
///   over a 1 pt `overlay.halo` outline, each edge clipped to the box less 1 pt; upright it is a
///   rectangle with `navigator.proxy_radius` corners.
/// - Press and drag on the preview: the view centre follows the pointer
///   ([`view_center_for`](super::view_center_for)), clamped to the document, every pass. A click
///   moves the centre to the clicked point. Esc during the drag puts the view back as it was at
///   the press (lead call L5).
/// - With Cmd (macOS) or Ctrl (elsewhere) held at the press, the drag draws marching ants
///   ([`Marquee`](crate::canvas::Marquee) with `MarqueeLook::Ants`) clipped to the fitted rect;
///   the release zooms to the rectangle ([`zoom_to_document_rect`](super::zoom_to_document_rect)).
///   The cursor is `ZoomIn` while the modifier is held over the preview.
/// - The wheel over the preview zooms about the view centre, one ×1.1 notch
///   ([`WheelNavigation::NOTCH`](crate::canvas::navigate::WheelNavigation::NOTCH)), eased over
///   passes when Animated Zoom is on; it zooms whether or not Zoom with Scroll Wheel is on, and
///   never pans. Shift + wheel steps the ladder. A notch past a zoom limit changes nothing and
///   calls [`WheelNavigation::settle`](crate::canvas::navigate::WheelNavigation::settle). A pinch
///   zooms the same way when [`NavigationOptions::pinch_zooms`] is on. The wheel outside the
///   preview does nothing here.
/// - The zoom row: see `zoom_row`. Buttons step the ladder and set
///   [`NavigatorOutput::animate_zoom`] when Animated Zoom is on; the field and slider set the zoom
///   without animation.
///
/// **Keyboard and focus.** The preview is one Tab stop and shows the focus ring. With it focused:
/// arrows move the view by 10 % of the visible span in the arrow's screen direction, Shift ×5
/// (half a span); Page Up and Page Down move by a span vertically; `+` and `-` step the ladder;
/// Home centres the document. Tab then visits the field, Zoom Out, the slider and Zoom In, in
/// that order. Keys go through [`focus::key_owner`](crate::focus::key_owner) so a focused text
/// field keeps its own.
///
/// **AccessKit.** The preview is a `Group` named [`NavigatorLabels::name`] with two `ScrollBar`
/// children ("Visible area horizontal position", "… vertical position"), numeric value 0 to 100
/// (the view centre in percent of the document), whose `Increment` and `Decrement` move the view
/// by the arrow-key step. The picture is a decorative `Image` named
/// [`NavigatorLabels::preview`]. The proxy has no node of its own.
///
/// **Direction.** The preview, proxy and ants never mirror
/// ([`Unmirrored::Image`](crate::direction::Unmirrored) and `Canvas`). The zoom row follows the
/// interface direction except the slider, whose value grows to the right (`SliderValue`), so in
/// right-to-left Zoom In comes before the slider and Zoom Out after it.
///
/// **What the app gets back.** [`NavigatorOutput`]: whether the view changed (every pass of a
/// drag), whether to animate it, whether the settings changed. The widget keeps the press view of
/// a drag and the ants' start in egui memory under its id. With no document the host shows its
/// own empty text and does not draw the widget.
#[must_use = "show the navigator with `show`"]
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
pub struct Navigator<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) size: Size,
    pub(super) numbers: &'a LocaleNumbers,
    pub(super) preview: NavigatorPreview<'a>,
    pub(super) navigation: Option<&'a NavigationOptions>,
    pub(super) settings: Option<&'a mut NavigatorSettings>,
    pub(super) labels: Option<NavigatorLabels>,
    pub(super) control_size: ControlSize,
    pub(super) enabled: bool,
}

impl<'a> Navigator<'a> {
    /// A navigator for a document of `size` (document units); `numbers` formats the zoom field.
    /// `id_salt` keeps its drag, focus and the zoom field's draft.
    pub fn new(id_salt: impl AsIdSalt, size: Size, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            size,
            numbers,
            preview: NavigatorPreview::Waiting,
            navigation: None,
            settings: None,
            labels: None,
            control_size: ControlSize::Sm,
            enabled: true,
        }
    }

    /// The picture; [`NavigatorPreview::Waiting`] by default.
    pub fn preview(mut self, preview: NavigatorPreview<'a>) -> Self {
        self.preview = preview;
        self
    }

    /// The app's navigation preferences (Animated Zoom, Zoom with Trackpad Pinch, Enable
    /// Gestures); [`NavigationOptions::default`] without a call.
    pub fn navigation(mut self, navigation: &'a NavigationOptions) -> Self {
        self.navigation = Some(navigation);
        self
    }

    /// The persisted section: the proxy colour. Without a call the proxy is
    /// [`ProxyColor::Default`](super::ProxyColor::Default) and the output never reports a
    /// settings change.
    pub fn settings(mut self, settings: &'a mut NavigatorSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The words; the installed words' [`NavigatorLabels`] without a call.
    pub fn labels(mut self, labels: NavigatorLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The zoom row's control size; [`ControlSize::Sm`] by default (lead call L6: the library's
    /// `NumberField`, `IconButton` and `Slider` at that size, the field at the library's width).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.control_size = size;
        self
    }

    /// Whether the user can move or zoom the view (default true); disabled, the preview and row
    /// draw dimmed and take no input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Lays out the zoom row at the bottom of the available rect, the preview box above it, reads
    /// this pass's pointer, wheel and keys, and edits `view`. Events are handled in this pass, in
    /// order: the zoom row, then the preview's drag, wheel and keys.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn show(self, ui: &mut Ui, view: &mut CanvasView) -> NavigatorOutput {
        // pc: crates/ui-egui/src/panels/navigator.rs:3-150 `navigator` (box, fit, picture, ⌘-drag, click/drag, wheel, proxy, row);
        // new: row laid out first (L7), keys and AccessKit scroll bars (L4), Esc restores (L5), rotated proxy (L3).
        todo!()
    }
}

/// What [`Navigator::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorOutput {
    /// The preview box: its focus and hover.
    pub response: Response,
    /// The view was changed this pass (a drag pass, a click, a notch, a key, the row). The app
    /// writes it back to its own view and clears its fit-pending and zoom anchor.
    pub view_changed: bool,
    /// The change was a Zoom In or Zoom Out button with Animated Zoom on: the app starts
    /// [`ZoomAnimation`](crate::canvas::ZoomAnimation) from the view before the change.
    pub animate_zoom: bool,
    /// The settings changed (never from the body today; the field exists for the dialog and
    /// later options): the app persists [`NavigatorSettings`].
    pub settings_changed: bool,
    /// Where the document picture was drawn (PhotoCraft's `last_navigator_preview` test hook).
    pub preview_rect: Rect,
    /// The zoom row (PhotoCraft's `last_navigator_controls` test hook).
    pub row_rect: Rect,
}

/// Paints a checkerboard of `style.checker`-point squares in `style.checker_colors` over `rect`,
/// starting at its top-left corner (lead call L8: `CheckerSize` offers 4, 8 and 16 only).
#[expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
fn paint_checker(painter: &Painter, rect: Rect, style: &NavigatorStyle) {
    // pc: crates/ui-egui/src/widgets/color.rs:29 `checker`, called with 6.0 at panels/navigator.rs:25, :28.
    todo!()
}
