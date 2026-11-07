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

mod guides;
mod properties;

pub use guides::{
    GUIDE_LAYOUT_COLOR_KEY, GuideColor, GuideColorChoice, GuideGroup, GuideLayoutDialog, GuideLayoutError, GuideLayoutOptions, GuideLayoutOutput,
    GuideLayoutPreset, GuideLayoutResult, GuideLayoutValues, GuideLength, GuideRect, GuideTarget, GuideUnits, GuideWords, HostedGuideLayoutDialog,
    HostedNewGuideDialog, NEW_GUIDE_COLOR_KEY, NewGuideDialog, NewGuideOutput, NewGuideValues,
};
pub use properties::{PropertiesDialog, PropertiesOutput, PropertiesTab, PropertyRow};
