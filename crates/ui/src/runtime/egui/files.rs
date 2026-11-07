//! The files the OS hovers over or drops on the running viewport this pass.

use crate::runtime::drag::OsFile;
use egui::Context;

/// What the OS reported for the running pass.
pub(crate) struct Seen {
    /// The files held over the window: a drop is under way.
    pub hovered: Vec<OsFile>,
    /// The files dropped on the window this pass.
    pub dropped: Vec<OsFile>,
}

/// The hovered and dropped files of the running pass.
pub(crate) fn seen(ctx: &Context) -> Seen {
    ctx.input(|input| Seen {
        hovered: input
            .raw
            .hovered_files
            .iter()
            .map(|file| OsFile {
                name: file.path.as_ref().and_then(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
                path: file.path.clone(),
                mime: file.mime.clone(),
                bytes: None,
            })
            .collect(),
        dropped: input
            .raw
            .dropped_files
            .iter()
            .map(|file| OsFile { name: file.name.clone(), path: file.path.clone(), mime: file.mime.clone(), bytes: file.bytes.clone() })
            .collect(),
    })
}
