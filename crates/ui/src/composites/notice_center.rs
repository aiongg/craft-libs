//! [`NoticeCenter`]: the app's queue of notices (saved, exported, couldn't open), shown as
//! [`ToastStack`](super::ToastStack) cards, with "don't show again" for notices that offer it.
//!
//! [`ToastStack`](super::ToastStack) draws, times and announces the cards; it keeps only timers. The notice center
//! is the model every app rebuilt beside it: ids, the cap that drops the oldest notice that can't
//! be silenced, the arguments a translated line is filled from, and the dismiss preference keys.

use std::time::Duration;

use egui::{AsIdSalt, Context, Id, Rect};

use super::{Toast, ToastEnd, ToastStack, Tone};
use crate::localized::SharedLabels as _;

/// Most notices kept at once.
const MAX_NOTICES: usize = 3;
/// Seconds before a notice auto-hides, when the app sets nothing else.
const DEFAULT_AUTO_HIDE: Duration = Duration::from_secs(6);

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

/// What [`NoticeCenter::post`] returns.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "handle `dropped`"]
pub struct NoticePost {
    /// The new notice's id; `None` when its dismiss key is silenced.
    pub id: Option<u64>,
    /// [`NoticeEnd::Dropped`] for each notice the cap removed to make room, reported now: the
    /// app handles them in the pass that posted.
    pub dropped: Vec<NoticeEnd>,
}

/// The app's notices, kept with its UI state between frames.
///
/// ```no_run
/// use craft_ui::composites::{NoticeCenter, Tone};
/// # fn demo(ctx: &egui::Context, body: egui::Rect, center: &mut NoticeCenter, silenced: &[String]) {
/// let posted = center.post("Exported", vec!["poster.png".into()], Tone::Good, None, silenced);
/// for end in posted.dropped { /* the cap removed a notice */ }
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
#[derive(Clone, Debug)]
pub struct NoticeCenter {
    notices: Vec<PostedNotice>,
    next_id: u64,
    auto_hide: Option<Duration>,
    id: Id,
}

impl Default for NoticeCenter {
    fn default() -> Self {
        Self::new()
    }
}

impl NoticeCenter {
    /// An empty center, auto-hiding after six seconds.
    pub fn new() -> Self {
        Self { notices: Vec::new(), next_id: 1, auto_hide: Some(DEFAULT_AUTO_HIDE), id: Id::unique("craft_ui::composites::NoticeCenter") }
    }

    /// The id salt of the toast stack, for an app with two centers in one viewport.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id = Id::unique("craft_ui::composites::NoticeCenter").with(id_salt);
        self
    }

    /// The auto-hide time; `None` keeps notices until closed.
    // pc: crates/engine/src/prefs.rs:337 (`notification_duration_seconds`, 6 s) and the
    // `interface.notification_auto_hide` toggle (notices.rs:263).
    pub fn auto_hide(mut self, duration: Option<Duration>) -> Self {
        self.auto_hide = duration;
        self
    }

    /// Posts a notice. No notice is posted when its `dismiss_key` is in `silenced`.
    // pc: crates/ui-egui/src/notices.rs:49 `post`, :56 `cap_notices`, :13 `MAX_NOTICES` (3).
    pub fn post(&mut self, title: impl Into<String>, lines: Vec<String>, tone: Tone, dismiss_key: Option<String>, silenced: &[String]) -> NoticePost {
        if dismiss_key.as_ref().is_some_and(|key| silenced.contains(key)) {
            return NoticePost { id: None, dropped: Vec::new() };
        }
        let id = self.next_id;
        self.next_id += 1;
        self.notices.push(PostedNotice { id, title: title.into(), lines, tone, dismiss_key, sticky: false });
        let mut dropped = Vec::new();
        while self.notices.len() > MAX_NOTICES {
            let oldest = self.notices.iter().position(|n| n.dismiss_key.is_none()).unwrap_or(0);
            dropped.push(NoticeEnd::Dropped(self.notices.remove(oldest).id));
        }
        NoticePost { id: Some(id), dropped }
    }

    /// Keeps notice `id` until it is closed, whatever the auto-hide time (an error the user must
    /// read).
    pub fn keep(&mut self, id: u64) {
        if let Some(notice) = self.notices.iter_mut().find(|n| n.id == id) {
            notice.sticky = true;
        }
    }

    /// Removes a notice the app no longer needs (the job it reported on was cancelled).
    pub fn remove(&mut self, id: u64) {
        self.notices.retain(|n| n.id != id);
    }

    pub fn notices(&self) -> &[PostedNotice] {
        &self.notices
    }

    /// Shows the notices as a [`ToastStack`](super::ToastStack) above the bottom edge of `body` and removes those that
    /// ended this frame.
    // pc: crates/ui-egui/src/notices.rs:216 `show` (bottom right, inset (-16, -36), 340 pt wide),
    // :183 `expire` (hover restarts the timer), :149 `dismiss` (writes `prefs.dialogs[key]`).
    pub fn show(&mut self, ctx: &Context, body: Rect) -> Vec<NoticeEnd> {
        let mut ends = Vec::new();
        let label = NoticeCenterLabels::shared_ctx(ctx).dont_show_again.clone();
        let toasts = self.notices.iter().map(|n| {
            let toast = Toast::new(n.id, n.title.as_str()).tone(n.tone).lines(n.lines.iter().map(String::as_str)).persistent(n.sticky);
            if n.dismiss_key.is_some() { toast.action(label.as_str()) } else { toast }
        });
        let ended = ToastStack::new(self.id).auto_hide(self.auto_hide).show(ctx, body, toasts);
        for (id, end) in ended {
            let key = self.notices.iter().find(|n| n.id == id).and_then(|n| n.dismiss_key.clone());
            ends.push(match (end, key) {
                (ToastEnd::Acted, Some(key)) => NoticeEnd::Silenced { id, key },
                (ToastEnd::Acted | ToastEnd::Closed, _) => NoticeEnd::Closed(id),
                (ToastEnd::Expired, _) => NoticeEnd::Expired(id),
                (ToastEnd::Dropped, _) => NoticeEnd::Dropped(id),
            });
        }
        self.notices.retain(|n| !ends.iter().any(|end| ends_id(end) == n.id));
        ends
    }
}

crate::words! {
    /// The notice center's labels, English by default.
    #[non_exhaustive]
    pub struct NoticeCenterLabels {
        dont_show_again: "noticeCenter.dontShowAgain" = "Don't Show Again";
    }
}

fn ends_id(end: &NoticeEnd) -> u64 {
    match end {
        NoticeEnd::Closed(id) | NoticeEnd::Expired(id) | NoticeEnd::Dropped(id) => *id,
        NoticeEnd::Silenced { id, .. } => *id,
    }
}
