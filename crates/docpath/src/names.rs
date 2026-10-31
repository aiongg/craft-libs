#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! File names made from user text (N10.09): a document title becomes the suggested name of an
//! export, a page range a set of file names, an attachment's stored name a Save suggestion.
//!
//! The rules hold on every platform, so a name made on Linux still works on Windows: no path
//! separators, reserved characters (`/ \ : * ? " < > |`) or control characters (each becomes
//! `_`); no bidirectional controls (U+061C, U+200E, U+200F, U+202A–U+202E, U+2066–U+2069,
//! dropped: they make `gnp.exe` read as `exe.png`); no leading spaces and no trailing dots or
//! spaces (Windows drops them, so `report.` would write `report`); Windows' device names (`CON`,
//! `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, with any extension) get a leading `_`.

/// The longest name [`safe_file_name`] and [`safe_file_stem`] return, in bytes: 200, so
/// `" (page 12).pdf"` still fits the usual 255-byte limit (PdfCraft `file_stem_from_user`).
pub const MAX_NAME_BYTES: usize = 200;

/// `text` made safe as a whole file name (`"Q3: plan?.pdf"` → `"Q3_ plan_.pdf"`), cut to
/// [`MAX_NAME_BYTES`] at a character boundary, keeping its extension when the cut would remove
/// it. `fallback` (`"attachment"`, `"Untitled"`) when nothing usable is left.
// pdfcraft: crates/ui-egui/src/workflows/files.rs:223 attachment_file_name
pub fn safe_file_name(text: &str, fallback: &str) -> String {
    todo!()
}

/// `text` made safe as the stem of a file whose extension the app adds (`extension` without the
/// dot, `"pdf"`): the same rules, and a trailing `.<extension>` in any case removed first
/// (`"Report.PDF"` → `"Report"`). Empty when nothing usable is left; the caller picks the
/// default.
// pdfcraft: crates/ui-egui/src/workflows/files.rs:196 file_stem_from_user
pub fn safe_file_stem(text: &str, extension: &str) -> String {
    todo!()
}

/// Whether `name` names a file directly inside a folder: not empty, no `/` or `\`, no drive
/// (`C:`), not ending in `.` or a space (which also rules out `.` and `..`). On Windows any `:`
/// is refused, which rules out alternate data streams.
// pdfcraft: crates/ui-egui/src/workflows/files.rs:246 is_plain_file_name
pub fn is_plain_file_name(name: &str) -> bool {
    todo!()
}

/// Whether `stem` (a name without its extension, compared without case) is a Windows device
/// name: `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, and the superscript digits
/// `COM¹`–`COM³`, `LPT¹`–`LPT³` Windows also reserves.
// pdfcraft: crates/ui-egui/src/workflows/files.rs:223 attachment_file_name (the device check at :234)
pub fn is_reserved_device_name(stem: &str) -> bool {
    todo!()
}

/// `name`, or the first of `name 2.ext`, `name 3.ext`, … that `taken` says is free, as macOS
/// Finder numbers copies (Windows Explorer writes `name (2).ext`; the app picks with `style`).
/// The number goes before the last extension; a name that already ends in ` 2` counts on from
/// it (`photo 2.jpg` → `photo 3.jpg`). Stops after 10 000 tries and returns the last candidate.
// new: PdfCraft, DeckCraft and the export dialog (`numbered_paths`) each number names their own way
pub fn unique_file_name(name: &str, style: CopyNumbering, taken: impl Fn(&str) -> bool) -> String {
    todo!()
}

/// How [`unique_file_name`] numbers a name that is taken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CopyNumbering {
    /// `photo 2.jpg` (macOS Finder).
    #[default]
    Space,
    /// `photo (2).jpg` (Windows Explorer, GNOME Files).
    Parenthesised,
}
