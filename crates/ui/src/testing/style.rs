//! Style and paint helpers: token sets, a `StyleCx`, the style cache and a `Draw` over a frame.

use super::Frames;
use crate::primitives::{Draw, DrawEnv, PaintRecord};
use crate::runtime::env::{AppValues, Scale};
use craft_host::target::Os;
use crate::style::{ComponentStyle, EnvKey, Ink, StyleCx, StyleKey, UiTokens, resolve_style};
use craft_i18n::Direction;
use craft_theme::{Appearance, Contrast, ControlSize, Mode, Size, TextSize, Variant, builtins, resolve};
use egui::{Context, Shape};
use std::cell::RefCell;
use std::sync::Arc;

/// The suite's default built-in theme resolved for `mode` and `contrast`.
pub fn tokens(mode: Mode, contrast: Contrast) -> UiTokens {
    tokens_named("artcraft", mode, contrast)
}

/// The built-in theme `craft.<name>` resolved for `mode` and `contrast`.
pub fn tokens_named(name: &str, mode: Mode, contrast: Contrast) -> UiTokens {
    let id = format!("craft.{name}");
    let index = builtins().iter().position(|theme| theme.id.as_str() == id);
    assert!(index.is_some(), "no built-in theme {id}");
    let index = index.unwrap_or_default();
    let appearance = Appearance { mode, contrast, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false };
    let resolved = resolve(&builtins()[index], appearance, None);
    UiTokens::new(Arc::new(resolved.tokens))
}

/// The `EnvKey` of `tokens` in a left-to-right Linux window at scale 1 with no style-affecting app values.
pub fn env_key(tokens: &UiTokens) -> EnvKey {
    EnvKey::new(tokens.key(), Direction::Ltr, Scale::default(), Os::Linux, true, 0)
}

/// Runs `f` with a `StyleCx` over `tokens` in a left-to-right Linux window at scale 1.
pub fn with_cx<R>(ctx: &Context, tokens: &UiTokens, variant: Variant, size: ControlSize, f: impl FnOnce(&StyleCx<'_>) -> R) -> R {
    let values = AppValues::default();
    let cx = StyleCx::new(ctx, tokens, &values, size, variant, Direction::Ltr, Scale::default(), Os::Linux, true, env_key(tokens));
    f(&cx)
}

/// The cached style for `key` in `cx`, built by `default` on a miss.
pub fn style_of<K: StyleKey, S: ComponentStyle>(ctx: &Context, cx: &StyleCx<'_>, key: K, default: for<'a> fn(&StyleCx<'a>, K) -> S) -> Arc<S> {
    resolve_style(cx, key, ctx, default)
}

/// The base id of the face painted for `part` of the composite `id`.
pub fn face_id(composite: &str, part: craft_behavior::PartRef) -> egui::Id {
    crate::primitives::face_id(crate::runtime::egui::memory::unique(composite), part)
}

/// What `Draw` recorded in one frame.
#[derive(Debug, Default, Clone, PartialEq)]
#[non_exhaustive]
pub struct Recorded {
    /// The full text of the first elided run.
    pub elided: Option<String>,
    /// The first baseline.
    pub baseline: Option<f32>,
}

/// Runs one frame in which `paint` draws through a `Draw` over the root `Ui`'s painter, in `direction` and on
/// `ink`. Returns the shapes painted and what `Draw` recorded.
pub fn paint_frame(
    frames: &mut Frames,
    tokens: &UiTokens,
    direction: Direction,
    ink: Ink,
    mut paint: impl FnMut(&Draw<'_>),
) -> (Vec<Shape>, Recorded) {
    let record = RefCell::new(PaintRecord::default());
    let shapes = frames.frame_shapes(|ui| {
        let draw = Draw::new(ui.painter(), &record, DrawEnv { tokens, direction, ink });
        paint(&draw);
    });
    let record = record.into_inner();
    (shapes, Recorded { elided: record.elided, baseline: record.baseline })
}
