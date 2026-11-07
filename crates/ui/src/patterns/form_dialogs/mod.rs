//! Form-dialog patterns on `Dialog` and `Field`: `ParamDialog`, `CommandForm` and the option, sizing and export dialogs.

mod words;

mod fields;
mod param;
mod rows;

pub use fields::{DialogFields, FieldValue};
pub use param::{
    ColorRequest, CustomContext, CustomRow, EyedropperRequest, NumberUnit, Param, ParamKind, PathKind, PathRequest, encode, open_fields,
    set_color_field,
};
pub use rows::{ParamRows, ParamRowsOutput};
pub use words::ParamWords;

mod dialog;
mod host_requests;
mod preview;

pub use dialog::{
    BelowRows, FooterExtra, HostedParamDialog, ParamDialog, ParamDialogOutput, ParamDialogWords, ParamLayout, ParamResult, ResetFooter, ResetPick,
};
pub use host_requests::HostRequests;
pub use preview::{PreviewOutput, PreviewPane, PreviewPaneStyle, PreviewState, PreviewView, PreviewWords, PreviewZoom, PreviewZoomOutput};

mod size_model;

pub use size_model::{LinkedSize, MAX_DOCUMENT_SIDE, SizeError, resize_from_scale, resize_from_side};

mod command_form;
mod list_manager;
mod password;
mod report;

pub use command_form::{CommandForm, CommandFormOutput, HostedCommandForm};
pub use list_manager::{
    ActionEnabled, ListManager, ListManagerAction, ListManagerChoice, ListManagerCommands, ListManagerEntry, ListManagerOutput, ListManagerWords,
};
pub use password::{PasswordAnswer, PasswordDialog, PasswordOutput, PasswordPurpose, PasswordStrength, PasswordWords, Secret, password_strength};
pub use report::{ReportDialog, ReportNote, ReportOutput, ReportWords};
