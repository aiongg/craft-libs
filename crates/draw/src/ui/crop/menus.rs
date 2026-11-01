//! The Crop options bar's two menus: the overlay menu ([`CropOverlayMenu`]) and the gear popover
//! ([`CropOptionsMenu`]).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-C5)")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::bar::CropBarLabels;
use super::crop_guides::CropShield;
use super::options::CropSettings;

/// The overlay menu: a 24 pt icon button (`Icon::Grid`, PhotoCraft's `grid-3x3`) named "Set the
/// overlay options for the Crop tool" with the current overlay as its value, opening a
/// [`Menu`](craft_ui::menu::Menu) at least `crop.menu_width` wide: the six overlays as check rows,
/// a separator, Auto, Always and Never Show Overlay as check rows, a separator, Cycle Overlay with
/// the shortcut text O, and Cycle Orientation with ⇧O (enabled for an overlay with more than one
/// orientation: Triangle, Golden Spiral). A row edits the settings and closes the menu.
pub struct CropOverlayMenu<'a> {
    id_salt: IdSalt,
    settings: &'a mut CropSettings,
    labels: Option<&'a CropBarLabels>,
}

impl<'a> CropOverlayMenu<'a> {
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut CropSettings) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, labels: None }
    }

    /// The words; default: [`CropBarLabels`] of the installed words.
    pub fn labels(mut self, labels: &'a CropBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The button, and the menu while it is open. The response is the button's.
    pub fn show(self, ui: &mut Ui) -> Response {
        // pc: crates/ui-egui/src/tools/crop_overlay.rs:285 `options_button` (min width 220, unified
        // to the gear menu's `crop.menu_width`, lead call 17).
        todo!()
    }
}

/// What [`CropOptionsMenu::show`] reports.
#[derive(Clone, Debug)]
pub struct CropOptionsMenuResponse {
    /// The gear button's response.
    pub response: Response,
    /// Custom… or the shield swatch was clicked: the app asks for a colour.
    pub pick_color: bool,
}

/// The gear popover: a 24 pt icon button (`Icon::Settings`) named "Set additional Crop options",
/// opening a [`Popover`](craft_ui::popover::Popover) `crop.menu_width` wide, at most
/// `crop.menu_max_width`: Use Classic Mode (shortcut text P), Show Cropped Area (H; shows on and
/// disabled in Classic Mode) and Auto Center Preview (disabled in Classic Mode), a separator,
/// Enable Crop Shield, then, indented and disabled with the shield off: Color: Match Canvas or
/// Custom… with the swatch, Opacity (a `SliderRow`, 0–100 %) and Auto Adjust Opacity. The popover
/// stays open on clicks inside and closes on a click outside, Esc or the button.
pub struct CropOptionsMenu<'a> {
    id_salt: IdSalt,
    shield: &'a mut CropShield,
    labels: Option<&'a CropBarLabels>,
}

impl<'a> CropOptionsMenu<'a> {
    pub fn new(id_salt: impl AsIdSalt, shield: &'a mut CropShield) -> Self {
        Self { id_salt: IdSalt::new(id_salt), shield, labels: None }
    }

    /// The words; default: [`CropBarLabels`] of the installed words.
    pub fn labels(mut self, labels: &'a CropBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The button, and the popover while it is open.
    pub fn show(self, ui: &mut Ui) -> CropOptionsMenuResponse {
        // pc: crates/ui-egui/src/tools/crop_shield.rs:150 `options_button` (min 240, max 300).
        todo!()
    }
}
