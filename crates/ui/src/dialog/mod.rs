//! Dialogs: the frame every dialog shares and the behaviour a host would otherwise repeat.
//!
//! - [`DialogFrame`]: the look. The popup family's surface with its shadow, a title row (title,
//!   draggable band, close box), a hairline, the body and a footer area. No behaviour.
//! - [`Dialog`]: a modal or modeless dialog with placement memory, width classes, a resizable
//!   size, a scrolling body and an [`Outcome`]. The host keeps its fields and passes them by `&mut`;
//!   [`DialogState`] holds the chrome state only.
//! - [`Footer`], [`FooterButtons`], [`ButtonRole`], [`Platform`]: the role-based button area in
//!   each platform's order, with Reset on Alt, Apply and the Enter and Esc rules
//!   ([`resolve_keys`]).
//! - [`keep_on_screen`], [`remembered_size`], [`Width::range`], [`scroll_height`]: the placement and
//!   size rules, as pure functions.
//!
//! Sizes are the `dialog.*` tokens ([`DialogTokens`](craft_theme::DialogTokens)); the colours are
//! the popup family's. Each part has a pure `style` function, dumped under
//! `crates/ui/tests/dumps/components/`.

mod buttons;
#[allow(clippy::module_inception)]
mod dialog;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_tests;
mod footer;
#[cfg(test)]
mod footer_pure_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod footer_tests;
mod frame;
mod keys;
mod placement;
#[cfg(test)]
mod placement_tests;
mod size;
#[cfg(test)]
mod size_tests;
mod state;
pub(crate) mod stories;

pub use buttons::{ButtonRole, Clicked, FooterButton, FooterButtons, FooterButtonsResponse, Platform, order};
pub use dialog::{Backdrop, Dialog, DialogResponse, InitialFocus, Outcome};
pub use footer::{DialogLabels, Footer, FooterFx, Reset};
pub use frame::{DialogFrame, DialogFrameResponse, DialogFrameStyle};
pub use keys::{EnterRule, KeyInput, resolve_keys};
pub use placement::{PlaceKey, keep_on_screen, next_pin};
pub use size::{Body, Resize, ResizeMax, Width, remembered_size, scroll_height};
pub use state::DialogState;
pub use stories::COMPONENTS;
