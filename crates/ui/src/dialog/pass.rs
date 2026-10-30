//! Canvas input under open dialogs. A modal dialog leaves the rest of the window inert (egui's
//! modal layer), but the image can still be panned and zoomed under it, as in Photoshop. These
//! helpers read the raw input outside the rects of the dialogs [`Dialog::show`](super::Dialog::show)
//! recorded.
//!
//! The canvas draws before the dialogs of a pass, so the rects are the ones of the last frame in
//! which dialogs were shown, if that frame is this one or the one before; later they are gone.
//! While any popup is open the helpers report nothing: the popup has the pointer.

use egui::{Context, Id, LayerId, Pos2, Rect, Vec2};

use crate::popup_list;

/// One shown dialog.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub id: Id,
    pub layer: LayerId,
    pub rect: Rect,
    pub modal: bool,
}

#[derive(Clone, Default)]
struct Registry {
    frame: u64,
    current: Vec<Entry>,
    previous: Vec<Entry>,
}

fn registry_id() -> Id {
    Id::unique("craft_ui::dialog::pass")
}

/// Records a dialog shown this pass: its outer rect for the canvas helpers and whether it blocks
/// the window. [`Dialog`](super::Dialog), [`BlockingOverlay`](super::BlockingOverlay) and the
/// full-window dialog call it.
pub(crate) fn record(ctx: &Context, entry: Entry) {
    let now = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| {
        let registry = d.get_temp_mut_or_default::<Registry>(registry_id());
        if registry.frame != now {
            registry.previous = if registry.frame + 1 == now { std::mem::take(&mut registry.current) } else { Vec::new() };
            registry.current.clear();
            registry.frame = now;
        }
        match registry.current.iter_mut().find(|e| e.id == entry.id) {
            Some(slot) => *slot = entry,
            None => registry.current.push(entry),
        }
    });
}

/// The dialogs shown in the latest frame the canvas can know of.
fn visible(ctx: &Context) -> Vec<Entry> {
    let now = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<Registry>(registry_id())).map_or_else(Vec::new, |r| {
        if r.frame == now {
            r.previous
        } else if r.frame + 1 == now {
            r.current
        } else {
            Vec::new()
        }
    })
}

/// The layer a shown dialog was drawn in, from this frame or the last.
pub(super) fn layer_of(ctx: &Context, id: Id) -> Option<LayerId> {
    ctx.data(|d| d.get_temp::<Registry>(registry_id())).and_then(|r| r.current.iter().chain(&r.previous).find(|e| e.id == id).map(|e| e.layer))
}

/// Whether a modal dialog, a full-window dialog or a blocking overlay is open in this viewport:
/// the host's menu and shortcut dispatch waits while it is. Dialogs that took part in the last
/// frame count, and so does any layer egui holds as its modal layer (a full-window dialog), which
/// the background layer is below.
pub fn any_modal(ctx: &Context) -> bool {
    visible(ctx).iter().any(|e| e.modal) || ctx.memory(|m| !m.is_above_modal_layer(LayerId::background()))
}

fn popup_open(ctx: &Context) -> bool {
    egui::Popup::is_any_open(ctx) || popup_list::any_open(ctx)
}

fn is_free(rects: &[Entry], canvas: Rect, p: Pos2) -> bool {
    canvas.contains(p) && !rects.iter().any(|e| e.rect.contains(p))
}

/// The primary button's state on the free canvas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    /// The drag began this pass.
    pub started: bool,
    /// The drag is going on.
    pub dragged: bool,
    /// The button was released after a drag.
    pub stopped: bool,
    /// A click on the free canvas.
    pub clicked: bool,
}

/// Whether the press being held began on a dialog. A dialog follows the pointer while its title
/// is dragged, so the press origin ends up outside the moved dialog and a check against the rects
/// alone would take the drag for a canvas drag. Remembered from the pass of the press, the only
/// one that carries the press event.
fn press_began_on_dialog(ctx: &Context, rects: &[Entry]) -> bool {
    let key = Id::unique("craft_ui::dialog::press-grabbed");
    let pressed = ctx.input(|i| {
        i.events.iter().rev().find_map(|e| match e {
            egui::Event::PointerButton { pos, pressed: true, .. } => Some(*pos),
            _ => None,
        })
    });
    if let Some(p) = pressed {
        let on_dialog = rects.iter().any(|e| e.rect.contains(p));
        ctx.data_mut(|m| m.insert_temp(key, on_dialog));
        return on_dialog;
    }
    ctx.data(|m| m.get_temp::<bool>(key)).unwrap_or(false)
}

/// The pointer when it is over `canvas`, not over a dialog and no popup is open.
pub fn free_pointer_over(ctx: &Context, canvas: Rect) -> Option<Pos2> {
    if popup_open(ctx) {
        return None;
    }
    let p = ctx.pointer_hover_pos()?;
    is_free(&visible(ctx), canvas, p).then_some(p)
}

/// Pan drag under an open dialog: Space-drag, middle-drag or, with `hand`, a plain drag, started
/// on the free canvas. This pass's pointer movement.
pub fn pan_delta(ctx: &Context, canvas: Rect, hand: bool) -> Option<Vec2> {
    let rects = visible(ctx);
    let grabbed = press_began_on_dialog(ctx, &rects);
    let (origin, panning, delta) = ctx.input(|i| {
        let p = &i.pointer;
        (p.press_origin(), p.middle_down() || (p.primary_down() && (hand || i.key_down(egui::Key::Space))), p.delta())
    });
    let origin = origin?;
    (panning && !grabbed && is_free(&rects, canvas, origin)).then_some(delta)
}

/// The primary button's drag and click on the free canvas, as the canvas response would report
/// them if it were not inert: for a held Zoom. A drag counts from egui's drag threshold, like
/// `drag_started`.
pub fn free_buttons(ctx: &Context, canvas: Rect) -> Buttons {
    let id = Id::unique("craft_ui::dialog::free-drag");
    let was = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    let rects = visible(ctx);
    let open_popup = popup_open(ctx);
    let free = |p: Pos2| !open_popup && is_free(&rects, canvas, p);
    let (down, dragging, clicked) = ctx.input(|i| {
        let p = &i.pointer;
        let dragging = p.primary_down() && p.is_decidedly_dragging() && (was || p.press_origin().is_some_and(free));
        (p.primary_down(), dragging, p.primary_clicked() && p.interact_pos().is_some_and(free))
    });
    ctx.data_mut(|d| d.insert_temp(id, dragging));
    Buttons { started: dragging && !was, dragged: dragging, stopped: was && !down, clicked }
}

/// A click or drag with the primary button that began on the free canvas, Space not held: where
/// the pointer is this pass. The press counts in the pass it is made even when released in the
/// same pass (a quick click); a drag counts while the pointer stays on the free canvas. A press
/// that began on a dialog never counts.
pub fn free_press(ctx: &Context, canvas: Rect) -> Option<Pos2> {
    if popup_open(ctx) {
        return None;
    }
    let rects = visible(ctx);
    let free = |p: Pos2| is_free(&rects, canvas, p);
    let grabbed = press_began_on_dialog(ctx, &rects);
    ctx.input(|i| {
        if i.key_down(egui::Key::Space) {
            return None;
        }
        let pressed = i.events.iter().rev().find_map(|e| match e {
            egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, .. } => Some(*pos),
            _ => None,
        });
        if let Some(p) = pressed {
            return free(p).then_some(p);
        }
        let held = i.pointer.primary_down() && !grabbed && i.pointer.press_origin().is_some_and(free);
        i.pointer.latest_pos().filter(|p| held && free(*p))
    })
}
