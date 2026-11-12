//! What the user is told when the window could not start.

use std::path::Path;

use super::ladder::{Rung, ladder_known};

/// The longest error shown per rung; wgpu's run to many lines.
const MAX_ERROR_SHOWN: usize = 400;

/// The message for a window that could not start: the last `error`, each rung this start tried
/// with the error it ended in (`failed`, [`super::GraphicsStart::failures`]), how to choose a rung
/// by hand on this platform, the app and its `version`, and where the log is (`log`). In English;
/// the app shows it before any translation is loaded.
pub fn failure_message(app: &str, version: &str, error: &str, failed: &[(Rung, String)], log: Option<&Path>) -> String {
    message_for(app, version, error, failed, log, ladder_known())
}

fn shorten(text: &str) -> String {
    let text = text.trim();
    match text.char_indices().nth(MAX_ERROR_SHOWN) {
        Some((end, _)) => format!("{}…", text.get(..end).unwrap_or(text)),
        None => text.to_string(),
    }
}

pub(crate) fn message_for(app: &str, version: &str, error: &str, failed: &[(Rung, String)], log: Option<&Path>, ladder: &[Rung]) -> String {
    let mut msg = format!("{app} could not start its window: {}\n", shorten(error));
    if !failed.is_empty() {
        msg.push_str("\nGraphics tried:\n");
        for (rung, err) in failed {
            msg.push_str(&format!("• {} (--graphics {}): {}\n", rung.label(), rung.name(), shorten(err)));
        }
    }
    msg.push_str("\nUpdating the graphics driver usually fixes this.");
    if ladder.len() > 1 {
        let names: Vec<String> = ladder.iter().map(|r| format!("--graphics {}", r.name())).collect();
        msg.push_str(&format!(" To choose the graphics yourself, start {app} with {}.", names.join(", ")));
    }
    msg.push_str(&format!("\n\n{app} {version} ({} {})", std::env::consts::OS, std::env::consts::ARCH));
    if let Some(log) = log {
        msg.push_str(&format!("\nDetails are in {}", log.display()));
    }
    msg
}
