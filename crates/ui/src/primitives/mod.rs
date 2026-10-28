//! The design system's primitives: the parts every control, composite and shell is drawn with.
//!
//! - [`Surface`]: a semantic surface (panel, raised, sunken, popup, ...) with its fill, border or
//!   bevel, radius and shadow. Only `Surface` reads the theme's edge style.
//! - [`StateLayer`] and [`FocusRing`]: the hover and pressed fills, and the keyboard focus ring.
//! - [`Divider`]: a one-physical-pixel line.
//! - [`Stack`] and [`Row`]: layouts with token gaps and padding.
//! - [`Interactive`]: a hit area with its AccessKit node and keyboard activation; the base of every
//!   control.
//! - [`Text`] and [`Icon`]: text in a type role and an icon at a size class, in a [`Content`]
//!   colour.
//! - [`Handle`]: a canvas handle.
//!
//! Primitives take semantic properties, never sizes or colours, and read their tokens with
//! [`Tokens::of`](crate::theme::TokensOf::of). Each resolves its look with a pure `style`
//! function of [`UiTokens`](crate::theme::UiTokens) and its properties, which tests and the
//! dumps under `crates/ui/tests/dumps/components/` read without a frame.
//!
//! In a disabled `Ui` egui fades everything drawn, primitives included; controls that draw their
//! own disabled state use [`Content::Disabled`] in an enabled `Ui`.
//!
//! ```
//! use craft_theme::TypeRole;
//! use craft_ui::primitives::{Content, Gap, Pad, Stack, Surface, SurfaceKind, Text};
//! # egui::__run_test_ui(|ui| {
//! Surface::new(SurfaceKind::Panel).show(ui, |ui| {
//!     Stack::new(Gap::Stack).pad(Pad::Panel).show(ui, |ui| {
//!         ui.add(Text::new("Layers").role(TypeRole::Title));
//!         ui.add(Text::new("No layers yet").content(Content::Muted));
//!     });
//! });
//! # });
//! ```

pub(crate) mod divider;
pub(crate) mod focus_ring;
pub(crate) mod handle;
pub(crate) mod icon;
pub(crate) mod interactive;
pub(crate) mod stack;
pub(crate) mod state_layer;
pub(crate) mod surface;
#[cfg(test)]
mod tests;
pub(crate) mod text;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod ui_tests;

use craft_theme::{Rgba, Tokens};
use egui::CornerRadius;

pub use divider::{Divider, DividerStyle};
pub use focus_ring::{FocusRing, FocusRingStyle};
pub use handle::{Handle, HandleShape, HandleStyle};
pub use icon::{Icon, IconStyle};
pub use interactive::{Interactive, Sensed};
pub use stack::{Gap, Pad, Row, Stack};
pub use state_layer::{StateColors, StateLayer, StateLayerStyle};
pub use surface::{Surface, SurfaceEdge, SurfaceKind, SurfaceStyle};
pub use text::Text;

use crate::theme::UiTokens;

/// The colour of text and icons, by what they show and what they are drawn on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Content {
    /// Body text and icons: `text.normal`.
    #[default]
    Normal,
    /// Secondary text: `text.muted`.
    Muted,
    /// Placeholders and hints: `text.faint`.
    Faint,
    /// Disabled controls: `text.disabled`.
    Disabled,
    /// Links and accent-coloured text: `accent.text`.
    Accent,
    /// On `accent.solid` and its states: `text.on_accent`.
    OnAccent,
    /// On a status solid fill: `text.on_status`.
    OnStatus,
    /// On `state.selected`: `text.on_selected`.
    OnSelected,
}

impl Content {
    /// Every content colour, in declaration order.
    pub const ALL: &'static [Content] = &[
        Content::Normal,
        Content::Muted,
        Content::Faint,
        Content::Disabled,
        Content::Accent,
        Content::OnAccent,
        Content::OnStatus,
        Content::OnSelected,
    ];

    /// The name in story args and dumps, e.g. `"on_accent"`.
    pub const fn name(self) -> &'static str {
        match self {
            Content::Normal => "normal",
            Content::Muted => "muted",
            Content::Faint => "faint",
            Content::Disabled => "disabled",
            Content::Accent => "accent",
            Content::OnAccent => "on_accent",
            Content::OnStatus => "on_status",
            Content::OnSelected => "on_selected",
        }
    }

    /// The content whose [`name`](Content::name) is `name`.
    pub fn from_name(name: &str) -> Option<Content> {
        Content::ALL.iter().copied().find(|c| c.name() == name)
    }

    /// The token colour.
    pub fn color(self, tokens: &Tokens) -> Rgba {
        let c = &tokens.color;
        match self {
            Content::Normal => c.text.normal,
            Content::Muted => c.text.muted,
            Content::Faint => c.text.faint,
            Content::Disabled => c.text.disabled,
            Content::Accent => c.accent.text,
            Content::OnAccent => c.text.on_accent,
            Content::OnStatus => c.text.on_status,
            Content::OnSelected => c.text.on_selected,
        }
    }
}

/// The corners of an element without a [`Surface`], by what it is: one of the `radius.*` tokens.
/// An element on a surface takes the surface's corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Corner {
    /// Small controls, fields, toolbar items: `radius.control`.
    #[default]
    Control,
    /// Push and toggle buttons: `button.radius` (`radius.button` unless the theme overrides
    /// it).
    Button,
    /// Popups and menus: `radius.popup`.
    Popup,
    /// Windows and dialogs: `radius.window`.
    Window,
    /// Docked panels: `radius.panel`.
    Panel,
}

impl Corner {
    /// Every corner, in declaration order.
    pub const ALL: &'static [Corner] = &[Corner::Control, Corner::Button, Corner::Popup, Corner::Window, Corner::Panel];

    /// The name in story args and dumps, e.g. `"button"`.
    pub const fn name(self) -> &'static str {
        match self {
            Corner::Control => "control",
            Corner::Button => "button",
            Corner::Popup => "popup",
            Corner::Window => "window",
            Corner::Panel => "panel",
        }
    }

    /// The token radius.
    pub fn radius(self, tokens: &UiTokens) -> CornerRadius {
        let r = &tokens.radius;
        match self {
            Corner::Control => r.control,
            Corner::Button => r.button,
            Corner::Popup => r.popup,
            Corner::Window => r.window,
            Corner::Panel => r.panel,
        }
    }
}

/// The names of `Content::ALL`, for story args.
const CONTENT_NAMES: &[&str] = &["normal", "muted", "faint", "disabled", "accent", "on_accent", "on_status", "on_selected"];
