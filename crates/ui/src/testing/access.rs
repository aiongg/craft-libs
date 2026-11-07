//! What the accessibility trials read: the AccessKit tree a frame produced, and the label scope a container opens.

use crate::runtime::access::{FieldLabel, scope};
use egui::{Context, Id};

pub use crate::runtime::egui::access::{AccessTree, Declared, NodeKey};

/// Opens the label scope of `container` with `label`, as a field container's body step does; the container's
/// report step closes it.
pub fn begin_label_scope(ctx: &Context, container: Id, label: FieldLabel) {
    scope::begin(ctx, container, container, scope::Kind::Label(label));
}
