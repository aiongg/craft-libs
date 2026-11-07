//! `NoticeCenter` (20.3): posts the app's notices (saved, exported, couldn't open) to the toast queue of
//! `runtime::toast`, which the shell's `ToastHost` shows.

use crate::contract::Politeness;
use crate::runtime::toast::{self, Toast, ToastId};
use craft_theme::variant::Tone;
use egui::Context;

/// Seconds before a notice hides when the app sets nothing else.
const DEFAULT_TIMEOUT: f32 = 6.0;

/// One notice, already translated and filled in.
#[derive(Clone, Debug, PartialEq, crate::contract::Props)]
#[non_exhaustive]
pub struct PostedNotice {
    /// The title line.
    #[prop(into)]
    title: String,
    /// The lines under the title.
    #[prop(into)]
    lines: Vec<String>,
    /// The tone the notice reads in; a failure is announced at once.
    #[prop]
    tone: Tone,
    /// A preference key. A notice whose key the app has stored in `silenced` is not posted.
    #[prop(into)]
    dismiss_key: Option<String>,
    /// Stays until dismissed, whatever the center's timeout (an error the user must read).
    #[prop(flag)]
    sticky: bool,
}

impl PostedNotice {
    /// A notice titled `title` in the neutral tone.
    pub fn new(title: impl Into<String>) -> Self {
        PostedNotice { title: title.into(), lines: Vec::new(), tone: Tone::Neutral, dismiss_key: None, sticky: false }
    }
}

/// The app's way to post notices. It holds only its timeout: the queue, the timers, the hover pause and the
/// announcement are the toast service's.
#[derive(Clone, Debug, PartialEq, crate::contract::Pattern, crate::contract::Props)]
pub struct NoticeCenter {
    /// Seconds before a notice hides, from the app's notification duration.
    #[prop(quantity = "seconds")]
    timeout: Option<f32>,
}

impl Default for NoticeCenter {
    fn default() -> Self {
        NoticeCenter::new()
    }
}

impl NoticeCenter {
    /// A center whose notices hide after six seconds.
    pub fn new() -> Self {
        NoticeCenter { timeout: Some(DEFAULT_TIMEOUT) }
    }

    /// Posts `notice` to the running viewport; `None` when its dismiss key is in `silenced`.
    pub fn post(&self, ctx: &Context, notice: &PostedNotice, silenced: &[String]) -> Option<ToastId> {
        if notice.dismiss_key.as_ref().is_some_and(|key| silenced.contains(key)) {
            return None;
        }
        let message = std::iter::once(notice.title.as_str()).chain(notice.lines.iter().map(String::as_str)).collect::<Vec<_>>().join("\n");
        let politeness = if notice.tone == Tone::Bad { Politeness::Assertive } else { Politeness::Polite };
        let timeout = if notice.sticky { None } else { self.timeout };
        Some(toast::post(ctx, Toast::new(message).timeout(timeout).politeness(politeness)))
    }

    /// Removes a notice the app no longer needs (the job it reported on was cancelled).
    pub fn remove(&self, ctx: &Context, id: ToastId) {
        toast::dismiss(ctx, id);
    }
}
