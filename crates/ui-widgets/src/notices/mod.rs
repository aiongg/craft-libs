//! `Notice` (inline, card and centred layouts), the settings notices and the notice center. The toast queue is
//! `runtime::toast`, drawn by `ToastStack` through the contract's `ToastHost`.

#[cfg(spike_full)]
pub(crate) mod center;
pub(crate) mod notice;
#[cfg(spike_full)]
pub(crate) mod settings;
#[cfg(spike_full)]
pub(crate) mod toasts;
mod words;

#[cfg(feature = "suite")]
#[cfg(spike_full)]
pub(crate) mod stories;

#[cfg(spike_full)]
pub use center::{NoticeCenter, NoticeCenterOutput, NoticeEnd, PostedNotice};
pub(crate) use notice::tone_icon;
pub use notice::{Notice, NoticeAction, NoticeIcon, NoticeLayout, NoticeOutput, NoticeStyle};
#[cfg(spike_full)]
pub use settings::{SaveResult, SettingsLoad, SettingsNoticeState, SettingsNotices, SettingsNoticesOutput, SettingsStatus};
#[cfg(spike_full)]
pub use toasts::ToastStack;
pub use words::NoticeWords;
