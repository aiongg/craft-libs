//! The canvas controls: leaves a `Canvas` body shows (`#[component(canvas_control)]`).

mod words;

use crate::runtime::egui::memory::Service;
use egui::Context;

/// The types declared as canvas controls that showed in a pass, so the canvas-control cases can check a press away
/// from their parts reaches the tool.
#[derive(Default)]
struct Declared(Vec<&'static str>);

/// Records that the canvas control `type_name` showed in this pass.
pub(crate) fn declare(ctx: &Context, type_name: &'static str) {
    Service::<Declared>::of(ctx).with(|declared| {
        if !declared.0.contains(&type_name) {
            declared.0.push(type_name);
        }
    });
}

/// The canvas-control types that have shown on the context.
pub fn shown(ctx: &Context) -> Vec<&'static str> {
    Service::<Declared>::of(ctx).with(|declared| declared.0.clone())
}
