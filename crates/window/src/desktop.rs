//! Which desktop and backend the window opens on, read from the environment and, on Wayland, the
//! compositor's registry.

/// The windowing backend the app opens its window on. An app that forces XWayland on a Wayland
/// session passes [`Backend::X11`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
}

/// The kind of desktop, as far as the frame decision cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopKind {
    Gnome,
    Kde,
    /// A tiling compositor or window manager (Sway, Hyprland, i3, niri, river, ...).
    Tiling,
    /// Any other desktop: COSMIC, Xfce, Cinnamon, MATE, Budgie, an unknown one.
    Other,
}

/// What the running desktop is and offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Desktop {
    pub kind: DesktopKind,
    /// Whether the compositor offers server-side decorations (`zxdg_decoration_manager_v1`).
    /// `None` when not probed (X11, or the probe failed).
    pub ssd: Option<bool>,
    /// The GTK theme name, when GTK's `settings.ini` or `GTK_THEME` names one.
    pub gtk_theme: Option<String>,
}

impl Desktop {
    /// Read the desktop. Reads environment variables and small config files; the Wayland registry
    /// is probed (500 ms at most) only when `backend` is [`Backend::Wayland`].
    pub fn detect(backend: Backend) -> Desktop {
        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
        {
            let var = |k: &str| std::env::var(k).ok();
            let ssd = if backend == Backend::Wayland { crate::platform::wayland_offers_ssd() } else { None };
            Desktop { kind: kind_from_env(&var), ssd, gtk_theme: crate::layout::read_gtk_theme(&var) }
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd")))]
        {
            let _ = backend;
            Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", test))]
/// Compositors and window managers that tile, matched against the `XDG_CURRENT_DESKTOP` entries.
const TILING: &[&str] = &["sway", "hyprland", "i3", "niri", "river", "bspwm", "awesome", "dwl", "qtile", "herbstluftwm", "leftwm", "xmonad", "dwm"];

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", test))]
/// The desktop kind from the first of its variables that is set (`XDG_CURRENT_DESKTOP`,
/// `XDG_SESSION_DESKTOP`, `DESKTOP_SESSION`). `ubuntu:GNOME` and `Budgie:GNOME` count as GNOME.
pub(crate) fn kind_from_env(var: &dyn Fn(&str) -> Option<String>) -> DesktopKind {
    for key in ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"] {
        let Some(v) = var(key).map(|v| v.to_ascii_lowercase()).filter(|v| !v.trim().is_empty()) else { continue };
        if v.split(':').map(str::trim).any(|entry| TILING.contains(&entry)) {
            return DesktopKind::Tiling;
        }
        return if v.contains("gnome") {
            DesktopKind::Gnome
        } else if v.contains("kde") || v.contains("plasma") {
            DesktopKind::Kde
        } else {
            DesktopKind::Other
        };
    }
    if var("KDE_FULL_SESSION").is_some() {
        DesktopKind::Kde
    } else if var("GNOME_DESKTOP_SESSION_ID").is_some() {
        DesktopKind::Gnome
    } else {
        DesktopKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let pairs: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn the_desktop_comes_from_the_first_variable_set() {
        assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", "ubuntu:GNOME")])), DesktopKind::Gnome);
        assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", "KDE")])), DesktopKind::Kde);
        assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", "X-Cinnamon"), ("KDE_FULL_SESSION", "true")])), DesktopKind::Other);
        assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", " "), ("DESKTOP_SESSION", "plasma")])), DesktopKind::Kde);
        assert_eq!(kind_from_env(&env(&[("KDE_FULL_SESSION", "true")])), DesktopKind::Kde);
        assert_eq!(kind_from_env(&env(&[("GNOME_DESKTOP_SESSION_ID", "x")])), DesktopKind::Gnome);
        assert_eq!(kind_from_env(&env(&[])), DesktopKind::Other);
    }

    #[test]
    fn tiling_compositors_are_recognised() {
        for name in ["sway", "Hyprland", "i3", "niri", "river", "bspwm"] {
            assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", name)])), DesktopKind::Tiling, "{name}");
        }
        assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", "i3:GNOME-Flashback")])), DesktopKind::Tiling);
        assert_eq!(kind_from_env(&env(&[("XDG_CURRENT_DESKTOP", "COSMIC")])), DesktopKind::Other);
    }
}
