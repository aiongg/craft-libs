//! The values an editor reports to the input method: its surrounding text and its composition. The IME engine is
//! `contract::rt::ime`.

/// The committed window around the caret that an editor reports to the input method, or why there is none.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EditorSurrounding<'a> {
    /// The text around the caret, with UTF-8 offsets of the cursor and the selection anchor into it.
    #[non_exhaustive]
    Text {
        /// The committed text.
        text: &'a str,
        /// The cursor's byte offset into `text`.
        cursor: usize,
        /// The selection anchor's byte offset into `text`.
        anchor: usize,
    },
    /// The editor has no text to report.
    #[non_exhaustive]
    Unavailable {
        /// The revision the editor reports it at, if it keeps one.
        revision: Option<u64>,
    },
}

impl<'a> EditorSurrounding<'a> {
    /// The text around the caret, with byte offsets of the cursor and the selection anchor into `text`.
    pub const fn text(text: &'a str, cursor: usize, anchor: usize) -> Self {
        EditorSurrounding::Text { text, cursor, anchor }
    }
}

/// What an editor reports of the input method's composition (`EditorState::composing`, `EditorState::shows_marked`).
#[derive(Clone, Copy)]
pub(crate) struct Composing {
    /// The editor shows marked text now.
    pub(crate) now: bool,
    /// The editor draws the marked text of the pre-edits it is given.
    pub(crate) shows_marked: bool,
}
