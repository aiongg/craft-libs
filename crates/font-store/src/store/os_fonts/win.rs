//! The per-user Windows font folder (Windows 10 1809 and later): copies are registered under
//! `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts`, one `REG_SZ` value per file holding
//! its full path, and Windows loads them at every sign-in.

use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use windows_registry::{CURRENT_USER, LOCAL_MACHINE};

const FONTS_KEY: &str = r"Software\Microsoft\Windows NT\CurrentVersion\Fonts";
/// Windows 10 1809, the first build with per-user fonts.
const FIRST_BUILD: u32 = 17763;

/// Why this process can't add fonts for the user's other apps; `Ok` when it can.
pub(super) fn support() -> Result<(), String> {
    let build = LOCAL_MACHINE
        .open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|k| k.get_string("CurrentBuildNumber"))
        .ok()
        .and_then(|b| b.trim().parse::<u32>().ok());
    match build {
        None => return Err("the Windows version can't be read".into()),
        Some(b) if b < FIRST_BUILD => {
            return Err(format!("Windows 10 version 1809 (build {FIRST_BUILD}) or later is needed for per-user fonts; this is build {b}"));
        }
        Some(_) => {}
    }
    if in_app_container() {
        return Err("an app in an AppContainer can't write the user font folder or its registry key".into());
    }
    if packaged() {
        return Err("a packaged (MSIX) app's font folder and registry writes are its own, so other apps wouldn't see the fonts".into());
    }
    Ok(())
}

fn in_app_container() -> bool {
    use winsafe::{HACCESSTOKEN, TokenInfo, co};
    matches!(
        HACCESSTOKEN::GetCurrentProcessToken().GetTokenInformation(co::TOKEN_INFORMATION_CLASS::IsAppContainer),
        Ok(TokenInfo::IsAppContainer(v)) if *v != 0
    )
}

/// Whether the app runs from an MSIX package, which Windows installs under `WindowsApps`.
fn packaged() -> bool {
    std::env::current_exe().is_ok_and(|p| p.components().any(|c| c.as_os_str().eq_ignore_ascii_case("WindowsApps")))
}

/// Whether the registry data `data` names `path`. Windows paths ignore case.
pub(super) fn names(data: &str, path: &Path) -> bool {
    path.to_str().is_some_and(|p| p.to_lowercase() == data.to_lowercase())
}

/// The current user's font registrations: each value's name in lower case (Windows compares
/// them ignoring case), and its data.
pub(super) fn registered() -> HashMap<String, String> {
    let Ok(key) = CURRENT_USER.open(FONTS_KEY) else { return HashMap::new() };
    key.values()
        .map(|values| values.filter_map(|(name, value)| Some((name.to_lowercase(), String::try_from(value).ok()?))).collect())
        .unwrap_or_default()
}

/// Registers the copy at `path` under `name`. Nothing changes when it is registered already.
pub(super) fn register(name: &str, path: &Path) -> std::io::Result<()> {
    let data = path.to_str().ok_or_else(|| std::io::Error::other("the path isn't valid Unicode"))?;
    let key = CURRENT_USER.create(FONTS_KEY)?;
    if key.get_string(name).is_ok_and(|d| names(&d, path)) {
        return Ok(());
    }
    Ok(key.set_string(name, data)?)
}

/// Deletes the registration `name` while it names `path`; one that names another file stays.
pub(super) fn unregister(name: &str, path: &Path) -> std::io::Result<()> {
    let Ok(key) = CURRENT_USER.create(FONTS_KEY) else { return Ok(()) };
    match key.get_string(name) {
        Ok(d) if names(&d, path) => Ok(key.remove_value(name)?),
        _ => Ok(()),
    }
}

/// Grants `ALL APPLICATION PACKAGES` and `ALL RESTRICTED APPLICATION PACKAGES` read and execute
/// access to `dir` and everything in it, as Windows does for its own per-user font folder:
/// without it, packaged apps (and from Windows 11 22H2 on, Windows' own apps) can't read the
/// fonts. Best effort: desktop apps read them either way.
pub(super) fn open_to_packaged_apps(dir: &Path) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let system = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir")).map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    let _ = Command::new(system.join("System32").join("icacls.exe"))
        .arg(dir)
        .args(["/grant", "*S-1-15-2-1:(OI)(CI)(RX)", "*S-1-15-2-2:(OI)(CI)(RX)", "/Q"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
