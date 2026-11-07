//! `StyleCx`: what `style` and the app colour hooks get. Tokens are readable only through it.

use super::key::{EnvKey, Ink};
use super::look::{StyleColor, SurfaceKind};
use super::paint::{HandleLook, ResetMarkLook, RuleKind, RuleLook, ScrollBarLook, handle_look, reset_mark_look, rule_look, scroll_bar_look};
use super::surface::{SurfaceStyle, surface_style};
use super::tokens::UiTokens;
use crate::runtime::egui::memory::Service;
use crate::runtime::env::{AppValues, Scale, StyleValue};
use craft_behavior::ControlMetrics;
use craft_host::target::Os;
use craft_i18n::Direction;
use craft_theme::{BySize, ControlSize, RadiusStep, Rgba, Stateful, States, TokenGroup, Variant, VariantLook, variant::Tone};
use egui::{Color32, Context, CornerRadius};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// What `fn style` gets. Tokens are read here and nowhere else.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct StyleCx<'a> {
    /// The resolved tokens of the scope.
    pub tokens: &'a UiTokens,
    /// The control size.
    pub size: ControlSize,
    /// The variant, one the component accepts.
    pub variant: Variant,
    /// The layout direction.
    pub direction: Direction,
    /// The interface scale.
    pub scale: Scale,
    /// The operating system.
    pub os: Os,
    /// Whether the window is active, so selection and focus looks dim in an inactive window.
    pub active: bool,
    /// The key of everything above that the style cache depends on.
    pub key: EnvKey,
    ctx: &'a Context,
    values: &'a AppValues,
}

/// A variant's look as style parts.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct VariantParts {
    /// The fill per state.
    pub fill: Stateful<StyleColor>,
    /// The text and icon colour per state.
    pub text: Stateful<StyleColor>,
    /// The edge per state.
    pub edge: Option<Stateful<StyleColor>>,
    /// The corner radius.
    pub radius: CornerRadius,
}

/// Token groups built for a token set, and the type behind each group path.
#[derive(Default)]
struct GroupCache {
    built: HashMap<(TypeId, u64), Arc<dyn Any + Send + Sync>>,
    paths: Vec<(&'static str, TypeId)>,
}

fn egui_color(c: Rgba) -> Color32 {
    let [r, g, b, a] = c.0;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

impl<'a> StyleCx<'a> {
    /// A style context for the scope the pipeline or an app colour hook runs in.
    #[expect(clippy::too_many_arguments, reason = "one argument per input the style cache key covers")]
    pub(crate) fn new(
        ctx: &'a Context,
        tokens: &'a UiTokens,
        values: &'a AppValues,
        size: ControlSize,
        variant: Variant,
        direction: Direction,
        scale: Scale,
        os: Os,
        active: bool,
        key: EnvKey,
    ) -> Self {
        StyleCx { tokens, size, variant, direction, scale, os, active, key, ctx, values }
    }

    /// The same context with another variant.
    pub(crate) fn with_variant(&self, variant: Variant) -> StyleCx<'a> {
        StyleCx { variant, ..*self }
    }

    /// An app value set by a scope. Its hash is part of the style cache key.
    pub fn env<T: StyleValue>(&self) -> T {
        debug_assert!(
            T::AFFECTS_STYLE,
            "{} is read by style but does not set AFFECTS_STYLE, so cached styles would go stale",
            std::any::type_name::<T>()
        );
        self.values.get::<T>()
    }

    /// The one constructor of `StyleColor`: a semantic token read or a token-group field becomes a style colour here.
    pub fn color(&self, color: Rgba) -> StyleColor {
        StyleColor(egui_color(color))
    }

    /// The colour text and icons of `tone` read in.
    pub fn tone(&self, tone: Tone) -> StyleColor {
        let c = &self.tokens.color;
        self.color(match tone {
            Tone::Accent => c.accent.text,
            Tone::Info => c.status.info.fg,
            Tone::Good => c.status.good.fg,
            Tone::Warn => c.status.warn.fg,
            Tone::Bad => c.status.bad.fg,
            _ => c.text.normal,
        })
    }

    /// The colour text and icons on a surface read in.
    pub fn ink(&self, ink: Ink) -> StyleColor {
        let c = &self.tokens.color.text;
        self.color(match ink {
            Ink::OnPanel => c.normal,
            Ink::OnDark => c.on_strong,
        })
    }

    /// The height, icon side, padding and gap of a control at the ambient size.
    pub fn control(&self) -> ControlMetrics {
        let size = &self.tokens.size;
        let pad = self.tokens.space.pad.control;
        let gap = self.tokens.space.gap.inline;
        ControlMetrics::at(
            self.size,
            &BySize { sm: size.control.sm, md: size.control.md },
            &BySize { sm: size.icon.sm, md: size.icon.md },
            &BySize { sm: pad, md: pad },
            &BySize { sm: gap, md: gap },
        )
    }

    /// What a surface of `kind` draws.
    pub fn surface(&self, kind: SurfaceKind) -> SurfaceStyle {
        surface_style(self.tokens, kind)
    }

    /// The line a rule of `kind` draws.
    pub fn rule(&self, kind: RuleKind) -> RuleLook {
        rule_look(self.tokens, kind)
    }

    /// The look of a canvas handle.
    pub fn handle(&self) -> HandleLook {
        handle_look(self.tokens)
    }

    /// The look of a scroll bar.
    pub fn scroll_bar(&self) -> ScrollBarLook {
        scroll_bar_look(self.tokens)
    }

    /// The look of the reset mark.
    pub fn reset_mark(&self) -> ResetMarkLook {
        reset_mark_look(self.tokens)
    }

    /// The look of the instance's variant as style parts.
    pub fn variant_look(&self) -> VariantParts {
        let look: VariantLook = self.variant.resolve(self.tokens);
        let states = |s: States<Rgba>| {
            let c = |c: Rgba| self.color(c);
            Stateful::new(c(s.rest)).off(States::new(c(s.rest), c(s.hover), c(s.pressed), c(s.disabled)))
        };
        let radius = &self.tokens.shape.radius;
        let px = match look.radius {
            RadiusStep::Button => radius.button,
            RadiusStep::Pill => radius.pill,
            _ => radius.control,
        };
        VariantParts { fill: states(look.fill), text: states(look.text), edge: look.edge.map(states), radius: UiTokens::corner(px) }
    }

    /// The token group `G` for this token set: derived from the tokens with the theme's values under `G::PATH` laid
    /// over it, built on the first read and cached by `(TypeId::of::<G>(), UiTokens::key)`.
    pub fn token_group<G: TokenGroup>(&self) -> G {
        let service = Service::<GroupCache>::of(self.ctx);
        let cache_key = (TypeId::of::<G>(), self.tokens.key());
        let hit = service.with(|cache| cache.built.get(&cache_key).cloned());
        if let Some(hit) = hit.and_then(|entry| entry.downcast::<G>().ok()) {
            return (*hit).clone();
        }
        let clash = service.with(|cache| {
            let clash = cache.paths.iter().any(|(path, ty)| *path == G::PATH && *ty != TypeId::of::<G>());
            if !clash && !cache.paths.iter().any(|(_, ty)| *ty == TypeId::of::<G>()) {
                cache.paths.push((G::PATH, TypeId::of::<G>()));
            }
            clash
        });
        debug_assert!(!clash, "two token group types share the path {}", G::PATH);
        let mut group = G::derive(self.tokens);
        group.overlay(self.tokens.groups());
        // The clone is app code, so it runs before the service lock is taken.
        let shared: Arc<dyn Any + Send + Sync> = Arc::new(group.clone());
        service.with(|cache| cache.built.insert(cache_key, shared));
        group
    }
}
