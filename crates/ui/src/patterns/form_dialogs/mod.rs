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
mod preview;

pub use dialog::{BelowRows, FooterExtra, HostedParamDialog, ParamDialog, ParamDialogOutput, ParamDialogWords, ParamLayout, ParamResult};
pub use preview::{PreviewOutput, PreviewPane, PreviewPaneStyle, PreviewWords, PreviewZoom, PreviewZoomOutput};

pub mod size_model;

pub use size_model::{
    COMMIT_DECIMALS, DEFAULT_RESOLUTION, LinkedSize, MAX_PIXELS, POINTS_PER_INCH, PPI_RANGE, PixelSize, SizeError, resize_from_scale,
    resize_from_side,
};
