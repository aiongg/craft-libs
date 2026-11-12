//! The monitors and the ICC profile the system assigns to each, for the colour-managed canvas
//! (`craft_display` resolves and applies them) and the screen colour picker.
//!
//! - X11: RandR monitors with their frames; each output's `_ICC_PROFILE` property (the ICC
//!   Profiles in X specification; the root window's for the primary monitor), and colord's
//!   profile for an output without one, matched by colord's exact `XRANDR_name` metadata. A
//!   monitor showing several outputs (clone mode) gets a profile only when every output has the
//!   same bytes.
//! - Wayland (also an Xwayland window in a Wayland session): the compositor tells no client which
//!   output shows it, so there is one monitor without a frame. Its profile is the one colord
//!   assigns to every display; with different profiles on different displays it has none rather
//!   than a guess.
//! - Other platforms have no reader here: [`read`] answers [`MonitorsError::Unsupported`].
//!
//! A read asks the X server and the system bus (colord with a 2 s timeout), so it runs off the UI
//! thread.

use std::sync::Arc;

/// A monitor as the system reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    /// The system's id for it (X11: the RandR monitor's name atom; Wayland: 0).
    pub id: u32,
    /// "DP-1", "Built-in Retina Display".
    pub name: String,
    /// `[x, y, width, height]` in the coordinates the system places windows in (X11: physical
    /// pixels), y down; all zero when the system doesn't say (Wayland).
    pub frame: [f64; 4],
    /// The primary monitor (menu bar, X11 primary output).
    pub primary: bool,
    /// The system's name for the profile (colord: its file name), when known.
    pub profile_name: Option<String>,
    /// The monitor's ICC profile: header-checked as a plausible RGB profile and truncated to its
    /// declared size, not parsed. `None` when the system assigns none.
    pub icc: Option<Arc<Vec<u8>>>,
}

/// Why [`read`] has no monitors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonitorsError {
    /// No reader for this platform or display session.
    Unsupported,
    /// The reader ran and failed, with its message.
    Failed(String),
}

/// Every monitor, primary first.
pub fn read() -> Result<Vec<Monitor>, MonitorsError> {
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    {
        linux::read()
    }
    #[cfg(not(all(unix, not(target_os = "macos"), not(target_arch = "wasm32"))))]
    {
        Err(MonitorsError::Unsupported)
    }
}

/// The longest ICC profile read from a file (profiles with large lookup tables run to a few MB).
#[cfg_attr(not(all(unix, not(target_os = "macos"), not(target_arch = "wasm32"))), allow(dead_code))]
const MAX_ICC_FILE_BYTES: usize = 64 * 1024 * 1024;

/// `bytes` when they are a plausible RGB ICC profile no longer than `max`: the `acsp` signature,
/// an `RGB ` data colour space and a declared size within the bytes (the bytes are cut to it).
#[cfg_attr(not(all(unix, not(target_os = "macos"), not(target_arch = "wasm32"))), allow(dead_code))]
pub(crate) fn checked_icc(mut bytes: Vec<u8>, max: usize) -> Option<Vec<u8>> {
    if bytes.len() > max || bytes.len() < 132 || bytes.get(36..40) != Some(b"acsp") || bytes.get(16..20) != Some(b"RGB ") {
        return None;
    }
    let declared = u32::from_be_bytes(bytes.get(0..4)?.try_into().ok()?) as usize;
    if declared < 132 || declared > bytes.len() {
        return None;
    }
    bytes.truncate(declared);
    Some(bytes)
}

/// The profile of a monitor showing `outputs`: the one they all have, `None` when one has none or
/// two differ.
#[cfg_attr(not(all(unix, not(target_os = "macos"), not(target_arch = "wasm32"))), allow(dead_code))]
fn unambiguous<'a, P: PartialEq>(mut profiles: impl Iterator<Item = Option<&'a P>>) -> Option<&'a P> {
    let first = profiles.next()??;
    for p in profiles {
        if p? != first {
            return None;
        }
    }
    Some(first)
}

#[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
pub(crate) mod linux {
    use std::collections::HashMap;
    use std::io::Read as _;
    use std::sync::Arc;
    use std::time::Duration;

    use x11rb::connection::{Connection as _, RequestConnection};
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};
    use zbus::blocking::Proxy;
    use zbus::blocking::connection::Builder;
    use zbus::zvariant::OwnedObjectPath;

    use super::{MAX_ICC_FILE_BYTES, Monitor, MonitorsError, checked_icc, unambiguous};

    /// The longest `_ICC_PROFILE` property read (the length limit of one property request).
    const MAX_ICC_PROPERTY_BYTES: usize = 1024 * 1024;
    /// Length limit of the property reads, in 32-bit units.
    const ICC_PROPERTY_LONG_LENGTH: u32 = 256 * 1024;
    /// The most colord display devices looked at.
    const MAX_COLORD_DEVICES: usize = 64;

    const COLORD: &str = "org.freedesktop.ColorManager";
    const COLORD_ROOT: &str = "/org/freedesktop/ColorManager";
    const COLORD_DEVICE: &str = "org.freedesktop.ColorManager.Device";
    const COLORD_PROFILE: &str = "org.freedesktop.ColorManager.Profile";

    /// A profile and the name the system gives it.
    #[derive(Clone, PartialEq)]
    struct Icc {
        name: Option<String>,
        bytes: Arc<Vec<u8>>,
    }

    /// The session's kind, as the screen capture decides it too.
    pub(crate) fn wayland_session() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland")
    }

    pub(super) fn read() -> Result<Vec<Monitor>, MonitorsError> {
        if wayland_session() {
            return Ok(vec![wayland()]);
        }
        if !std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty()) {
            return Err(MonitorsError::Unsupported);
        }
        let (conn, index) = x11rb::connect(None).map_err(|e| MonitorsError::Failed(format!("couldn't connect to the X11 display: {e}")))?;
        let root = conn.setup().roots.get(index).ok_or_else(|| MonitorsError::Failed("the X11 server didn't provide a screen".into()))?.root;
        let monitors = x11(&conn, root);
        if monitors.is_empty() {
            return Err(MonitorsError::Failed("X11 reported no RandR monitors".into()));
        }
        Ok(monitors)
    }

    /// The RandR monitors of the screen with root window `root`, primary first, each with its
    /// profile. Also the screen capture's list of regions.
    pub(crate) fn x11<C: RequestConnection>(conn: &C, root: u32) -> Vec<Monitor> {
        let Some(reply) = conn.randr_get_monitors(root, true).ok().and_then(|c| c.reply().ok()) else { return Vec::new() };
        let atom = atom(conn, "_ICC_PROFILE");
        let names = output_names(conn, root);
        let mut colord: Option<HashMap<String, Icc>> = None;
        let mut monitors = Vec::with_capacity(reply.monitors.len());
        for m in reply.monitors {
            let name = conn
                .get_atom_name(m.name)
                .ok()
                .and_then(|c| c.reply().ok())
                .map(|r| String::from_utf8_lossy(&r.name).into_owned())
                .unwrap_or_default();
            let profiles: Vec<Option<Icc>> = m
                .outputs
                .iter()
                .map(|&output| {
                    let own = atom.and_then(|a| output_profile(conn, output, a)).map(|bytes| Icc { name: None, bytes: Arc::new(bytes) });
                    own.or_else(|| {
                        let output_name = names.get(&output)?;
                        colord.get_or_insert_with(colord_by_output).get(output_name).cloned()
                    })
                })
                .collect();
            let mut icc = unambiguous(profiles.iter().map(Option::as_ref)).cloned();
            if icc.is_none() && m.primary && m.outputs.is_empty() {
                icc = atom.and_then(|a| root_profile(conn, root, a)).map(|bytes| Icc { name: None, bytes: Arc::new(bytes) });
            }
            monitors.push(Monitor {
                id: m.name,
                name,
                frame: [f64::from(m.x), f64::from(m.y), f64::from(m.width), f64::from(m.height)],
                primary: m.primary,
                profile_name: icc.as_ref().and_then(|p| p.name.clone()),
                icc: icc.map(|p| p.bytes),
            });
        }
        monitors.sort_by_key(|m| !m.primary);
        if let Some(first) = monitors.first_mut() {
            first.primary = true;
        }
        monitors
    }

    fn atom<C: RequestConnection>(conn: &C, name: &str) -> Option<u32> {
        let atom = conn.intern_atom(true, name.as_bytes()).ok()?.reply().ok()?.atom;
        (atom != 0).then_some(atom)
    }

    /// RandR output → its name ("DP-1"), the key colord's `XRANDR_name` metadata uses.
    fn output_names<C: RequestConnection>(conn: &C, root: u32) -> HashMap<u32, String> {
        let Some(resources) = conn.randr_get_screen_resources_current(root).ok().and_then(|c| c.reply().ok()) else { return HashMap::new() };
        let mut names = HashMap::new();
        for output in resources.outputs {
            let Some(info) = conn.randr_get_output_info(output, resources.config_timestamp).ok().and_then(|c| c.reply().ok()) else { continue };
            if let Ok(name) = String::from_utf8(info.name)
                && !name.is_empty()
            {
                names.insert(output, name);
            }
        }
        names
    }

    fn output_profile<C: RequestConnection>(conn: &C, output: u32, atom: u32) -> Option<Vec<u8>> {
        let reply = conn.randr_get_output_property(output, atom, AtomEnum::CARDINAL, 0, ICC_PROPERTY_LONG_LENGTH, false, false).ok()?.reply().ok()?;
        if reply.type_ != u32::from(AtomEnum::CARDINAL) || reply.format != 8 || reply.bytes_after != 0 {
            return None;
        }
        checked_icc(reply.data, MAX_ICC_PROPERTY_BYTES)
    }

    fn root_profile<C: RequestConnection>(conn: &C, root: u32, atom: u32) -> Option<Vec<u8>> {
        let reply = conn.get_property(false, root, atom, AtomEnum::ANY, 0, ICC_PROPERTY_LONG_LENGTH).ok()?.reply().ok()?;
        if reply.format != 8 || reply.bytes_after != 0 {
            return None;
        }
        checked_icc(reply.value, MAX_ICC_PROPERTY_BYTES)
    }

    /// One monitor carrying the profile every colord display shares (none when they differ).
    fn wayland() -> Monitor {
        let profiles = match colord_displays() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("couldn't read the display profiles from colord: {e}");
                Vec::new()
            }
        };
        let icc = unambiguous(profiles.iter().map(|(_, p)| Some(p))).cloned();
        Monitor {
            id: 0,
            name: "Display".into(),
            frame: [0.0; 4],
            primary: true,
            profile_name: icc.as_ref().and_then(|p| p.name.clone()),
            icc: icc.map(|p| p.bytes),
        }
    }

    /// colord's profile per X11 output name; empty (with a warning) without colord.
    fn colord_by_output() -> HashMap<String, Icc> {
        let displays = colord_displays().unwrap_or_else(|e| {
            log::warn!("couldn't read the X11 display profiles from colord: {e}");
            Vec::new()
        });
        let mut by_output: HashMap<String, Vec<Icc>> = HashMap::new();
        for (output, icc) in displays {
            if let Some(output) = output {
                by_output.entry(output).or_default().push(icc);
            }
        }
        // Several devices claiming one output with different profiles: no profile, not a guess.
        by_output.into_iter().filter_map(|(output, p)| unambiguous(p.iter().map(Some)).cloned().map(|icc| (output, icc))).collect()
    }

    /// Every colord display device that has a default profile: its `XRANDR_name` (when set) and
    /// the profile.
    fn colord_displays() -> Result<Vec<(Option<String>, Icc)>, String> {
        let conn = Builder::system()
            .map_err(|e| format!("couldn't connect to the system bus: {e}"))?
            .method_timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| format!("couldn't connect to the system bus: {e}"))?;
        let manager = Proxy::new(&conn, COLORD, COLORD_ROOT, COLORD).map_err(|e| format!("couldn't access colord: {e}"))?;
        let devices: Vec<OwnedObjectPath> =
            manager.call("GetDevicesByKind", &("display",)).map_err(|e| format!("colord couldn't list display devices: {e}"))?;
        let mut displays = Vec::new();
        for path in devices.iter().take(MAX_COLORD_DEVICES) {
            let Ok(device) = Proxy::new(&conn, COLORD, path, COLORD_DEVICE) else { continue };
            let output =
                device.get_property::<HashMap<String, String>>("Metadata").ok().and_then(|m| m.get("XRANDR_name").cloned()).filter(|n| !n.is_empty());
            // `*` is colord's default qualifier: the device's default profile.
            let profile_path: OwnedObjectPath = match device.call("GetProfileForQualifiers", &(vec!["*"],)) {
                Ok(path) => path,
                Err(_) => continue,
            };
            let Ok(profile) = Proxy::new(&conn, COLORD, &profile_path, COLORD_PROFILE) else { continue };
            let filename = match profile.get_property::<String>("Filename") {
                Ok(f) if !f.is_empty() => f,
                _ => continue,
            };
            match read_icc(std::path::Path::new(&filename)) {
                Some(icc) => displays.push((output, icc)),
                None => log::warn!("couldn't read a valid ICC profile colord assigns to a display ({filename})"),
            }
        }
        Ok(displays)
    }

    fn read_icc(path: &std::path::Path) -> Option<Icc> {
        let mut bytes = Vec::new();
        std::fs::File::open(path).ok()?.take((MAX_ICC_FILE_BYTES + 1) as u64).read_to_end(&mut bytes).ok()?;
        let bytes = checked_icc(bytes, MAX_ICC_FILE_BYTES)?;
        let name = path.file_name().and_then(|n| n.to_str()).map(str::to_owned);
        Some(Icc { name, bytes: Arc::new(bytes) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icc(space: &[u8; 4], len: usize, declared: usize) -> Vec<u8> {
        let mut b = vec![0u8; len];
        b[0..4].copy_from_slice(&(declared as u32).to_be_bytes());
        b[16..20].copy_from_slice(space);
        b[36..40].copy_from_slice(b"acsp");
        b
    }

    /// Guards the header check every reader shares: a profile with trailing bytes is cut to its
    /// declared size, and a non-RGB, unsigned, truncated or oversized one is never handed on.
    #[test]
    fn icc_bytes_are_bounded_rgb_profiles() {
        assert_eq!(checked_icc(icc(b"RGB ", 200, 200), 1024).map(|b| b.len()), Some(200));
        assert_eq!(checked_icc(icc(b"RGB ", 300, 200), 1024).map(|b| b.len()), Some(200));
        assert!(checked_icc(icc(b"CMYK", 200, 200), 1024).is_none());
        assert!(checked_icc(icc(b"RGB ", 200, 400), 1024).is_none());
        assert!(checked_icc(icc(b"RGB ", 200, 100), 1024).is_none());
        assert!(checked_icc(icc(b"RGB ", 2000, 2000), 1024).is_none());
        assert!(checked_icc(vec![0; 200], 1024).is_none());
    }

    /// Guards the clone rule: a mirrored monitor never shows one output's profile when another
    /// output has a different one or none, and colord's devices for one output must agree.
    #[test]
    fn cloned_outputs_use_only_one_unambiguous_profile() {
        let (a, b) = ("sRGB", "Display P3");
        assert_eq!(unambiguous([Some(&a), Some(&a)].into_iter()), Some(&a));
        assert_eq!(unambiguous([Some(&a), Some(&b)].into_iter()), None);
        assert_eq!(unambiguous([Some(&a), None].into_iter()), None);
        assert_eq!(unambiguous([None, Some(&a)].into_iter()), None);
        assert_eq!(unambiguous(std::iter::empty::<Option<&&str>>()), None);
    }
}
