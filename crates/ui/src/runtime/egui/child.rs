//! Child `Ui`s: env scopes and gates (`ui.disable()`, a scope's own direction and layout), `new_child`,
//! `UiBuilder::scope_id`, `accessibility_parent`, the egui layout of each `BodyLayout` and sense.

use egui::{Align, Layout, Ui, UiBuilder};

/// The physical layout of a body with no layout of its own: top down, items hugging the reading start.
fn stack_layout(right_to_left: bool) -> Layout {
    Layout::top_down(if right_to_left { Align::Max } else { Align::Min })
}

/// Runs `body` in a child `Ui`. `disabled` disables the child and everything below it. `direction` gives the
/// child a cloned style carrying that direction and an explicit layout, never the parent's.
pub(crate) fn scope<R>(ui: &mut Ui, disabled: bool, direction: Option<bool>, body: impl FnOnce(&mut Ui) -> R) -> R {
    let mut builder = UiBuilder::new();
    if let Some(right_to_left) = direction {
        let mut style = (**ui.style()).clone();
        style.right_to_left = right_to_left;
        builder = builder.layout(stack_layout(right_to_left)).style(style);
    }
    ui.scope_builder(builder, |child| {
        if disabled {
            child.disable();
        }
        body(child)
    })
    .inner
}

/// Runs `f` on a child `Ui` of `ui`.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "used by F2 pipeline"))]
pub(crate) fn scoped<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(f).inner
}
