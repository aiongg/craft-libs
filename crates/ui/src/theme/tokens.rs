//! [`UiTokens`]: resolved tokens with the egui values components use most.

use std::ops::Deref;
use std::sync::Arc;

use craft_theme::{Appearance, Mode, Tokens, TypeRole};
use egui::{CornerRadius, FontId, Shadow, Style, Ui};

use super::convert::{FontFamilies, ToEgui, corner_radius};
use super::style::style_for;

/// Resolved tokens for a part of the UI, from [`Tokens::of`](TokensOf::of).
///
/// Derefs to the [`Tokens`]; colours convert with [`ToEgui`] (`t.color.text.normal.to_egui()`).
/// Fonts, radii, shadows and the egui [`Style`] are converted once, here.
#[derive(Debug)]
pub struct UiTokens {
    tokens: Arc<Tokens>,
    /// The appearance the tokens were resolved for.
    pub appearance: Appearance,
    /// The variant shown: differs from `appearance.mode` for a single-variant theme.
    pub mode: Mode,
    /// [`style_for`] the tokens.
    pub style: Arc<Style>,
    pub radius: Radii,
    pub shadow: Shadows,
    families: FontFamilies,
    fonts: [FontId; 7],
}

/// `shape.radius` as egui radii.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radii {
    pub control: CornerRadius,
    pub popup: CornerRadius,
    pub window: CornerRadius,
    pub panel: CornerRadius,
    /// Push buttons: `button.radius`, which is `shape.radius.button` unless the theme overrides
    /// it.
    pub button: CornerRadius,
    /// Pill-shaped parts: `shape.radius.pill`, to clamp to half the part's height.
    pub pill: CornerRadius,
}

/// `elevation` as egui shadows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadows {
    pub popup: Shadow,
    pub dialog: Shadow,
    pub floating: Shadow,
    pub knob: Shadow,
}

const ROLES: [TypeRole; 7] =
    [TypeRole::Caption, TypeRole::Label, TypeRole::Body, TypeRole::BodyStrong, TypeRole::Title, TypeRole::Heading, TypeRole::Mono];

impl UiTokens {
    pub fn new(tokens: Tokens, appearance: Appearance, mode: Mode, families: FontFamilies) -> Self {
        let r = &tokens.shape.radius;
        let radius = Radii {
            control: corner_radius(r.control),
            popup: corner_radius(r.popup),
            window: corner_radius(r.window),
            panel: corner_radius(r.panel),
            button: corner_radius(tokens.component.button.radius),
            pill: corner_radius(r.pill),
        };
        let e = &tokens.elevation;
        let shadow = Shadows { popup: e.popup.to_egui(), dialog: e.dialog.to_egui(), floating: e.floating.to_egui(), knob: e.knob.to_egui() };
        let fonts = ROLES.map(|role| families.font_id(tokens.typography.get(role)));
        let style = Arc::new(style_for(&tokens, &families));
        Self { tokens: Arc::new(tokens), appearance, mode, style, radius, shadow, families, fonts }
    }

    /// The font of a type role.
    pub fn font(&self, role: TypeRole) -> FontId {
        let index = ROLES.iter().position(|r| *r == role).unwrap_or(0);
        self.fonts[index].clone()
    }

    /// The egui families the font slots map to.
    pub fn families(&self) -> &FontFamilies {
        &self.families
    }

    /// The resolved tokens, shared.
    pub fn tokens(&self) -> &Arc<Tokens> {
        &self.tokens
    }
}

impl Deref for UiTokens {
    type Target = Tokens;
    fn deref(&self) -> &Tokens {
        &self.tokens
    }
}

/// `Tokens::of(ui)`: the tokens in effect where `ui` draws.
pub trait TokensOf {
    /// The innermost [`scope`](super::scope) around `ui` in its viewport, else the
    /// [`install`](super::install)-ed base for the context's theme, else the suite default theme
    /// in that mode.
    fn of(ui: &Ui) -> Arc<UiTokens>;
}

impl TokensOf for Tokens {
    fn of(ui: &Ui) -> Arc<UiTokens> {
        super::bridge::current(ui.ctx())
    }
}
