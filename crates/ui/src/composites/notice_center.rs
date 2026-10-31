//! [`NoticeCenter`]: the app's queue of notices (saved, exported, couldn't open), shown as
//! [`ToastStack`](super::ToastStack) cards, with "don't show again" for notices that offer it.
//!
//! [`ToastStack`](super::ToastStack) draws, times and announces the cards; it keeps only timers. The notice center
//! is the model every app rebuilt beside it: ids, the cap that drops the oldest notice that can't
//! be silenced, the arguments a translated line is filled from, and the dismiss preference keys.

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::time::Duration;

use egui::{Context, Rect};

use super::Tone;

/// One notice.
#[derive(Clone, Debug, PartialEq)]
pub struct PostedNotice {
    /// Given by [`NoticeCenter::post`].
    pub id: u64,
    pub title: String,
    /// Lines under the title, already translated and filled in.
    pub lines: Vec<String>,
    pub tone: Tone,
    /// A preference key; when set, closing the notice with "Don't Show Again" reports
    /// [`NoticeEnd::Silenced`] with this key and the app stores it. A notice whose key the app
    /// has stored is never posted again ([`NoticeCenter::post`] checks `silenced`).
    pub dismiss_key: Option<String>,
    /// Stays until closed, whatever the auto-hide setting (an error the user must read).
    pub sticky: bool,
}

/// Why a notice left the center, for the app to act on.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NoticeEnd {
    Closed(u64),
    Expired(u64),
    /// Dropped by the cap.
    Dropped(u64),
    /// "Don't Show Again": store the key.
    Silenced {
        id: u64,
        key: String,
    },
}

/// The app's notices, kept with its UI state between frames.
///
/// ```no_run
/// use craft_ui::composites::{NoticeCenter, Tone};
/// # fn demo(ctx: &egui::Context, body: egui::Rect, center: &mut NoticeCenter, silenced: &[String]) {
/// center.post("Exported", vec!["poster.png".into()], Tone::Good, None, silenced);
/// for end in center.show(ctx, body) { /* store a Silenced key */ }
/// # }
/// ```
///
/// - **Cap:** at most three notices; a fourth drops the oldest that has no dismiss key, else the
///   oldest.
/// - **Auto-hide:** [`NoticeCenter::auto_hide`] (the app's Notification Duration preference, six
///   seconds by default; `None` keeps notices until closed). The timer pauses while the pointer
///   or keyboard is on the stack ([`ToastStack`](super::ToastStack)'s rule).
/// - **Don't Show Again:** a notice with a dismiss key shows the link under its lines.
#[derive(Clone, Debug, Default)]
pub struct NoticeCenter {
    notices: Vec<PostedNotice>,
    next_id: u64,
    auto_hide: Option<Duration>,
}

impl NoticeCenter {
    /// An empty center, auto-hiding after six seconds.
    pub fn new() -> Self {
        todo!()
    }

    /// The auto-hide time; `None` keeps notices until closed.
    // pc: crates/engine/src/prefs.rs:337 (`notification_duration_seconds`, 6 s) and the
    // `interface.notification_auto_hide` toggle (notices.rs:263).
    pub fn auto_hide(mut self, duration: Option<Duration>) -> Self {
        todo!()
    }

    /// Posts a notice and returns its id; `None` when its `dismiss_key` is in `silenced`.
    // pc: crates/ui-egui/src/notices.rs:49 `post`, :56 `cap_notices`, :13 `MAX_NOTICES` (3).
    pub fn post(
        &mut self,
        title: impl Into<String>,
        lines: Vec<String>,
        tone: Tone,
        dismiss_key: Option<String>,
        silenced: &[String],
    ) -> Option<u64> {
        todo!()
    }

    /// Removes a notice the app no longer needs (the job it reported on was cancelled).
    pub fn remove(&mut self, id: u64) {
        todo!()
    }

    pub fn notices(&self) -> &[PostedNotice] {
        todo!()
    }

    /// Shows the notices as a [`ToastStack`](super::ToastStack) above the bottom edge of `body` and removes those that
    /// ended this frame.
    // pc: crates/ui-egui/src/notices.rs:216 `show` (bottom right, inset (-16, -36), 340 pt wide),
    // :183 `expire` (hover restarts the timer), :149 `dismiss` (writes `prefs.dialogs[key]`).
    pub fn show(&mut self, ctx: &Context, body: Rect) -> Vec<NoticeEnd> {
        todo!()
    }
}

crate::words! {
    /// The notice center's labels, English by default.
    #[non_exhaustive]
    pub struct NoticeCenterLabels {
        dont_show_again: "noticeCenter.dontShowAgain" = "Don't Show Again";
    }
}
