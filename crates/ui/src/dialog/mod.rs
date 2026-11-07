//! Dialog shells and prompts: `Dialog`, `DialogFrame`, `FullWindowDialog`, `BlockingOverlay`, `Prompt` with its
//! configurations (`Alert`, `StandardPrompt`, `UnsavedChangesDialog`, `RecoveryDialog`) and the values an app routes
//! its dialogs by (`DialogStack`, `DialogHost`). Form dialogs build on the shells.

mod alert;
mod blocking;
mod frame;
mod full_window;
mod host;
mod modal;
mod prompt;
mod prompts;
mod recovery;
mod stack;
#[cfg(feature = "testing")]
mod stories;
#[cfg(feature = "testing")]
mod trials;
mod unsaved;
mod words;

pub use alert::Alert;
pub use blocking::{BlockingOverlay, BlockingOverlayStyle};
pub use frame::{DialogFrame, DialogFrameStyle, DialogOutput, FooterCx};
pub use full_window::{FullWindowDialog, FullWindowDialogStyle};
pub use host::{DialogHost, DialogResult, StackDialog, StartupStage};
pub use modal::{Dialog, DialogStyle};
pub use prompt::{Prompt, PromptOutput, accelerator_labels};
pub use prompts::{PromptAnswer, PromptChoice, PromptKind, StandardPrompt};
pub use recovery::{RecoveredDocument, RecoveryAnswer, RecoveryDialog};
pub use stack::{DialogId, DialogStack, dialog_id};
pub use unsaved::{UnsavedAnswer, UnsavedChangesDialog};
pub use words::{PromptLabels, RecoveryLabels, UnsavedLabels};
