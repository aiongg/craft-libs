#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Making the app the system's default for its file types, and reading the paths the system
//! hands it when it is (N10.04).
//!
//! The installers register an app for its types without taking over the defaults (WordCraft's
//! `.wxs`, the `.desktop` file's `MimeType=`, `Info.plist`). [`make_default`] is the button in
//! Preferences that does it when the user asks:
//!
//! - **Windows** doesn't let apps set defaults: it opens Settings › Default apps on the app's
//!   page (`ms-settings:defaultapps?registeredAppMachine=<Name>`; Windows 10 ignores the query
//!   and shows the list), where the user picks the app per type: [`DefaultApp::SettingsOpened`].
//! - **Linux and BSD**: `xdg-mime default <desktop id> <types>`, which writes the user's
//!   `mimeapps.list`: [`DefaultApp::MadeDefault`]. Only when the desktop entry is installed
//!   (deb, rpm, Flatpak; a bare AppImage has none), and not from inside a Flatpak sandbox, which
//!   can't change the host's defaults.
//! - **macOS** has no API for it here: [`make_default`] answers [`DefaultAppError::Unsupported`]
//!   and the app's Preferences explain Finder's Get Info › Change All.
//!
//! Only a button may call it, never a command: nothing programmatic (automation, macros) changes
//! system defaults (WordCraft's rule).
//!
//! [`path_from_arg`] and [`rejoin_split_paths`] read what a launcher passes once the app is the
//! default (`file://` URIs from GNOME Files, unquoted paths with spaces from Windows programs).

use std::path::PathBuf;

use crate::PlatformText;

/// Who the app is to the system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppIdentity {
    /// The name under `HKLM\Software\RegisteredApplications` (Windows), e.g. `WordCraft`.
    pub registered_name: String,
    /// The freedesktop desktop entry, e.g. `ai.storyteller.wordcraft.desktop`; also the
    /// Flatpak's app id without `.desktop`.
    pub desktop_id: String,
    /// The MIME types to take over on Linux and BSD, every one listed in the desktop entry's
    /// `MimeType=` (WordCraft: Word documents and templates, RTF, ODT).
    pub mime_types: Vec<String>,
}

/// What [`make_default`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultApp {
    /// The system's Default apps settings opened; the user picks there (Windows).
    SettingsOpened,
    /// The app is now the default for its types (Linux and BSD).
    MadeDefault,
}

/// Why nothing changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefaultAppError {
    /// The platform has no way here (macOS, the web).
    Unsupported,
    /// Linux: the desktop entry isn't installed in any `applications` folder (an AppImage).
    NotInstalled,
    /// Linux: running inside a Flatpak sandbox.
    Sandboxed,
    /// What the system said, cut to 200 characters (WordCraft's `MAX_ERROR`).
    Failed(String),
}

/// Makes the app the default for its types, or opens the system page that does. Blocks for the
/// `xdg-mime` run (well under a second); call it from the button's click.
// wordcraft: apps/wordcraft/src/default_apps.rs:23 hook, :90 open_default_apps, :107 make_default
pub fn make_default(app: &AppIdentity) -> Result<DefaultApp, DefaultAppError> {
    todo!()
}

/// The `applications` folders the desktop looks for entries in, from `XDG_DATA_HOME` (default
/// `~/.local/share`) and `XDG_DATA_DIRS` (default `/usr/local/share:/usr/share`). Relative paths
/// are ignored, as the XDG Base Directory spec says.
// wordcraft: apps/wordcraft/src/default_apps.rs:66 application_dirs
pub fn application_dirs(data_home: Option<&str>, data_dirs: Option<&str>, home: Option<&str>) -> Vec<PathBuf> {
    todo!()
}

/// The path in one launch argument. A local `file://` URI (`file:///path`,
/// `file://localhost/path`, `file:///C:/x.pdf` on Windows) is percent-decoded to a path, since
/// GNOME Files and Dolphin only treat an app as a default handler when its `Exec` takes URIs
/// (`%U`). Other schemes, URIs naming another host and plain paths pass through unchanged, as
/// does a URI whose decoded bytes aren't UTF-8.
// pdfcraft: apps/pdfcraft/src/main.rs:141 path_from_arg
pub fn path_from_arg(arg: String) -> String {
    todo!()
}

/// Launch arguments with unquoted paths put back together. Some programs start the default app
/// with the path unquoted (`pdfcraft.exe P:\Docs\01 Contract.PDF`), so a path with spaces
/// arrives split. When an argument names no file but it and the ones after it, joined with one
/// space, do, they are rejoined (the longest such run, at most 32 arguments); anything else is
/// kept as given. Runs of several spaces can't be recovered.
// pdfcraft: apps/pdfcraft/src/main.rs:171 rejoin_split_paths
pub fn rejoin_split_paths(args: Vec<String>, exists: impl Fn(&str) -> bool) -> Vec<String> {
    todo!()
}

/// After [`DefaultApp::SettingsOpened`]; `{app}` is the app's name.
pub const DEFAULT_SETTINGS_OPENED: PlatformText =
    PlatformText { id: "defaultApp.settingsOpened", english: "In Default apps, choose {app} for the file types you want it to open." };
/// After [`DefaultApp::MadeDefault`]; `{app}` and `{types}` ("Word documents").
pub const DEFAULT_MADE: PlatformText = PlatformText { id: "defaultApp.made", english: "{app} is now the default app for {types}." };
/// A [`DefaultAppError`]; `{app}` and `{error}`.
pub const DEFAULT_FAILED: PlatformText = PlatformText { id: "defaultApp.failed", english: "Couldn't make {app} the default: {error}" };
/// [`DefaultAppError::Unsupported`] on macOS, shown in Preferences instead of the button.
pub const DEFAULT_ON_MAC: PlatformText = PlatformText {
    id: "defaultApp.mac",
    english: "To open files with {app}, select one in the Finder, choose File › Get Info, pick {app} under Open with, and click Change All.",
};
