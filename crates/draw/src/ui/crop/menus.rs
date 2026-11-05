//! The Crop options bar's two menus: the overlay menu ([`CropOverlayMenu`]) and the gear popover
//! ([`CropOptionsMenu`]).

use egui::{AsIdSalt, Color32, IdSalt, Key, KeyboardShortcut, Modifiers, Response, Ui};

use super::bar::{CropBar, CropBarLabels};
use super::crop_guides::{CropGuide, CropShield, GuideShow, ShieldColor};
use super::options::CropSettings;
use craft_theme::Srgba;
use craft_ui::composites::SliderRow;
use craft_ui::controls::{Checkbox, IconButton, Kbd, RadioSet, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::patterns::OptionsBar;
use craft_ui::popover::{Popover, PopoverAnchor};
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::{Tokens, TokensOf as _};

/// The overlay's name in `labels`.
fn guide_name(labels: &CropBarLabels, guide: CropGuide) -> &str {
    match guide {
        CropGuide::Thirds => &labels.thirds,
        CropGuide::Grid => &labels.grid,
        CropGuide::Diagonal => &labels.diagonal,
        CropGuide::Triangle => &labels.triangle,
        CropGuide::GoldenRatio => &labels.golden_ratio,
        CropGuide::GoldenSpiral => &labels.golden_spiral,
    }
}

/// The show rule's name in `labels`.
fn show_name(labels: &CropBarLabels, show: GuideShow) -> &str {
    match show {
        GuideShow::Auto => &labels.show_auto,
        GuideShow::Always => &labels.show_always,
        GuideShow::Never => &labels.show_never,
    }
}

/// A row of the overlay menu.
#[derive(Clone, Copy, Debug, PartialEq)]
enum OverlayRow {
    Guide(CropGuide),
    Show(GuideShow),
    Cycle,
    Orientation,
}

/// The key `key` as the text of a menu row's shortcut.
fn key_text(modifiers: Modifiers, key: Key) -> Kbd {
    Kbd::new(&KeyboardShortcut::new(modifiers, key))
}

/// The overlay menu: a 24 pt icon button (`Icon::Grid`, PhotoCraft's `grid-3x3`) named "Set the
/// overlay options for the Crop tool" with the current overlay as its value, opening a
/// [`Menu`](craft_ui::menu::Menu) at least `crop.menu_width` wide (through `min_width`): the six overlays as check rows,
/// a separator, Auto, Always and Never Show Overlay as check rows, a separator, Cycle Overlay with
/// the shortcut text O, and Cycle Orientation with ⇧O (enabled for an overlay with more than one
/// orientation: Triangle, Golden Spiral). A row edits the settings and closes the menu.
pub struct CropOverlayMenu<'a> {
    id_salt: IdSalt,
    settings: &'a mut CropSettings,
    labels: Option<CropBarLabels>,
}

impl<'a> CropOverlayMenu<'a> {
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut CropSettings) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, labels: None }
    }

    /// The words; default: [`CropBarLabels`] of the installed words.
    pub fn labels(mut self, labels: CropBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The button, and the menu while it is open. The response is the button's.
    pub fn show(self, ui: &mut Ui) -> Response {
        // pc: crates/ui-egui/src/tools/crop_overlay.rs:285 `options_button` (min width 220, unified
        // to the gear menu's `crop.menu_width`, lead call 17).
        let Self { id_salt, settings, labels } = self;
        let labels = labels.unwrap_or_else(|| CropBarLabels::of(ui));
        let labels = &labels;
        let style = CropBar::style(&Tokens::of(ui));
        let name = guide_name(labels, settings.guide).to_owned();
        let button = ui.add(IconButton::new(Icon::Grid, labels.overlay.as_str()).size(OptionsBar::CONTROL_SIZE).id_salt((&id_salt, "button")));
        ui.ctx().accesskit_node_builder(button.id, |node| node.set_value(name));

        let mut entries: Vec<MenuEntry<OverlayRow>> = CropGuide::ALL
            .into_iter()
            .map(|guide| MenuItem::new(guide_name(labels, guide), OverlayRow::Guide(guide)).check(settings.guide == guide).into())
            .collect();
        entries.push(MenuEntry::Separator);
        entries.extend(
            GuideShow::ALL
                .into_iter()
                .map(|show| MenuItem::new(show_name(labels, show), OverlayRow::Show(show)).check(settings.guide_show == show).into()),
        );
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.cycle_overlay.as_str(), OverlayRow::Cycle).shortcut(key_text(Modifiers::NONE, Key::O)).into());
        entries.push(
            MenuItem::new(labels.cycle_orientation.as_str(), OverlayRow::Orientation)
                .shortcut(key_text(Modifiers::SHIFT, Key::O))
                .enabled(settings.guide.orientations() > 1)
                .into(),
        );
        match PressMenu::new(&entries).size(OptionsBar::CONTROL_SIZE).min_width(style.menu_width).show(ui, &button).chosen {
            Some(OverlayRow::Guide(guide)) => settings.set_guide(guide),
            Some(OverlayRow::Show(show)) => settings.guide_show = show,
            Some(OverlayRow::Cycle) => settings.cycle_guide(),
            Some(OverlayRow::Orientation) => settings.cycle_orientation(),
            None => {}
        }
        button
    }
}

/// What [`CropOptionsMenu::show`] reports.
#[derive(Clone, Debug)]
pub struct CropOptionsMenuResponse {
    /// The gear button's response.
    pub response: Response,
    /// Custom… or the shield swatch was clicked: the app opens its colour picker with this
    /// request and sets [`CropShield::custom_color`] from the answer for
    /// [`CropOptionsMenu::COLOR_KEY`].
    pub pick_color: Option<ColorRequest>,
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
    labels: Option<CropBarLabels>,
}

impl<'a> CropOptionsMenu<'a> {
    /// The [`ColorRequest::key`] of the shield colour.
    pub const COLOR_KEY: &'static str = "crop-shield-color";

    pub fn new(id_salt: impl AsIdSalt, shield: &'a mut CropShield) -> Self {
        Self { id_salt: IdSalt::new(id_salt), shield, labels: None }
    }

    /// The words; default: [`CropBarLabels`] of the installed words.
    pub fn labels(mut self, labels: CropBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The button, and the popover while `open`; a click on the button toggles `open`.
    pub fn show(self, ui: &mut Ui, open: &mut bool) -> CropOptionsMenuResponse {
        // pc: crates/ui-egui/src/tools/crop_shield.rs:150 `options_button` (min 240, max 300).
        let Self { id_salt, shield, labels } = self;
        let labels = labels.unwrap_or_else(|| CropBarLabels::of(ui));
        let labels = &labels;
        let numbers = LocaleNumbers::of(ui);
        let style = CropBar::style(&Tokens::of(ui));
        let button = ui.add(IconButton::new(Icon::Settings, labels.options.as_str()).size(OptionsBar::CONTROL_SIZE).id_salt((&id_salt, "button")));
        if button.clicked() {
            *open = !*open;
        }
        let mut pick_color = None;
        Popover::new((&id_salt, "popover"), PopoverAnchor::Control(button.clone())).toggle(button.id).label(labels.options.as_str()).show(
            ui,
            open,
            |ui, _| {
                ui.set_min_width(style.menu_width);
                ui.set_max_width(style.menu_max_width);
                pick_color = gear_rows(ui, shield, labels, &numbers);
            },
        );
        CropOptionsMenuResponse { response: button, pick_color }
    }
}

/// A check box with its shortcut text at the reading end.
fn shortcut_check(ui: &mut Ui, label: &str, value: &mut bool, key: Key) {
    direction::horizontal(ui, |ui| {
        let _ = Checkbox::bool(label, value).size(OptionsBar::CONTROL_SIZE).show(ui).response;
        ui.with_layout(direction::trailing(ui), |ui| ui.add(key_text(Modifiers::NONE, key)));
    });
}

/// The gear popover's rows. Returns the request when Custom… or the swatch asked for a colour.
fn gear_rows(ui: &mut Ui, shield: &mut CropShield, labels: &CropBarLabels, numbers: &LocaleNumbers) -> Option<ColorRequest> {
    let sm = OptionsBar::CONTROL_SIZE;
    let mut pick = false;
    shortcut_check(ui, &labels.classic_mode, &mut shield.classic_mode, Key::P);
    // Classic Mode greys the other two out; Show Cropped Area then shows as on, which is how it acts.
    let classic = shield.classic_mode;
    ui.add_enabled_ui(!classic, |ui| {
        if classic {
            shortcut_check(ui, &labels.show_cropped_area, &mut true, Key::H);
        } else {
            shortcut_check(ui, &labels.show_cropped_area, &mut shield.show_cropped_area, Key::H);
        }
        let _ = Checkbox::bool(labels.auto_center.as_str(), &mut shield.auto_center_preview).size(sm).show(ui).response;
    });
    ui.separator();
    let _ = Checkbox::bool(labels.shield.as_str(), &mut shield.enabled).size(sm).show(ui).response;
    ui.add_enabled_ui(shield.enabled, |ui| {
        ui.indent("crop-shield", |ui| {
            direction::horizontal(ui, |ui| {
                ui.add(Text::new(labels.color.as_str()).content(Content::Muted));
                let mut color = shield.color;
                let mut set = RadioSet::new("color", labels.color_name.as_str(), &mut color)
                    .item(ShieldColor::MatchCanvas)
                    .item(ShieldColor::Custom)
                    .size(sm)
                    .begin(ui);
                set.radio(ui, 0, &labels.match_canvas);
                let custom = set.radio(ui, 1, &labels.custom);
                let _ = set.end(ui).response;
                shield.color = color;
                if custom.clicked() {
                    shield.color = ShieldColor::Custom;
                    pick = true;
                }
                if shield.color == ShieldColor::Custom {
                    let [r, g, b] = shield.custom_color;
                    if ui.add(Swatch::new("shield-swatch", labels.shield_color.as_str(), Color32::from_rgb(r, g, b)).size(sm)).clicked() {
                        pick = true;
                    }
                }
            });
            if !shield.opacity.is_finite() {
                shield.opacity = CropShield::default().opacity;
            }
            let row = SliderRow::new("opacity", labels.opacity_name.as_str(), f64::from(shield.opacity), 0.0..=100.0, numbers)
                .unit(Unit::new("{n}%"))
                .decimals(0)
                .size(sm)
                .show(ui);
            for event in row.edits() {
                if let Some(value) = event.value() {
                    shield.opacity = *value as f32;
                }
            }
            let _ = Checkbox::bool(labels.auto_adjust.as_str(), &mut shield.auto_adjust).size(sm).show(ui).response;
        });
    });
    pick.then(|| {
        let [r, g, b] = shield.custom_color;
        ColorRequest::new(CropOptionsMenu::COLOR_KEY, Srgba::from_rgb_array([r, g, b].map(|c| f32::from(c) / 255.0)))
            .title(labels.shield_color.as_str())
    })
}
