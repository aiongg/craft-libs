//! The options-bar brush chip ([`BrushPresetChip`]) that opens the Brush Preset picker.

use craft_theme::TypeRole;
use craft_ui::direction;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Tokens, TokensOf as _, role_format};
use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{AsIdSalt, Color32, IdSalt, Painter, Rect, Response, Ui, pos2};

use super::labels::BrushPickerLabels;
use super::style::BrushPresetChipStyle;

/// Photoshop's brush chip in the options bar: the current tip's image, its size under it and a
/// chevron, `brush.chip` (44 × 30). A click toggles the [`BrushPresetPicker`](super::BrushPresetPicker);
/// the host passes [`BrushPresetChipResponse::response`] as the picker's anchor and toggle, so a
/// press on the chip never counts as outside the picker.
///
/// - **Look.** The tip image (`brush.chip_tip`, 14) centred at `brush.chip_tip_at` from the
///   leading top corner, the size number (`caption` role, `text.muted`) centred under it 5 pt
///   off the bottom, the chevron at the trailing end; the hover fill while hovered or open.
/// - **Keys.** One Tab stop; Space and Enter toggle, as a button.
/// - **AccessKit.** A `Button` named "Brush Preset picker" with `has_popup` and `expanded` (the
///   `Popover` sets them on its control anchor); the same text as tooltip.
/// - **RTL.** The tip sits at the leading end, the chevron at the trailing end.
#[must_use = "show the chip with `show`"]
pub struct BrushPresetChip {
    id_salt: IdSalt,
    diameter: f64,
    open: bool,
    labels: Option<BrushPickerLabels>,
}

impl BrushPresetChip {
    /// A chip for the current brush.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), diameter: 0.0, open: false, labels: None }
    }

    /// The brush's diameter in pixels, shown rounded to a whole number in the installed number
    /// format (four digits fit).
    pub fn diameter(mut self, diameter: f64) -> Self {
        self.diameter = diameter;
        self
    }

    /// Whether the picker is open: the chip keeps its hover fill.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The words (shared with the picker); the installed language's by default.
    pub fn labels(mut self, labels: BrushPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the chip; `tip` paints the brush's tip image into its rect (white on dark themes,
    /// the theme's icon colour on light ones, as the app renders it).
    pub fn show(self, ui: &mut Ui, tip: impl FnOnce(&Painter, Rect)) -> BrushPresetChipResponse {
        // pc: crates/ui-egui/src/panels/brush_controls.rs:94-119 `brush_preset_chip` (44 × 30, tip 14 at (14, 11), number 5 pt off the bottom, chevron 9 pt in from the end)
        let Self { id_salt, diameter, open, labels } = self;
        let tokens = Tokens::of(ui);
        let style = BrushPresetChipStyle::of(&tokens);
        let labels = labels.unwrap_or_else(|| BrushPickerLabels::of(ui));
        let rtl = direction::is_rtl(ui);
        let response =
            Interactive::new(id_salt, Role::Button, labels.picker.as_str()).state_layer(false).corner_radius(style.radius).show(ui, style.size);
        let rect = response.rect;
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if response.hovered() || open {
                painter.rect_filled(rect, style.radius, style.hover);
            }
            let x_at = |dx: f32| if rtl { rect.right() - dx } else { rect.left() + dx };
            let tip_centre = pos2(x_at(style.tip_at.x), rect.top() + style.tip_at.y);
            tip(painter, Rect::from_center_size(tip_centre, egui::vec2(style.tip, style.tip)));

            let numbers = LocaleNumbers::of(ui).as_ref().clone().ungrouped();
            let mut job = LayoutJob::single_section(numbers.format(diameter.round(), 0..=0), role_format(&tokens, TypeRole::Caption, style.number));
            job.wrap = TextWrapping::truncate_at_width(style.size.x);
            let galley = painter.layout_job(job);
            let at = pos2(tip_centre.x - galley.size().x / 2.0, rect.top() + style.tip_at.y + style.tip - galley.size().y / 2.0);
            painter.galley(at, galley, Color32::PLACEHOLDER);

            // The chevron keeps the tip's distance from the top edge as its distance from the end.
            let edge_inset = style.tip_at.y - style.tip / 2.0;
            let chevron = Rect::from_center_size(
                pos2(x_at(style.size.x - edge_inset - style.chevron / 2.0), tip_centre.y),
                egui::vec2(style.chevron, style.chevron),
            );
            icons::paint_on(painter, chevron, Icon::ChevronDown, icons::active(ui.ctx()), style.chevron, IconSize::Sm.stroke(), style.chevron_color);
        }
        let clicked = response.clicked();
        BrushPresetChipResponse { response: response.on_hover_text(labels.picker.as_str()), clicked }
    }
}

/// What [`BrushPresetChip::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct BrushPresetChipResponse {
    /// The chip: the picker's anchor (`PopoverAnchor::Control`) and toggle (`response.id`).
    pub response: Response,
    /// A click, Space or Enter on the chip this pass: the host flips the picker's `open`.
    pub clicked: bool,
}
