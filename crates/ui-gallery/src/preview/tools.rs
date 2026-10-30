//! The tool rail at the start edge and the tool keys.

use craft_theme::Srgba;
use craft_ui::composites::{PairWell, SwatchPair, Well};
use craft_ui::icons::Icon;
use craft_ui::localized::LocaleNumbers;
use craft_ui::patterns::{LastUsed, RailTool, ToolRail, ToolSlot};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{Key, Modifiers, Ui};

use super::model::{Colors, Mock, Tool};

/// The tool's icon.
pub fn icon(tool: Tool) -> Icon {
    match tool {
        Tool::Move => Icon::Move,
        Tool::Artboard => Icon::Artboard,
        Tool::RectMarquee => Icon::MarqueeRect,
        Tool::EllipseMarquee => Icon::MarqueeEllipse,
        Tool::Lasso => Icon::Lasso,
        Tool::PolygonLasso => Icon::LassoSelect,
        Tool::MagneticLasso => Icon::LassoMagnetic,
        Tool::Crop => Icon::Crop,
        Tool::PerspectiveCrop => Icon::PerspectiveCrop,
        Tool::Eyedropper => Icon::Eyedropper,
        Tool::Ruler => Icon::Ruler,
        Tool::Brush => Icon::Brush,
        Tool::Pencil => Icon::Pencil,
        Tool::Eraser => Icon::Eraser,
        Tool::BackgroundEraser => Icon::EraserBackground,
        Tool::Gradient => Icon::Gradient,
        Tool::PaintBucket => Icon::PaintBucket,
        Tool::Pen => Icon::PenTool,
        Tool::FreeformPen => Icon::PenFreeform,
        Tool::Type => Icon::Text,
        Tool::Hand => Icon::Hand,
        Tool::RotateView => Icon::RotateCw,
        Tool::Zoom => Icon::ZoomIn,
    }
}

fn rail_tool(tool: Tool) -> RailTool<Tool> {
    let rail = RailTool::new(tool, icon(tool), tool.label()).enabled(tool.is_working());
    match tool.key() {
        Some(key) => rail.shortcut(key.to_string()),
        None => rail,
    }
}

/// The rail's slots, grouped as PhotoCraft groups them. Tools the mock does not implement fill
/// their flyouts disabled.
fn slots() -> Vec<ToolSlot<Tool>> {
    use Tool::{
        Artboard, BackgroundEraser, Brush, Crop, EllipseMarquee, Eraser, Eyedropper, FreeformPen, Gradient, Hand, Lasso, MagneticLasso, Move,
        PaintBucket, Pen, Pencil, PerspectiveCrop, PolygonLasso, RectMarquee, RotateView, Ruler, Type, Zoom,
    };
    let groups: [&[Tool]; 12] = [
        &[Move, Artboard],
        &[RectMarquee, EllipseMarquee],
        &[Lasso, PolygonLasso, MagneticLasso],
        &[Crop, PerspectiveCrop],
        &[Eyedropper, Ruler],
        &[Brush, Pencil],
        &[Eraser, BackgroundEraser],
        &[Gradient, PaintBucket],
        &[Pen, FreeformPen],
        &[Type],
        &[Hand, RotateView],
        &[Zoom],
    ];
    groups.iter().enumerate().map(|(i, tools)| ToolSlot::new(i as u32, tools.iter().map(|t| rail_tool(*t)).collect())).collect()
}

fn srgba(c: [u8; 3]) -> Srgba {
    let [r, g, b] = c.map(|v| f32::from(v) / 255.0);
    Srgba::rgb(r, g, b)
}

fn rgb8(c: Srgba) -> [u8; 3] {
    [c.r, c.g, c.b].map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Draws the rail at the start edge and applies what was chosen in it.
pub fn rail(ui: &mut Ui, mock: &mut Mock, last_used: &mut LastUsed<Tool>) {
    let slots = slots();
    let numbers = LocaleNumbers::from_tag("en-US");
    let backdrop = Tokens::of(ui).component.rail.fill.to_egui();
    let colors = mock.colors;
    let mut after = colors;
    let out = ToolRail::new("tools").show(ui, &slots, &mock.tools.current, last_used, |ui, _| {
        let wells = [PairWell::new("Foreground colour", srgba(colors.fg)), PairWell::new("Background colour", srgba(colors.bg))];
        let active = if colors.bg_active { Well::Secondary } else { Well::Primary };
        let pair = SwatchPair::new("colours", "Colours", wells, &numbers)
            .active(active)
            .swap_button("Swap colours")
            .default_button("Default colours")
            .backdrop(backdrop)
            .show(ui);
        after.bg_active = pair.active == Well::Secondary;
        if pair.swap {
            after.swap();
        }
        if pair.defaults {
            after = Colors { bg_active: after.bg_active, ..Colors::DEFAULT };
        }
        if let Some(picked) = pair.edit.event.as_ref().and_then(|e| e.value()) {
            let color = rgb8(picked.color);
            *(if pair.well == Well::Secondary { &mut after.bg } else { &mut after.fg }) = color;
        }
    });
    mock.colors = after;
    if let Some(tool) = out.select {
        mock.select_tool(tool);
    }
}

/// The key that chooses a working tool, when no text field has the keyboard.
pub fn handle_keys(ui: &Ui, mock: &mut Mock) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let pressed = ui.input_mut(|input| {
        Tool::WORKING.into_iter().find(|tool| {
            let key = tool.key().and_then(|c| Key::from_name(&c.to_string()));
            key.is_some_and(|key| input.consume_key(Modifiers::NONE, key))
        })
    });
    if let Some(tool) = pressed {
        mock.select_tool(tool);
    }
}
