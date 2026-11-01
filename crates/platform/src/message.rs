//! [`PlatformText`]: a user-visible message of a platform service, with its catalog id and
//! English text. craft-platform has no catalog; the app looks the id up in its own.

/// A message with `{name}`-style placeholders the app fills after looking `id` up (the
/// English is the fallback). The open, clipboard, watch, default-app and temp services and
/// craft-dialogs' save flow word their messages with these, so every app shows the same text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlatformText {
    pub id: &'static str,
    pub english: &'static str,
}
