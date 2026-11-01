//! [`StatusLine`]: a message for the status bar.

use super::Tone;

/// A message for the status bar and its tone. Every component that reports a status-bar message
/// reports this.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusLine {
    /// The message.
    pub text: String,
    /// How it is drawn: Good, Warn or Bad.
    pub tone: Tone,
}
