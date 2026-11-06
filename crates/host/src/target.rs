//! The target platform, egui-free so layers below the UI read it.

/// The operating system of the host, also in a browser: shortcut labels in a browser on a Mac read "Mac".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Os {
    /// macOS.
    Mac,
    /// Windows.
    Windows,
    /// Linux and every other system.
    #[default]
    Linux,
}
