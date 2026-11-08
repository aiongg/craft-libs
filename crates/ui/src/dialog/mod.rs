//! Dialog shells and prompts: `Dialog`, `DialogFrame`, `FullWindowDialog`, `BlockingOverlay`, `Prompt` with its
//! configurations (`Alert`, `StandardPrompt`, `UnsavedChangesDialog`, `RecoveryDialog`) and the values an app routes
//! its dialogs by (`DialogStack`, `DialogHost`). Form dialogs build on the shells.

mod alert;
mod blocking;
mod frame;
mod full_window;
// full-window parts
mod full_window_parts;
// end of full-window parts
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

pub use alert::{Alert, AlertOutput};
pub use blocking::{BlockingOverlay, BlockingOverlayStyle};
pub use frame::{DialogFrame, DialogFrameStyle, DialogOutput, FooterCx};
pub use full_window::{FullWindowDialog, FullWindowDialogStyle};
// full-window parts
pub use full_window_parts::{
    FullWindowCanvas, FullWindowCanvasOutput, FullWindowSide, FullWindowSideOutput, FullWindowSideStyle, FullWindowStatus, FullWindowStatusOutput,
    FullWindowTitle, FullWindowTitleOutput, FullWindowView, FullWindowZoomBar, FullWindowZoomBarOutput, RasterOverlay,
    SideFooter, SideWidth,
};
// end of full-window parts
pub use host::{DialogHost, DialogResult, StackDialog, StartupStage};
pub use modal::{Dialog, DialogStyle};
pub use prompt::{Prompt, PromptOutput, accelerator_labels};
pub use prompts::{PromptAnswer, PromptChoice, PromptKind, StandardPrompt, StandardPromptOutput};
pub use recovery::{RecoveredDocument, RecoveryAnswer, RecoveryDialog, RecoveryOutput};
pub use stack::{DialogId, DialogStack, dialog_id};
pub use unsaved::{UnsavedAnswer, UnsavedChangesDialog, UnsavedChangesOutput};
pub use words::DialogWords;
