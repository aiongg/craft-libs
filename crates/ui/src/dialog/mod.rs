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
//! - [`DialogHost`], [`StackDialog`], [`DialogResult`]: the dialogs an app has open as values, one
//!   `open` and one `take`, with the startup modal queue ([`StartupStage`]).
//! - [`DialogFields`], [`Param`], [`ParamDialog`], [`CommandForm`]: a dialog from a parameter list,
//!   its values by key for scripts and tests, last-used values and Reset; [`translated`], the
//!   catalog lookup of a runtime label.
//! - [`ImageSizeDialog`], [`CanvasSizeDialog`], [`LinkedSizeFields`], [`AnchorGrid`]: the size
//!   dialogs.
//! - [`StandardPrompt`], [`UnsavedChangesDialog`], [`PropertiesDialog`], [`ReportDialog`]: the
//!   confirmations every app asks the same way, the unsaved-changes question of
//!   [`CloseGuard`](craft_quit::CloseGuard), tabs of properties, a result report.
//!
//! - [`NewDocumentDialog`]: File › New with preset cards and the details column.
//! - [`RecoveryDialog`]: the offer to recover a document an earlier run autosaved.
//! - [`export`]: Export As, Save for Web, the format option dialogs, the watermark group and the
//!   output path row.
//!
//! Sizes are the `dialog.*` tokens ([`DialogTokens`](craft_theme::DialogTokens)); the colours are
//! the popup family's. Each part has a pure `style` function, dumped under
//! `crates/ui/tests/dumps/components/`.

mod about;
mod about_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod about_tests;
mod alert;
mod blocking;
mod buttons;
mod color_request;
mod command_form;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod command_form_tests;
#[allow(clippy::module_inception)]
mod dialog;
mod dialog_host;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_host_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_tests;
pub mod export;
mod fields;
#[cfg(test)]
mod fields_tests;
mod files_stories;
mod focus_scroll;
mod footer;
#[cfg(test)]
mod footer_pure_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod footer_tests;
mod frame;
pub(crate) mod framework_stories;
pub mod full_window;
pub(crate) mod full_window_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod full_window_tests;
mod guides;
mod guides_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod guides_tests;
mod history;
mod keys;
mod list_manager;
mod list_manager_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod list_manager_tests;
mod new_document;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod new_document_tests;
mod params;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod params_tests;
pub mod pass;
mod password;
mod password_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod password_tests;
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
mod prompts;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod prompts_tests;
mod properties;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod properties_tests;
mod recovery;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod recovery_tests;
mod report;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod report_tests;
mod size;
#[cfg(test)]
mod size_tests;
mod sizing;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod sizing_tests;
mod stack;
mod stack_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod stack_tests;
mod state;
pub(crate) mod stories;
mod unsaved;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod unsaved_tests;

pub use about::{
    AboutDialog, AboutInfo, AboutLabels, AboutLink, AboutTab, CommitShare, Contributor, ContributorSort, Credits, ModelCredit, NameMode,
    commit_share, sorted_contributors,
};
pub use alert::Alert;
pub use blocking::BlockingOverlay;
pub use buttons::{ButtonRole, Clicked, FooterButton, FooterButtons, FooterButtonsResponse, Platform, order};
pub use color_request::ColorRequest;
pub use command_form::CommandForm;
pub use dialog::focus_inside;
pub use dialog::{Backdrop, Dialog, DialogResponse, InitialFocus, Outcome};
pub use dialog_host::{DialogHost, DialogResult, StackDialog, StartupStage};
pub use fields::{DialogFields, FieldValue};
pub use footer::{DialogLabels, Footer, FooterFx, Reset};
pub use frame::{DialogFrame, DialogFrameResponse, DialogFrameStyle};
pub use full_window::{FullWindowDialog, FullWindowFx};
pub use guides::{
    GUIDE_LAYOUT_COLOR_KEY, GuideAxis, GuideColor, GuideLayoutDialog, GuideLayoutError, GuideLayoutLines, GuideLayoutOptions, GuideLayoutPreset,
    GuideLayoutResult, GuideLines, GuidePreview, GuideRect, GuideTarget, GuideUnits, GuidesLabels, NEW_GUIDE_COLOR_KEY, NewGuideDialog,
    NewGuideOptions,
};
pub use history::{LocalHistory, Step, take_step};
pub use keys::{EnterRule, KeyInput, resolve_keys};
pub use list_manager::{ActionEnabled, ListManagerAction, ListManagerChoice, ListManagerDialog, ListManagerEntry, ListManagerLabels};
pub use new_document::{
    BACKGROUND_COLOR_KEY, BackgroundContents, BitDepth, ColorMode, DocumentOptions, DocumentPreset, NewDocumentDialog, NewDocumentLabels,
    Orientation, PresetCategory, push_recent, size_line,
};
pub use params::{
    CustomContext, CustomRow, EyedropperRequest, Param, ParamDialog, ParamKind, ParamLayout, ParamResult, ParamRows, ParamRowsResponse, PathKind,
    PathRequest, translated,
};
pub use pass::any_modal;
pub use password::{PasswordAnswer, PasswordDialog, PasswordLabels, PasswordPurpose, PasswordStrength, Secret, password_strength};
pub use placement::{PlaceKey, keep_on_screen, next_pin};
pub use preview::Preview;
pub use prompt::{Prompt, accelerator_labels};
pub use prompts::{PromptAnswer, PromptChoice, PromptKind, PromptLabels, StandardPrompt};
pub use properties::{PropertiesDialog, PropertiesTab, PropertyRow};
pub use recovery::{RecoveredDocument, RecoveryAnswer, RecoveryDialog, RecoveryLabels};
pub use report::{NoteTone, ReportDialog, ReportLabels, ReportNote};
pub use size::{Body, Resize, ResizeMax, Width, remembered_size, scroll_height};
pub use sizing::{
    Anchor, AnchorGrid, CanvasExtension, CanvasSizeDialog, CanvasSizeResult, ImageSizeDialog, ImageSizeResult, LinkedSizeFields, PixelSize, Resample,
    ResolutionUnit, SizeUnit, SizingLabels, human_bytes,
};
pub use stack::{DialogId, DialogStack, dialog_id};
pub use state::DialogState;
pub use stories::COMPONENTS;
pub use unsaved::{UnsavedAnswer, UnsavedChangesDialog, UnsavedLabels};
