//! `HostRequests`: what a hosted dialog's rows ask of the host, kept until the host takes it.

use super::param::{ColorRequest, EyedropperRequest, PathRequest};
use super::rows::ParamRowsOutput;

/// The requests of parameter rows that the host answers: the colour picker, the file dialog and the eyedropper. A
/// hosted dialog embeds one, feeds it each pass's rows with [`absorb`](HostRequests::absorb) and offers the
/// `take_*` methods to its host. A request stays until it is taken; a newer one of the same kind replaces it.
#[derive(Clone, Debug, Default)]
pub struct HostRequests {
    color: Option<ColorRequest>,
    path: Option<PathRequest>,
    eyedropper: Option<EyedropperRequest>,
}

impl HostRequests {
    /// No request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the requests of `rows` in, leaving the rows' fields empty.
    pub fn absorb(&mut self, rows: &mut ParamRowsOutput) {
        self.color = rows.pick_color.take().or(self.color.take());
        self.path = rows.path.take().or(self.path.take());
        self.eyedropper = rows.eyedropper.take().or(self.eyedropper.take());
    }

    /// The colour parameter whose button was pressed, once.
    pub fn take_color(&mut self) -> Option<ColorRequest> {
        self.color.take()
    }

    /// The Browse button that was pressed, once.
    pub fn take_path(&mut self) -> Option<PathRequest> {
        self.path.take()
    }

    /// The eyedropper toggle that was pressed, once.
    pub fn take_eyedropper(&mut self) -> Option<EyedropperRequest> {
        self.eyedropper.take()
    }

    /// Drops every request not taken yet (the dialog closed).
    pub fn cancel(&mut self) {
        *self = Self::default();
    }
}
