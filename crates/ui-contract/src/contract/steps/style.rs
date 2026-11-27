//! Step 2, style: the component's style from the window's style cache (contract-rt 15.7).

use super::env::EnvStep;
use crate::contract::Component;
use crate::runtime::env::Env;
use crate::style::{StyleCaches, StyleCx, resolve_style};
use craft_ui_rt::BuildCx;
use std::sync::Arc;

/// The `StyleCx` of the env the component shows in.
pub(crate) fn style_cx<'a>(env: &'a Env, step: &EnvStep) -> StyleCx<'a> {
    StyleCx::new(&env.tokens, &env.values, step.size, step.requested, env.direction, env.scale, env.os, env.active, env.key, env.line_ratio)
        .with_glyphs(step.glyphs)
}

/// Step 2, style: `&C::Style` from the window root's cache, keyed `(TypeId of StyleKey, key, EnvKey,
/// variant, size)`. Outside a window (a test without a root) the style resolves uncached.
pub(crate) fn style<C: Component>(cx: &mut BuildCx<'_>, scx: &StyleCx<'_>, key: C::StyleKey) -> Arc<C::Style> {
    match cx.provided::<StyleCaches>() {
        Some(caches) => resolve_style(scx, key, caches, C::style),
        None => Arc::new(C::style(scx, key)),
    }
}

/// The style of a face, which runs inside its host's paint with no build at hand: resolved through
/// the cache the host's `StyleCx` carries.
pub(crate) fn style_cached<C: Component>(scx: &StyleCx<'_>, key: C::StyleKey) -> Arc<C::Style> {
    match scx.caches() {
        Some(caches) => resolve_style(scx, key, caches, C::style),
        None => Arc::new(C::style(scx, key)),
    }
}
