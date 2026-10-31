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
//! - [`Preview`], [`LocalHistory`], [`take_step`]: the Preview check box with hold-to-compare, and
//!   undo inside a dialog ([`Dialog::history`]).
//! - [`DialogStack`], [`pass`], [`any_modal`]: which modal dialog takes the input, and the canvas
//!   input that passes through under it.
//! - [`Prompt`], [`Alert`], [`BlockingOverlay`]: a question with accelerator keys, a one-button
//!   notice, and an input block with no chrome.
//! - [`keep_on_screen`], [`remembered_size`], [`Width::range`], [`scroll_height`]: the placement and
//!   size rules, as pure functions.
//!
//! Sizes are the `dialog.*` tokens ([`DialogTokens`](craft_theme::DialogTokens)); the colours are
//! the popup family's. Each part has a pure `style` function, dumped under
//! `crates/ui/tests/dumps/components/`.

mod alert;
mod blocking;
mod buttons;
#[allow(clippy::module_inception)]
mod dialog;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_tests;
mod focus_scroll;
mod footer;
#[cfg(test)]
mod footer_pure_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod footer_tests;
mod frame;
mod full_window;
pub(crate) mod full_window_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod full_window_tests;
mod history;
mod keys;
pub mod pass;
mod placement;
#[cfg(test)]
mod placement_tests;
mod preview;
pub(crate) mod preview_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod preview_tests;
mod prompt;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod prompt_tests;
mod size;
#[cfg(test)]
mod size_tests;
mod stack;
mod stack_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod stack_tests;
mod state;
pub(crate) mod stories;

pub use alert::Alert;
pub use blocking::BlockingOverlay;
pub use buttons::{ButtonRole, Clicked, FooterButton, FooterButtons, FooterButtonsResponse, Platform, order};
pub(crate) use dialog::focus_inside;
pub use dialog::{Backdrop, Dialog, DialogResponse, InitialFocus, Outcome};
pub use footer::{DialogLabels, Footer, FooterFx, Reset};
pub use frame::{DialogFrame, DialogFrameResponse, DialogFrameStyle};
pub use full_window::{FullWindowDialog, FullWindowFx};
pub use history::{LocalHistory, Step, take_step};
pub use keys::{EnterRule, KeyInput, resolve_keys};
pub use pass::any_modal;
pub use placement::{PlaceKey, keep_on_screen, next_pin};
pub use preview::Preview;
pub use prompt::{Prompt, accelerator_labels};
pub use size::{Body, Resize, ResizeMax, Width, remembered_size, scroll_height};
pub use stack::{DialogId, DialogStack, dialog_id};
pub use state::DialogState;
pub use stories::COMPONENTS;
