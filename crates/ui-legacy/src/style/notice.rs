//! Style of the `notice` family.

use craft_theme::{Rgba, Tokens, Tone};

/// Text colour of each notice tone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoticeStyle {
    fg: [Rgba; Tone::ALL.len()],
}

impl NoticeStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let c = &tokens.color;
        NoticeStyle { fg: Tone::ALL.map(|tone| tone.status().map_or(c.text.muted, |status| c.status.get(status).fg)) }
    }

    /// The colour of a notice of `tone`.
    pub fn fg(&self, tone: Tone) -> Rgba {
        self.fg[tone as usize]
    }
}
