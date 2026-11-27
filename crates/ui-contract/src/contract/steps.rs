//! The step functions of the pipeline (1.6): named functions over plain data, in pipeline order. The driver
//! (`contract/rt/run.rs`) calls them for every run.

mod apply;
mod env;
mod finish;
mod label;
mod measure;
mod paint;
mod parts;
pub(crate) mod report;
mod sense;
mod settle;
mod state;
mod style;
mod update;
mod value;

pub(crate) use apply::{ValueEdit, apply_values};
pub(crate) use env::{EnvStep, env, resolved, variants};
pub(crate) use finish::before_finish;
pub(crate) use label::scrub_by_label;
pub(crate) use measure::{MeasureIn, measure, measure_cx, padding, whole};
pub(crate) use paint::{Fade, UnderVeil, fade, paint_part_layers, surface_ink, surface_items, unfade};
pub(crate) use parts::{PartsStep, declare_parts, declared_with, focus_shapes, inactive, kept_parts, toggles};
pub(crate) use report::validity_of;
pub(crate) use sense::part_state;
pub(crate) use settle::settle;
pub(crate) use state::{ResetFacts, register_reset, shown_bool, state, value_flags};
pub(crate) use style::{style, style_cached, style_cx};
pub(crate) use update::{ContextRequest, context_slot, edit_intents, gesture_gate, record_context, takes_activation, takes_focus, update_inputs};
pub(crate) use value::{Shown, ShownState, ValueStep, value};
