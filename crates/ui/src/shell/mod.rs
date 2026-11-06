//! `AppShell` and its presets, `TitleBar`, captions, window zones, `ToastHost` and `app::run`.

mod words;
#[cfg(feature = "app")]
pub mod app;
#[cfg(target_os = "macos")]
pub mod title_bar;
