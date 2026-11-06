//! The style layer: the state set and its resolver, style parts and `ComponentStyle`, tokens (`UiTokens`,
//! `StyleCx`), the token-group cache and the style cache.

mod cache;
mod cx;
mod key;
mod look;
mod paint;
mod replacements;
mod state;
mod surface;
mod tokens;

pub use cache::{Accepted, accepted_variant};
pub use craft_theme::{FlagMask, Flags, Mix, Resolve, State, StateMix, Stateful, States, TokenGroup, TypeRole};
pub use cx::{StyleCx, VariantParts};
pub use key::{EnvKey, Ink, StyleKey};
pub use look::{ComponentStyle, Corners, DataColor, FocusLook, IconLook, StyleColor, SurfaceKind, SurfaceLook, TextLook};
pub use craft_behavior::{ControlMetrics, Insets, Metrics};
pub use paint::{HandleLook, ResetMarkLook, RuleKind, RuleLook, ScrollBarLook};
pub use replacements::{DuplicateStyle, StyleReplacements};
pub use state::StateSet;
pub use surface::{SurfaceEdge, SurfaceStyle};
pub use tokens::UiTokens;

pub(crate) use cache::resolve_style;
pub(crate) use paint::{ink_color, rule_look};
pub(crate) use surface::surface_style;
