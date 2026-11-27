//! Step 2, style: the component's style from the window's style cache (contract-rt 15.7).

use super::env::EnvStep;
use crate::contract::{Component, Context};
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
pub(crate) fn style<C: Component>(cx: &mut BuildCx<'_>, ctx: &Context, scx: &StyleCx<'_>, key: C::StyleKey) -> Arc<C::Style> {
    let caches = cx.provided::<StyleCaches>();
    // Peeked: the env read that gave the style its key already tracks the scale.
    let scale = *cx.peek_state(cx.env().scale);
    let fonts = crate::fonts::stack_facts(cx);
    let (text, images) = cx.paint_services();
    let services = crate::primitives::FrameServices::new(text, images, scale, fonts);
    let scx = scx.with_services(&services);
    let style = match &caches {
        Some(caches) => resolve_style(&scx.with_caches(caches), key, caches, C::style),
        None => Arc::new(C::style(&scx, key)),
    };
    // A harness reads what its stories resolved (`Harness::styles_of`).
    #[cfg(feature = "testing")]
    crate::testing::style::record::<C>(ctx, &scx, key, &style);
    #[cfg(not(feature = "testing"))]
    let _ = ctx;
    style
}

/// The style of a face, which runs inside its host's paint with no build at hand: resolved through
/// the cache the host's `StyleCx` carries.
pub(crate) fn style_cached<C: Component>(scx: &StyleCx<'_>, key: C::StyleKey) -> Arc<C::Style> {
    match scx.caches() {
        Some(caches) => resolve_style(scx, key, caches, C::style),
        None => Arc::new(C::style(scx, key)),
    }
}
