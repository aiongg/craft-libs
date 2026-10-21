use std::ffi::OsString;

/// Windowing backend for Linux, FreeBSD and the other free Unix platforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum LinuxBackend {
    /// Native Wayland: text-input-v3 for input methods, fractional scaling, no XWayland blur.
    Wayland,
    /// X11, or XWayland on a Wayland session: input methods go through XIM.
    X11,
}

impl LinuxBackend {
    /// Parses `wayland` or `x11` (also `xwayland`, `xorg`), ignoring case and surrounding space.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wayland" => Some(Self::Wayland),
            "x11" | "xwayland" | "xorg" => Some(Self::X11),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wayland => "wayland",
            Self::X11 => "x11",
        }
    }
}

/// Environment variable that forces a backend: `wayland` or `x11`.
///
/// It takes precedence over the app's setting so a user can recover from a broken backend without
/// opening the app.
pub const BACKEND_ENV: &str = "CRAFT_LINUX_BACKEND";

/// The environment variables [`choose_linux_backend`] reads, captured so the choice is testable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendEnv {
    /// [`BACKEND_ENV`].
    pub forced: Option<OsString>,
    pub wayland_display: Option<OsString>,
    pub wayland_socket: Option<OsString>,
    pub x11_display: Option<OsString>,
}

impl BackendEnv {
    pub fn from_process() -> Self {
        Self {
            forced: std::env::var_os(BACKEND_ENV),
            wayland_display: std::env::var_os("WAYLAND_DISPLAY"),
            wayland_socket: std::env::var_os("WAYLAND_SOCKET"),
            x11_display: std::env::var_os("DISPLAY"),
        }
    }

    fn has_wayland(&self) -> bool {
        is_set(&self.wayland_display) || is_set(&self.wayland_socket)
    }

    fn has_x11(&self) -> bool {
        is_set(&self.x11_display)
    }
}

fn is_set(value: &Option<OsString>) -> bool {
    value.as_ref().is_some_and(|v| !v.is_empty())
}

/// Chooses the backend for this process: [`BACKEND_ENV`], then the app's `setting`, then native
/// Wayland on a Wayland session and X11 otherwise.
///
/// See [`choose_linux_backend`] for the rules and the module docs for how to apply the result.
pub fn linux_backend(setting: Option<LinuxBackend>) -> LinuxBackend {
    choose_linux_backend(&BackendEnv::from_process(), setting)
}

/// Pure form of [`linux_backend`].
///
/// - A valid [`BACKEND_ENV`] wins over `setting`; an unrecognised value is ignored.
/// - Without either, the session decides: Wayland when `WAYLAND_DISPLAY` or `WAYLAND_SOCKET` is
///   set (the same test winit uses), otherwise X11.
/// - A forced backend whose display is missing falls back to the one that exists, so forcing X11
///   on a Wayland session without XWayland still opens a window.
pub fn choose_linux_backend(env: &BackendEnv, setting: Option<LinuxBackend>) -> LinuxBackend {
    let forced = env.forced.as_ref().and_then(|value| value.to_str()).and_then(LinuxBackend::parse);
    let auto = if env.has_wayland() { LinuxBackend::Wayland } else { LinuxBackend::X11 };
    match forced.or(setting) {
        Some(LinuxBackend::X11) if !env.has_x11() && env.has_wayland() => LinuxBackend::Wayland,
        Some(LinuxBackend::Wayland) if !env.has_wayland() && env.has_x11() => LinuxBackend::X11,
        Some(choice) => choice,
        None => auto,
    }
}
