//! [`ColorPopover`]: a swatch that opens a Compact colour picker in a popup.

use craft_theme::{ControlSize, Srgba};
use egui::accesskit::{HasPopup, Role};
use egui::{AsIdSalt, Id, IdSalt, Key, LayerId, Modifiers, Order, PopupCloseBehavior, Response, Ui};

use super::hex::hex_id;
use super::{ColorArea, ColorPicker, ColorPickerLayout, ColorPickerResponse, PickedColor};
use crate::controls::Swatch;
use crate::edit::{EditEvent, EditResponse};
use crate::localized::LocaleNumbers;
use crate::popup_list::PopupList;
use crate::primitives::{Gap, Interactive, Surface, SurfaceKind};
use crate::theme::{Tokens, TokensOf};

/// A [`ColorPicker`] (Compact) in a popup under a [`Swatch`], as PhotoCraft's colour button:
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::ColorPopover;
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.2, 0.5, 0.8);
/// let numbers = LocaleNumbers::from_tag("en");
/// let popover = ColorPopover::new("fill", "Fill colour", color, &numbers).show(ui);
/// if let Some(p) = popover.edit.event.as_ref().and_then(|e| e.value()) {
///     color = p.color;
/// }
/// # });
/// ```
///
/// - Opens on a click, Enter or Space on the swatch; the hex field takes the focus with its text
///   selected. The popup opens below the swatch, or above it when there is no room, and keeps
///   that side while it is open; it is as wide as the picker's area and strip.
/// - Closes on a click outside it, on a click on the swatch, and on Esc while no gesture is
///   under way (an Esc that cancels a gesture is the picker's). The colour stays as edited. The
///   focus goes back to the swatch unless it moved to another control.
/// - Edits are the picker's: one step per gesture, so the popup edits live as PhotoCraft's does. A
///   host that wants the popup as one step wraps it in [`edit::scope`](crate::edit::scope).
/// - AccessKit: the swatch is a `ColorWell` named `label` with the colour as value, expanded
///   while the popup shows, and the popup is a `Dialog` holding the picker.
/// - While open the popup is registered with the other popups, so Tab and Esc readers outside it
///   leave their keys alone.
#[must_use = "show the popover with `show`"]
pub struct ColorPopover<'a> {
    id_salt: IdSalt,
    label: String,
    color: Srgba,
    numbers: &'a LocaleNumbers,
    size: ControlSize,
    dimensions: Option<(f32, f32)>,
    enabled: bool,
    picker: Option<Box<dyn FnOnce(ColorPicker<'a>) -> ColorPicker<'a> + 'a>>,
}

/// What [`ColorPopover::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event"]
pub struct ColorPopoverResponse {
    /// The edit of the picker in the popup; the response is the swatch's. Without an event while
    /// the popup is closed.
    pub edit: EditResponse<PickedColor>,
    /// The picker's eyedropper button was pressed ([`ColorPickerResponse`](super::ColorPickerResponse)).
    /// The result goes to [`ColorPicker::sampled`] with [`picker_id`](Self::picker_id).
    pub eyedropper_requested: bool,
    /// The id of the picker in the popup, once it has been shown.
    pub picker_id: Option<Id>,
    /// Whether the popup is open after this frame.
    pub open: bool,
}

impl<'a> ColorPopover<'a> {
    /// A swatch named `label` (the swatch's accessible name, translated: "Fill colour") of the
    /// host's `color`, opening a picker that writes and reads its fields with `numbers`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, color: Srgba, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            color,
            numbers,
            size: ControlSize::Md,
            dimensions: None,
            enabled: true,
            picker: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A well `width` wide and `height` high in place of the square of [`size`](ColorPopover::size)
    /// ([`Swatch::dimensions`]).
    pub fn dimensions(mut self, width: f32, height: f32) -> Self {
        self.dimensions = Some((width, height));
        self
    }

    /// Whether the swatch takes input; a disabled one never opens.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets up the picker in the popup (alpha, eyedropper, constraint, texts, models): `setup`
    /// gets the Compact picker the popover made.
    pub fn picker(mut self, setup: impl FnOnce(ColorPicker<'a>) -> ColorPicker<'a> + 'a) -> Self {
        self.picker = Some(Box::new(setup));
        self
    }

    pub fn show(self, ui: &mut Ui) -> ColorPopoverResponse {
        let Self { id_salt, label, color, numbers, size, dimensions, enabled, picker: setup } = self;
        let enabled = enabled && ui.is_enabled();
        let mut well = Swatch::new(id_salt, label.as_str(), color.to_rgba()).size(size).enabled(enabled);
        if let Some((width, height)) = dimensions {
            well = well.dimensions(width, height);
        }
        let swatch = ui.add(well);
        let toggled = swatch.clicked();
        popup(ui, swatch, toggled, &label, color, numbers, enabled, setup)
    }
}

const STATE: &str = "craft_ui::color_popover";

/// The popup of a colour popover under `anchor`, which the caller has drawn: `toggled` opens it
/// when closed and closes it when open (the anchor's click, or another gesture of the caller's).
/// The response's edit carries `anchor`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn popup<'a, 'b>(
    ui: &mut Ui,
    swatch: Response,
    toggled: bool,
    label: &str,
    color: Srgba,
    numbers: &'a LocaleNumbers,
    enabled: bool,
    setup: Option<Box<dyn FnOnce(ColorPicker<'a>) -> ColorPicker<'a> + 'b>>,
) -> ColorPopoverResponse {
    let ctx = ui.ctx().clone();
    let state_id = swatch.id.with(STATE);
    let stored = ctx.data(|d| d.get_temp::<Open>(state_id)).filter(|_| enabled).unwrap_or_default();
    let was_open = stored.open;
    let mut open = was_open;
    let mut fresh = stored.fresh;
    // A frame may run in two passes (the popup's first is measured, then discarded); the
    // click shows in both.
    let frame = ctx.cumulative_frame_nr();
    if toggled && enabled {
        if stored.clicked != Some(frame) {
            open = !open;
            fresh = open;
        }
        if open {
            // The key that opened the popup is not the hex field's, which takes the focus at once.
            ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter);
                i.consume_key(Modifiers::NONE, Key::Space);
            });
        }
    }
    let clicked = if toggled { Some(frame) } else { stored.clicked };

    let popup = egui::Popup::menu(&swatch);
    let popup_id = popup.get_id();
    let tokens = Tokens::of(ui);
    let parts = ColorArea::style(&tokens);
    let width = parts.area + Gap::Inline.points(&tokens) + parts.strip;

    let mut shown = None;
    PopupList::with_popup(popup, ui, &swatch)
        .open(&mut open)
        .focus_owners(&[swatch.id])
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .height(f32::INFINITY)
        .surface(Surface::style(&tokens, SurfaceKind::Popup))
        .show_ui(|ui| {
            ui.set_width(width);
            let picker_id = Interactive::id_in(ui, "picker");
            let hex = hex_id(picker_id);
            if fresh {
                ColorPicker::focus_hex(ui.ctx(), picker_id);
            }
            let mut picker = ColorPicker::new("picker", label, color, numbers).layout(ColorPickerLayout::Compact).enabled(enabled);
            if let Some(setup) = setup {
                picker = setup(picker);
            }
            let response = picker.show(ui);
            // A gesture the picker cancelled with Esc is not a request to close.
            if matches!(response.edit.event, Some(EditEvent::Cancel { .. })) {
                ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }
            // The request waits for the pass that is shown; ask until the field has the focus.
            fresh &= !ui.memory(|m| m.has_focus(hex));
            shown = Some(response);
        });
    ctx.data_mut(|d| d.insert_temp(state_id, Open { open, fresh: fresh && open, clicked }));
    if was_open && !open && focus_free_or_inside(&ctx, popup_id) {
        ctx.memory_mut(|m| m.request_focus(swatch.id));
    }
    ctx.accesskit_node_builder(swatch.id, |node| {
        node.set_has_popup(HasPopup::Dialog);
        node.set_expanded(open);
    });
    if open {
        ctx.accesskit_node_builder(popup_id, |node| node.set_role(Role::Dialog));
    }
    match shown {
        Some(ColorPickerResponse { edit, eyedropper_requested, .. }) => {
            let picker_id = Some(edit.response.id);
            ColorPopoverResponse { edit: EditResponse::new(swatch, edit.event), eyedropper_requested, picker_id, open }
        }
        None => ColorPopoverResponse { edit: EditResponse::new(swatch, None), eyedropper_requested: false, picker_id: None, open },
    }
}

/// Closes the popup of the popover anchored on `anchor`, for a caller that stops drawing the
/// anchor while the popup is open.
pub(crate) fn close(ctx: &egui::Context, anchor: Id) {
    ctx.data_mut(|d| d.remove::<Open>(anchor.with(STATE)));
}

/// The popover's state between frames.
#[derive(Clone, Copy, Default)]
struct Open {
    open: bool,
    /// The popup opened and its hex field has not yet taken the focus.
    fresh: bool,
    /// The frame of the last click on the swatch.
    clicked: Option<u64>,
}

/// Whether no control has the focus, or the focused one is in the popup `popup`: a close that
/// leaves the focus there gives it back to the swatch.
fn focus_free_or_inside(ctx: &egui::Context, popup: Id) -> bool {
    let Some(focused) = ctx.memory(|m| m.focused()) else { return true };
    ctx.read_response(focused).is_none_or(|r| r.layer_id == LayerId::new(Order::Foreground, popup))
}
