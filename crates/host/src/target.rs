//! The target platform, egui-free so layers below the UI read it.

/// The operating system of the host, also in a browser: shortcut labels in a browser on a Mac read "Mac".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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

impl Os {
    /// The system this build runs on. Web builds count as Linux; with the `ui` feature,
    /// [`Os::of_egui`] gives the browser's operating system.
    pub const fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }

    /// Whether this is macOS.
    pub const fn is_mac(self) -> bool {
        matches!(self, Os::Mac)
    }

    /// The system egui reports: iOS counts as Mac; Android and unknown systems count as Linux.
    #[cfg(feature = "ui")]
    pub fn of_egui(os: egui::os::OperatingSystem) -> Os {
        use egui::os::OperatingSystem;
        match os {
            OperatingSystem::Mac | OperatingSystem::IOS => Os::Mac,
            OperatingSystem::Windows => Os::Windows,
            OperatingSystem::Nix | OperatingSystem::Android | OperatingSystem::Unknown => Os::Linux,
        }
    }
}
