//! Dialog shells: `Dialog`, `DialogFrame`, `FullWindowDialog`, `BlockingOverlay` and the values an app routes its
//! dialogs by (`DialogStack`, `DialogHost`). Prompts and form dialogs build on them.

mod blocking;
mod dialog;
mod frame;
mod full_window;
mod host;
mod stack;
#[cfg(feature = "testing")]
mod stories;
#[cfg(feature = "testing")]
mod trials;
mod words;

pub use blocking::{BlockingOverlay, BlockingOverlayStyle};
pub use dialog::{Dialog, DialogStyle};
pub use frame::{DialogFrame, DialogFrameStyle, FooterCx};
pub use full_window::{FullWindowDialog, FullWindowDialogStyle};
pub use host::{DialogHost, DialogResult, StackDialog, StartupStage};
pub use stack::{DialogId, DialogStack, dialog_id};
