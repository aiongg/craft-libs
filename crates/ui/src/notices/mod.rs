//! `Notice` (inline, card and centred layouts), the settings notices and the notice center. The toast queue is
//! `runtime::toast`, shown by the shell's `ToastHost`.

mod center;
mod notice;
mod settings;
mod words;

#[cfg(feature = "testing")]
mod stories;

pub use center::{NoticeCenter, PostedNotice};
pub use notice::{Notice, NoticeAction, NoticeIcon, NoticeLayout, NoticeOutput, NoticeStyle};
pub use settings::{SaveResult, SettingsLoad, SettingsNoticeState, SettingsNotices, SettingsStatus};
pub use words::NoticeWords;
