//! The focus service's fixture: egui widgets for stops, the egui focus, and the events the key and press
//! stages read.

use crate::runtime::egui::{fixture, focus as egui_focus};
use crate::runtime::focus::PressShapeRecord;
use craft_behavior::focus::PressShape;
use egui::{Context, Event, Id, Key, Modifiers, Pos2, Rect, Ui};

/// Shows a focusable egui widget of `id`, as a component root's sensing core does, and returns its rect.
pub fn widget(ui: &mut Ui, id: Id) -> Rect {
    fixture::focusable_widget(ui, id)
}

/// The widget egui holds key focus on.
pub fn egui_focused(ctx: &Context) -> Option<Id> {
    egui_focus::focused(ctx)
}

/// A shape that senses a press at `rect` on the layer of `ui`, owned by `owner`.
pub fn press_shape(ui: &Ui, rect: Rect, shape: PressShape, owner: Id) -> PressShapeRecord {
    PressShapeRecord::new(fixture::layer_of(ui), rect, shape, owner)
}

/// A primary button press or release at `pos`.
pub fn primary_button(pos: Pos2, pressed: bool) -> Event {
    fixture::primary_button(pos, pressed)
}

/// A key release.
pub fn key_release(key: Key, modifiers: Modifiers) -> Event {
    fixture::key_release(key, modifiers)
}

pub use crate::runtime::egui::fixture::AccessActionKind;

/// An AccessKit action request for the node numbered by `target`.
pub fn access_request(kind: AccessActionKind, target: Id) -> Event {
    fixture::access_request(kind, target)
}

/// A globally unique id named by `name`.
pub fn named_id(name: &str) -> Id {
    fixture::named_id(name)
}
