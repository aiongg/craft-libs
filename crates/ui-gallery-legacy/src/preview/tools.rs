//! The tool rail at the start edge and the tool keys.

use craft_ui::icons::Icon;
use craft_ui::menu;
use craft_ui::patterns::{LastUsed, RailTool, ToolRail, ToolSlot};
use craft_ui::popover;
use egui::{Event, Modifiers, Popup, Ui};

use super::model::{Mock, Tool};

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

/// Draws the rail at the start edge and applies what was chosen in it.
pub fn rail(ui: &mut Ui, mock: &mut Mock, last_used: &mut LastUsed<Tool>) {
    let slots = slots();
    let out = ToolRail::new("tools").show(ui, &slots, &mock.tools.current, last_used, |_, _| {});
    if let Some(tool) = out.select {
        mock.select_tool(tool);
    }
}

/// The key that chooses a working tool. It is left alone while a menu, a popup, a popover or the
/// dialog is open, the keyboard is on the menu bar, or a text field is being edited; with any
/// modifier it is a shortcut of its own.
pub fn handle_keys(ui: &Ui, mock: &mut Mock) {
    let ctx = ui.ctx();
    let busy = menu::any_open(ctx)
        || menu::bar::is_focused(ctx)
        || Popup::is_any_open(ctx)
        || popover::any_open(ctx)
        || craft_ui::dialog::any_modal(ctx)
        || ctx.text_edit_focused();
    if busy {
        return;
    }
    let tool = ui.input_mut(|input| {
        let pressed = input.events.iter().find_map(|event| match event {
            Event::Key { key, pressed: true, repeat: false, modifiers, .. } if modifiers.is_none() => Some(*key),
            _ => None,
        })?;
        let name = pressed.name();
        let tool = Tool::from_key(name.chars().next().filter(|_| name.len() == 1)?)?;
        input.consume_key(Modifiers::NONE, pressed);
        Some(tool)
    });
    if let Some(tool) = tool {
        mock.select_tool(tool);
    }
}
