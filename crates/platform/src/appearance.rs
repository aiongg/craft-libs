//! Desktop colour, contrast and motion preferences and GNOME's window-button layout through the
//! XDG Settings portal, plus the browser's contrast and motion media queries on the web.
//!
//! Derived from PdfCraft's activation-safe subscription and GNOME schema fallback,
//! CADCraft's BSD support, and ArtCraft Launcher's ReadOne/Read compatibility.
//! Palettes, manual choices and the fallback when no preference exists belong to the host.
mod reader;
pub use reader::{Reading, SystemReader};
use std::{
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::Duration,
};

/// Desktop color preference observed independently of the application palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    /// The desktop requests a dark appearance.
    Dark,
    /// The desktop requests a light appearance.
    Light,
}

/// Desktop contrast preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contrast {
    /// The desktop has no contrast preference.
    Standard,
    /// The desktop requests higher contrast.
    High,
}

struct Shared {
    value: AtomicU8,
    /// 0 unknown, 1 standard, 2 high.
    contrast: AtomicU8,
    /// 0 unknown, 1 animations allowed, 2 reduced motion.
    motion: AtomicU8,
    active: AtomicBool,
    /// True once the source has answered, found no answer, or been cancelled.
    settled: Mutex<bool>,
    settled_changed: Condvar,
    /// GNOME's `button-layout` as the desktop reported it; `None` until an answer arrives.
    layout: Mutex<Option<String>>,
    /// GNOME's `icon-theme` name as the desktop reported it; `None` until an answer arrives.
    icons: Mutex<Option<String>>,
    /// `XDG_CURRENT_DESKTOP` names GNOME; read once when the watcher starts.
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    gnome: bool,
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    wake: Box<dyn Fn() + Send + Sync>,
}
impl Shared {
    fn get(&self) -> Option<Theme> {
        match self.value.load(Ordering::Acquire) {
            1 => Some(Theme::Dark),
            2 => Some(Theme::Light),
            _ => None,
        }
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn settle(&self) {
        *self.settled.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.settled_changed.notify_all();
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish(&self, theme: Option<Theme>) {
        let code = match theme {
            Some(Theme::Dark) => 1,
            Some(Theme::Light) => 2,
            None => 0,
        };
        let changed = self.value.swap(code, Ordering::AcqRel) != code;
        self.settle();
        if changed && self.active.load(Ordering::Acquire) {
            (self.wake)();
        }
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish_code(&self, slot: &AtomicU8, code: u8) {
        if slot.swap(code, Ordering::AcqRel) != code && self.active.load(Ordering::Acquire) {
            (self.wake)();
        }
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish_contrast(&self, contrast: Contrast) {
        self.publish_code(
            &self.contrast,
            match contrast {
                Contrast::Standard => 1,
                Contrast::High => 2,
            },
        );
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish_motion(&self, reduced: bool) {
        self.publish_code(&self.motion, if reduced { 2 } else { 1 });
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish_layout(&self, layout: String) {
        self.publish_text(&self.layout, layout);
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish_icons(&self, icons: String) {
        // The GNOME key holds a value on every desktop that has the schema (`gsettings` answers
        // "Adwaita" on KDE), but it is the user's setting only on GNOME-like desktops.
        if self.gnome {
            self.publish_text(&self.icons, icons);
        }
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn publish_text(&self, slot: &Mutex<Option<String>>, text: String) {
        let mut current = slot.lock().unwrap_or_else(PoisonError::into_inner);
        if current.as_deref() == Some(text.as_str()) {
            return;
        }
        *current = Some(text);
        drop(current);
        if self.active.load(Ordering::Acquire) {
            (self.wake)();
        }
    }
}
/// An owned subscription. Dropping cancels connection setup, reads, backoff and helper capture,
/// and waits at most 100 ms for acknowledgement. Only a finished worker is joined: an OS call
/// or caller wake callback can block synchronously, so an exceptional unfinished worker is
/// detached and retains its resources until it returns (possibly until process exit).
/// Cache reads never block the UI; callers should keep wake callbacks nonblocking.
pub struct Watcher {
    shared: Arc<Shared>,
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    stop: async_channel::Sender<()>,
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    worker: Option<std::thread::JoinHandle<()>>,
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    finished: std::sync::mpsc::Receiver<()>,
}
impl Watcher {
    /// Start an active subscription, or return `None` on unsupported targets or worker failure.
    /// `wake` runs on the worker and must not block.
    pub fn start(wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
        Self::start_active(true, wake)
    }
    /// Start observing with explicit control over wake notifications.
    /// An inactive subscription still updates its cache. Unsupported targets and worker
    /// startup failure return `None`; browser targets never start a desktop watcher.
    pub fn start_active(active: bool, wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
        #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
        {
            Self::spawn(active, wake, portal::watch)
        }
        #[cfg(not(all(unix, not(target_os = "macos"), not(target_arch = "wasm32"))))]
        {
            let _ = (active, wake);
            None
        }
    }
    /// Read the cached preference without blocking; `None` means no known preference.
    pub fn current(&self) -> Option<Theme> {
        self.shared.get()
    }
    /// GNOME's `org.gnome.desktop.wm.preferences` `button-layout` as the desktop reports it, such
    /// as `appmenu:minimize,close`; an empty string is an answer too. `None` until the portal or
    /// the `gsettings` fallback has answered, and on targets without a desktop watcher.
    ///
    /// It comes over the same connection as the theme, and a change runs the same wake callback.
    /// Only a GNOME-family window manager follows this setting; `craft_window::policy::LayoutWatch`
    /// decides when it applies. [`Watcher::wait_first`] does not wait for it.
    pub fn button_layout(&self) -> Option<String> {
        self.shared.layout.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
    /// GNOME's `org.gnome.desktop.interface` `icon-theme` name as the desktop reports it, such as
    /// `Adwaita`. `None` until the portal or the `gsettings` fallback has answered, on targets
    /// without a desktop watcher, and always off GNOME-like desktops (`XDG_CURRENT_DESKTOP` names
    /// GNOME), where the key holds the schema's default rather than the user's choice. Other
    /// desktops keep the name in their own files; read those with
    /// `craft_window::Desktop::icon_theme`.
    ///
    /// It comes over the same connection as the theme, a change runs the same wake callback, and
    /// [`Watcher::wait_first`] does not wait for it. A name that is empty or has no theme folder is
    /// the caller's to ignore.
    pub fn icon_theme(&self) -> Option<String> {
        self.shared.icons.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
    /// The desktop's contrast preference, from the portal's `org.freedesktop.appearance`
    /// `contrast`. `None` until the portal answers, when it has no such key, and on targets
    /// without a desktop watcher.
    ///
    /// It comes over the same connection as the theme, a change runs the same wake callback, and
    /// [`Watcher::wait_first`] returns after the first read of it. macOS and Windows have no
    /// watcher: reading their settings needs platform bindings, and this crate forbids `unsafe`.
    /// On the web use [`browser_contrast`].
    pub fn contrast(&self) -> Option<Contrast> {
        match self.shared.contrast.load(Ordering::Acquire) {
            1 => Some(Contrast::Standard),
            2 => Some(Contrast::High),
            _ => None,
        }
    }
    /// Whether the desktop asks for reduced motion, from the portal's `org.freedesktop.appearance`
    /// `reduced-motion`, or GNOME's `org.gnome.desktop.interface` `enable-animations` (inverted)
    /// when the portal has no `reduced-motion` key. `None` until one answers, when neither key
    /// exists, and on targets without a desktop watcher.
    ///
    /// Wake and [`Watcher::wait_first`] behave as for [`Watcher::contrast`]. On the web use
    /// [`browser_reduced_motion`].
    pub fn reduced_motion(&self) -> Option<bool> {
        match self.shared.motion.load(Ordering::Acquire) {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }
    /// Block until the first answer is known, then return it; at most `timeout`.
    ///
    /// Returns early, with `current()`, when the portal is absent and the `gsettings` fallback
    /// found nothing, or when the desktop reports no preference. Call it once while building the
    /// first frame so a light desktop does not flash dark:
    ///
    /// ```ignore
    /// let theme = watcher.wait_first(std::time::Duration::from_millis(250));
    /// ```
    pub fn wait_first(&self, timeout: Duration) -> Option<Theme> {
        let settled = self.shared.settled.lock().unwrap_or_else(PoisonError::into_inner);
        drop(self.shared.settled_changed.wait_timeout_while(settled, timeout, |settled| !*settled).unwrap_or_else(PoisonError::into_inner));
        self.current()
    }
    /// Enable or suppress future change notifications without stopping observation.
    /// Changing this flag does not itself wake the host or clear the cached preference.
    pub fn set_active(&self, active: bool) {
        self.shared.active.store(active, Ordering::Release);
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn spawn<F: std::future::Future<Output = ()> + 'static>(
        active: bool,
        wake: impl Fn() + Send + Sync + 'static,
        source: impl FnOnce(Arc<Shared>) -> F + Send + 'static,
    ) -> Option<Self> {
        let gnome = is_gnome(std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref());
        let shared = Arc::new(Shared {
            value: AtomicU8::new(0),
            contrast: AtomicU8::new(0),
            motion: AtomicU8::new(0),
            active: AtomicBool::new(active),
            settled: Mutex::new(false),
            settled_changed: Condvar::new(),
            layout: Mutex::new(None),
            icons: Mutex::new(None),
            gnome,
            wake: Box::new(wake),
        });
        let worker_shared = Arc::clone(&shared);
        let (stop, stopped) = async_channel::bounded::<()>(1);
        let (done, finished) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("desktop-appearance".into())
            .spawn(move || {
                futures_lite::future::block_on(futures_lite::future::race(
                    async {
                        let _ = stopped.recv().await;
                    },
                    source(Arc::clone(&worker_shared)),
                ));
                worker_shared.settle();
                let _ = done.send(());
            })
            .ok()?;
        Some(Self { shared, stop, worker: Some(worker), finished })
    }
}
impl Drop for Watcher {
    fn drop(&mut self) {
        self.set_active(false);
        #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
        {
            self.shared.settle();
            self.stop.close();
            if let Some(worker) = self.worker.take() {
                let _ = self.finished.recv_timeout(std::time::Duration::from_millis(100));
                // Acknowledgement precedes thread-local destructors. They can block too, so
                // never join solely on the acknowledgement (or channel disconnection).
                if worker.is_finished() {
                    let _ = worker.join();
                }
            }
        }
    }
}

/// The browser's `prefers-contrast: more` media query: `None` outside a browser window or when
/// the browser doesn't know the feature. The query runs on each call; there is no change
/// notification, so read it when building a frame.
#[cfg(target_arch = "wasm32")]
pub fn browser_contrast() -> Option<Contrast> {
    media_matches("(prefers-contrast: more)").map(|more| if more { Contrast::High } else { Contrast::Standard })
}
/// The browser's `prefers-reduced-motion: reduce` media query, read like [`browser_contrast`].
#[cfg(target_arch = "wasm32")]
pub fn browser_reduced_motion() -> Option<bool> {
    media_matches("(prefers-reduced-motion: reduce)")
}
#[cfg(target_arch = "wasm32")]
fn media_matches(query: &str) -> Option<bool> {
    let list = web_sys::window()?.match_media(query).ok()??;
    // A browser that doesn't know the feature serializes the query as `not all`.
    (list.media() != "not all").then(|| list.matches())
}

/// Whether a colon-separated `XDG_CURRENT_DESKTOP` value names GNOME.
#[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
fn is_gnome(desktop: Option<&str>) -> bool {
    desktop.is_some_and(|list| list.split(':').any(|name| name.eq_ignore_ascii_case("gnome")))
}

#[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
mod portal {
    use super::{Contrast, Shared, Theme};
    use futures_lite::{StreamExt, future::zip};
    use std::{sync::Arc, time::Duration};
    use zbus::zvariant::{OwnedValue, Value};
    const DEST: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const IFACE: &str = "org.freedesktop.portal.Settings";
    const STANDARD: &str = "org.freedesktop.appearance";
    const GNOME: &str = "org.gnome.desktop.interface";
    const KEY: &str = "color-scheme";
    const WM: &str = "org.gnome.desktop.wm.preferences";
    const LAYOUT_KEY: &str = "button-layout";
    const ICONS_KEY: &str = "icon-theme";
    const CONTRAST_KEY: &str = "contrast";
    const MOTION_KEY: &str = "reduced-motion";
    const ANIMATIONS_KEY: &str = "enable-animations";
    const TIMEOUT: Duration = Duration::from_secs(2);

    // Outer None means unknown/malformed; Some(None) is an explicit no-preference answer.
    // GNOME 42-46 writes 'default' (portal value 0) for Settings > Appearance > Light, so on GNOME
    // both mean light; elsewhere both are no preference.
    fn decode(namespace: &str, value: &Value<'_>, gnome: bool) -> Option<Option<Theme>> {
        match (namespace, value) {
            (STANDARD, Value::U32(n)) => Some(match n {
                1 => Some(Theme::Dark),
                2 => Some(Theme::Light),
                0 if gnome => Some(Theme::Light),
                _ => None,
            }),
            (GNOME, Value::Str(s)) => Some(match s.as_str() {
                "prefer-dark" => Some(Theme::Dark),
                "prefer-light" => Some(Theme::Light),
                "default" if gnome => Some(Theme::Light),
                _ => None,
            }),
            _ => None,
        }
    }
    // The legacy `Read` wraps the value in one more variant; anything else is malformed.
    fn unwrap_legacy<'a>(value: &'a Value<'a>, legacy: bool) -> Option<&'a Value<'a>> {
        match (legacy, value) {
            (false, value) => Some(value),
            (true, Value::Value(inner)) => Some(inner),
            (true, _) => None,
        }
    }
    fn reply(namespace: &str, value: &Value<'_>, legacy: bool, gnome: bool) -> Option<Option<Theme>> {
        decode(namespace, unwrap_legacy(value, legacy)?, gnome)
    }
    // `None` means malformed. The portal spec treats unknown values as 0, no preference.
    fn decode_contrast(namespace: &str, value: &Value<'_>) -> Option<Contrast> {
        match (namespace, value) {
            (STANDARD, Value::U32(n)) => Some(if *n == 1 { Contrast::High } else { Contrast::Standard }),
            _ => None,
        }
    }
    fn contrast_reply(value: &Value<'_>, legacy: bool) -> Option<Contrast> {
        decode_contrast(STANDARD, unwrap_legacy(value, legacy)?)
    }
    // Whether motion is reduced: the portal's `reduced-motion` (1 reduced; other values are no
    // preference) or GNOME's `enable-animations` boolean, inverted. `None` means malformed.
    fn decode_motion(namespace: &str, value: &Value<'_>) -> Option<bool> {
        match (namespace, value) {
            (STANDARD, Value::U32(n)) => Some(*n == 1),
            (GNOME, Value::Bool(enabled)) => Some(!enabled),
            _ => None,
        }
    }
    fn motion_reply(namespace: &str, value: &Value<'_>, legacy: bool) -> Option<bool> {
        decode_motion(namespace, unwrap_legacy(value, legacy)?)
    }
    // `None` means malformed. Any string is an answer, the empty one included: what the buttons
    // mean, and what an unusable layout falls back to, is craft-window's call.
    fn decode_layout(value: &Value<'_>) -> Option<String> {
        let Value::Str(s) = value else { return None };
        Some(s.as_str().trim().to_string())
    }
    fn layout_reply(value: &Value<'_>, legacy: bool) -> Option<String> {
        decode_layout(unwrap_legacy(value, legacy)?)
    }
    // `None` means malformed. A blank name is no answer.
    fn decode_icons(value: &Value<'_>) -> Option<String> {
        let Value::Str(s) = value else { return None };
        Some(s.as_str().trim().to_string()).filter(|name| !name.is_empty())
    }
    fn icons_reply(value: &Value<'_>, legacy: bool) -> Option<String> {
        decode_icons(unwrap_legacy(value, legacy)?)
    }
    fn combined(standard: Option<Option<Theme>>, gnome: Option<Option<Theme>>) -> Option<Option<Theme>> {
        if standard.is_none() && gnome.is_none() { None } else { Some(standard.flatten().or(gnome.flatten())) }
    }
    async fn bounded<T>(future: impl std::future::Future<Output = T>) -> Option<T> {
        futures_lite::future::race(async { Some(future.await) }, async {
            async_io::Timer::after(TIMEOUT).await;
            None
        })
        .await
    }
    async fn read_with<T>(connection: &zbus::Connection, namespace: &str, key: &str, answer: impl Fn(&Value<'_>, bool) -> Option<T>) -> Option<T> {
        for (method, legacy) in [("ReadOne", false), ("Read", true)] {
            if let Some(Ok(message)) = bounded(connection.call_method(Some(DEST), PATH, Some(IFACE), method, &(namespace, key))).await
                && let Ok(value) = message.body().deserialize::<OwnedValue>()
                && let Some(answer) = answer(&value, legacy)
            {
                return Some(answer);
            }
        }
        None
    }
    async fn read(connection: &zbus::Connection, namespace: &str, gnome: bool) -> Option<Option<Theme>> {
        read_with(connection, namespace, KEY, |value, legacy| reply(namespace, value, legacy, gnome)).await
    }
    fn rule(key: &'static str) -> zbus::Result<zbus::MatchRule<'static>> {
        Ok(zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(DEST)?
            .path(PATH)?
            .interface(IFACE)?
            .member("SettingChanged")?
            .arg(1, key)?
            .build())
    }
    fn owner_rule() -> zbus::Result<zbus::MatchRule<'static>> {
        Ok(zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.freedesktop.DBus")?
            .interface("org.freedesktop.DBus")?
            .member("NameOwnerChanged")?
            .arg(0, DEST)?
            .build())
    }
    /// The one-shot `gsettings` reads used when the portal can't answer, each at most once per
    /// watcher, and the `PATH` they search.
    pub(super) struct Fallbacks {
        theme_used: bool,
        layout_used: bool,
        icons_used: bool,
        path: Option<std::ffi::OsString>,
    }
    impl Fallbacks {
        fn from_env() -> Self {
            Self { theme_used: false, layout_used: false, icons_used: false, path: std::env::var_os("PATH") }
        }
        #[cfg(test)]
        pub(super) fn used() -> Self {
            Self { theme_used: true, layout_used: true, icons_used: true, path: None }
        }
        async fn get(&self, schema: &str, key: &str) -> Option<String> {
            let exe = gsettings_exe(self.path.as_deref())?;
            let text = crate::process::capture(&exe, &["get", schema, key], TIMEOUT).await?;
            Some(text.trim().trim_matches('\'').to_string())
        }
        async fn theme(&mut self, shared: &Shared) {
            if std::mem::replace(&mut self.theme_used, true) {
                return;
            }
            if let Some(text) = self.get(GNOME, KEY).await
                && let Some(answer) = decode(GNOME, &Value::from(text.as_str()), shared.gnome)
            {
                shared.publish(answer);
            }
        }
        async fn icons(&mut self, shared: &Shared) {
            if std::mem::replace(&mut self.icons_used, true) {
                return;
            }
            if let Some(text) = self.get(GNOME, ICONS_KEY).await
                && let Some(answer) = decode_icons(&Value::from(text.as_str()))
            {
                shared.publish_icons(answer);
            }
        }
        async fn layout(&mut self, shared: &Shared) {
            if std::mem::replace(&mut self.layout_used, true) {
                return;
            }
            if let Some(text) = self.get(WM, LAYOUT_KEY).await
                && let Some(answer) = decode_layout(&Value::from(text.as_str()))
            {
                shared.publish_layout(answer);
            }
        }
    }
    pub(super) async fn watch(shared: Arc<Shared>) {
        let mut retry = Duration::from_secs(5);
        let mut fallbacks = Fallbacks::from_env();
        loop {
            let mut connected = false;
            let _ = session(&shared, &mut fallbacks, &mut connected).await;
            // No session bus, no portal and an empty fallback all end here without a publish.
            shared.settle();
            if connected {
                retry = Duration::from_secs(5);
            }
            // Keep the last valid answer during disconnect; a new explicit no-preference clears it.
            async_io::Timer::after(retry).await;
            retry = (retry * 2).min(Duration::from_secs(300));
        }
    }
    // Keep Nix/profile PATH entries, but never execute a helper from a relative search folder.
    fn gsettings_exe(path: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf> {
        let dirs = path.map(std::env::split_paths).into_iter().flatten().filter(|p| p.is_absolute());
        dirs.chain(["/usr/bin", "/bin", "/usr/local/bin"].map(std::path::PathBuf::from)).map(|p| p.join("gsettings")).find(|p| p.is_file())
    }
    async fn session(shared: &Shared, fallbacks: &mut Fallbacks, connected: &mut bool) -> Option<()> {
        let connection = match zbus::connection::Builder::session() {
            Ok(builder) => bounded(builder.method_timeout(TIMEOUT).build()).await.and_then(Result::ok),
            Err(_) => None,
        };
        let Some(connection) = connection else {
            fallbacks.theme(shared).await;
            fallbacks.layout(shared).await;
            fallbacks.icons(shared).await;
            return None;
        };
        watch_connection(shared, fallbacks, connected, connection).await
    }
    enum Event {
        Setting(zbus::Result<zbus::Message>),
        Owner(zbus::Result<zbus::Message>),
    }
    /// The latest valid answer from each source of a setting that combines two namespaces.
    struct Answers {
        standard: Option<Option<Theme>>,
        gnome: Option<Option<Theme>>,
        motion: Option<bool>,
        gnome_motion: Option<bool>,
    }
    impl Answers {
        fn publish_motion(&self, shared: &Shared) {
            if let Some(reduced) = self.motion.or(self.gnome_motion) {
                shared.publish_motion(reduced);
            }
        }
        /// Apply one `SettingChanged`; malformed values and other settings change nothing.
        fn apply(&mut self, shared: &Shared, namespace: &str, key: &str, value: &Value<'_>) {
            match (namespace, key) {
                (WM, LAYOUT_KEY) => {
                    if let Some(layout) = decode_layout(value) {
                        shared.publish_layout(layout);
                    }
                }
                (GNOME, ICONS_KEY) => {
                    if let Some(icons) = decode_icons(value) {
                        shared.publish_icons(icons);
                    }
                }
                (STANDARD, CONTRAST_KEY) => {
                    if let Some(contrast) = decode_contrast(namespace, value) {
                        shared.publish_contrast(contrast);
                    }
                }
                (STANDARD, MOTION_KEY) | (GNOME, ANIMATIONS_KEY) => {
                    let Some(reduced) = decode_motion(namespace, value) else { return };
                    if namespace == STANDARD {
                        self.motion = Some(reduced);
                    } else {
                        self.gnome_motion = Some(reduced);
                    }
                    self.publish_motion(shared);
                }
                (STANDARD | GNOME, KEY) => {
                    let Some(answer) = decode(namespace, value, shared.gnome) else { return };
                    if namespace == STANDARD {
                        self.standard = Some(answer);
                    } else {
                        self.gnome = Some(answer);
                    }
                    if let Some(answer) = combined(self.standard, self.gnome) {
                        shared.publish(answer);
                    }
                }
                _ => {}
            }
        }
    }
    async fn subscribe(connection: &zbus::Connection, rule: zbus::Result<zbus::MatchRule<'static>>, capacity: usize) -> Option<zbus::MessageStream> {
        bounded(zbus::MessageStream::for_match_rule(rule.ok()?, connection, Some(capacity))).await?.ok()
    }
    async fn next_event(settings: &mut [zbus::MessageStream], owners: &mut zbus::MessageStream) -> Option<Event> {
        let setting = futures_lite::future::poll_fn(|cx| {
            for stream in settings.iter_mut() {
                if let std::task::Poll::Ready(message) = stream.poll_next(cx) {
                    return std::task::Poll::Ready(message.map(Event::Setting));
                }
            }
            std::task::Poll::Pending
        });
        futures_lite::future::race(setting, async { owners.next().await.map(Event::Owner) }).await
    }
    async fn watch_connection(shared: &Shared, fallbacks: &mut Fallbacks, connected: &mut bool, connection: zbus::Connection) -> Option<()> {
        // MatchRule installs before activation: unlike a proxy subscription it needs no current owner.
        let mut settings = [
            subscribe(&connection, rule(KEY), 64).await?,
            subscribe(&connection, rule(LAYOUT_KEY), 8).await?,
            subscribe(&connection, rule(ICONS_KEY), 8).await?,
            subscribe(&connection, rule(CONTRAST_KEY), 8).await?,
            subscribe(&connection, rule(MOTION_KEY), 8).await?,
            subscribe(&connection, rule(ANIMATIONS_KEY), 8).await?,
        ];
        let mut owners = subscribe(&connection, owner_rule(), 8).await?;
        // Concurrent reads, so contrast and motion are known by the time the theme settles
        // `wait_first` without adding round trips before it.
        let ((standard, gnome), (contrast, (motion, gnome_motion))) = zip(
            zip(read(&connection, STANDARD, shared.gnome), read(&connection, GNOME, shared.gnome)),
            zip(
                read_with(&connection, STANDARD, CONTRAST_KEY, contrast_reply),
                zip(
                    read_with(&connection, STANDARD, MOTION_KEY, |value, legacy| motion_reply(STANDARD, value, legacy)),
                    read_with(&connection, GNOME, ANIMATIONS_KEY, |value, legacy| motion_reply(GNOME, value, legacy)),
                ),
            ),
        )
        .await;
        let mut answers = Answers { standard, gnome, motion, gnome_motion };
        if let Some(contrast) = contrast {
            shared.publish_contrast(contrast);
        }
        answers.publish_motion(shared);
        match combined(standard, gnome) {
            Some(answer) => {
                *connected = true;
                shared.publish(answer);
            }
            None => {
                fallbacks.theme(shared).await;
                fallbacks.layout(shared).await;
                fallbacks.icons(shared).await;
                return None;
            }
        }
        // Read after the theme is published so `wait_first` never waits for the layout.
        match read_with(&connection, WM, LAYOUT_KEY, layout_reply).await {
            Some(layout) => shared.publish_layout(layout),
            None => fallbacks.layout(shared).await,
        }
        match read_with(&connection, GNOME, ICONS_KEY, icons_reply).await {
            Some(icons) => shared.publish_icons(icons),
            None => fallbacks.icons(shared).await,
        }
        loop {
            let message = match next_event(&mut settings, &mut owners).await? {
                Event::Owner(message) => {
                    // Activation is already covered by the initial read. Loss/replacement reconnects.
                    if let Ok((_, old, _)) = message.ok()?.body().deserialize::<(String, String, String)>()
                        && !old.is_empty()
                    {
                        return None;
                    }
                    continue;
                }
                Event::Setting(message) => message.ok()?,
            };
            if let Ok((namespace, key, value)) = message.body().deserialize::<(String, String, OwnedValue)>() {
                answers.apply(shared, &namespace, &key, &value);
            }
        }
    }
    #[cfg(test)]
    #[path = "appearance_bus_tests.rs"]
    mod bus_tests;
    #[cfg(test)]
    mod tests {
        #[test]
        fn gsettings_respects_absolute_nix_style_path_without_using_relative_folders() -> Result<(), Box<dyn std::error::Error>> {
            struct Fixture(std::path::PathBuf);
            impl Drop for Fixture {
                fn drop(&mut self) {
                    let _ = std::fs::remove_dir_all(&self.0);
                }
            }
            let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
            let directory = std::env::temp_dir().join(format!("craft-platform-gsettings-{}-{nonce}", std::process::id()));
            std::fs::create_dir(&directory)?;
            let fixture = Fixture(directory);
            let helper = fixture.0.join("gsettings");
            std::fs::write(&helper, b"fixture")?;
            let path = std::env::join_paths([std::path::Path::new("relative-untrusted"), fixture.0.as_path()])?;
            assert_eq!(super::gsettings_exe(Some(path.as_os_str())), Some(helper));
            assert!(super::gsettings_exe(Some(std::ffi::OsStr::new("relative-untrusted"))).is_none_or(|p| p.is_absolute()));
            Ok(())
        }

        #[test]
        fn button_layout_decodes_strings_verbatim_and_ignores_malformed_values() {
            use super::{decode_layout, layout_reply};
            assert_eq!(decode_layout(&Value::from("appmenu:minimize,close")).as_deref(), Some("appmenu:minimize,close"));
            assert_eq!(decode_layout(&Value::from(" close: ")).as_deref(), Some("close:"));
            // Empty is the desktop's answer; craft-window maps it to the desktop's default.
            assert_eq!(decode_layout(&Value::from("")).as_deref(), Some(""));
            assert_eq!(decode_layout(&Value::U32(1)), None);
            assert_eq!(decode_layout(&Value::Bool(true)), None);
            let wrapped = Value::Value(Box::new(Value::from("close,minimize:")));
            assert_eq!(layout_reply(&wrapped, true).as_deref(), Some("close,minimize:"));
            assert_eq!(layout_reply(&wrapped, false), None);
            assert_eq!(layout_reply(&Value::from("close:"), true), None);
            assert_eq!(layout_reply(&Value::from("close:"), false).as_deref(), Some("close:"));
        }
        #[test]
        fn the_gnome_icon_key_counts_only_on_gnome_desktops() {
            let shared = |gnome| crate::appearance::Shared {
                value: std::sync::atomic::AtomicU8::new(0),
                contrast: std::sync::atomic::AtomicU8::new(0),
                motion: std::sync::atomic::AtomicU8::new(0),
                active: std::sync::atomic::AtomicBool::new(false),
                settled: std::sync::Mutex::new(false),
                settled_changed: std::sync::Condvar::new(),
                layout: std::sync::Mutex::new(None),
                icons: std::sync::Mutex::new(None),
                gnome,
                wake: Box::new(|| {}),
            };
            let (kde, gnome) = (shared(false), shared(true));
            kde.publish_icons("Adwaita".into());
            gnome.publish_icons("Papirus".into());
            assert_eq!(*kde.icons.lock().unwrap(), None);
            assert_eq!(gnome.icons.lock().unwrap().as_deref(), Some("Papirus"));
        }
        #[test]
        fn icon_theme_decodes_names_and_ignores_blank_or_malformed_values() {
            use super::{decode_icons, icons_reply};
            assert_eq!(decode_icons(&Value::from("Papirus-Dark")).as_deref(), Some("Papirus-Dark"));
            assert_eq!(decode_icons(&Value::from(" Adwaita ")).as_deref(), Some("Adwaita"));
            assert_eq!(decode_icons(&Value::from("")), None);
            assert_eq!(decode_icons(&Value::U32(1)), None);
            let wrapped = Value::Value(Box::new(Value::from("breeze")));
            assert_eq!(icons_reply(&wrapped, true).as_deref(), Some("breeze"));
            assert_eq!(icons_reply(&wrapped, false), None);
            assert_eq!(icons_reply(&Value::from("breeze"), true), None);
            assert_eq!(icons_reply(&Value::from("breeze"), false).as_deref(), Some("breeze"));
        }
        #[test]
        fn without_a_portal_gsettings_supplies_theme_and_button_layout() -> Result<(), Box<dyn std::error::Error>> {
            use std::os::unix::fs::PermissionsExt;
            struct Fixture(std::path::PathBuf);
            impl Drop for Fixture {
                fn drop(&mut self) {
                    let _ = std::fs::remove_dir_all(&self.0);
                }
            }
            let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
            let directory = std::env::temp_dir().join(format!("craft-platform-layout-fallback-{}-{nonce}", std::process::id()));
            std::fs::create_dir(&directory)?;
            let fixture = Fixture(directory);
            let helper = fixture.0.join("gsettings");
            std::fs::write(
                &helper,
                "#!/bin/sh\ncase \"$2 $3\" in\n\
                 'org.gnome.desktop.wm.preferences button-layout') echo \"'close,minimize:appmenu'\" ;;\n\
                 'org.gnome.desktop.interface color-scheme') echo \"'prefer-dark'\" ;;\n\
                 *) exit 1 ;;\nesac\n",
            )?;
            std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755))?;
            let path = fixture.0.clone().into_os_string();
            let watcher = Watcher::spawn(
                false,
                || {},
                move |shared| async move {
                    let mut fallbacks = super::Fallbacks { theme_used: false, layout_used: false, icons_used: false, path: Some(path) };
                    fallbacks.theme(&shared).await;
                    fallbacks.layout(&shared).await;
                },
            )
            .ok_or("worker creation failed")?;
            assert_eq!(watcher.wait_first(Duration::from_secs(2)), Some(Theme::Dark));
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            while watcher.button_layout().is_none() {
                assert!(std::time::Instant::now() < deadline, "gsettings layout was not published");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(watcher.button_layout().as_deref(), Some("close,minimize:appmenu"));
            Ok(())
        }

        use super::{GNOME, STANDARD, combined, decode, reply};
        use crate::appearance::is_gnome;
        use crate::appearance::{Contrast, Theme, Watcher};
        use std::{
            sync::{Arc, atomic::Ordering},
            time::Duration,
        };
        use zbus::zvariant::Value;
        #[test]
        fn no_preference_is_an_answer_and_gnome_only_fills_missing_preference() {
            assert_eq!(combined(None, None), None);
            assert_eq!(combined(Some(None), None), Some(None));
            assert_eq!(combined(Some(None), Some(Some(Theme::Dark))), Some(Some(Theme::Dark)));
            assert_eq!(combined(Some(Some(Theme::Light)), Some(Some(Theme::Dark))), Some(Some(Theme::Light)));
            assert_eq!(combined(None, Some(None)), Some(None));
        }
        #[test]
        fn malformed_is_distinct_from_no_preference_and_legacy_is_exact() {
            assert_eq!(decode(STANDARD, &Value::U32(0), false), Some(None));
            assert_eq!(decode(STANDARD, &Value::U32(77), false), Some(None));
            assert_eq!(decode(STANDARD, &Value::from("dark"), false), None);
            assert_eq!(reply(STANDARD, &Value::Value(Box::new(Value::U32(1))), true, false), Some(Some(Theme::Dark)));
            assert_eq!(reply(STANDARD, &Value::U32(1), true, false), None);
            assert_eq!(reply(STANDARD, &Value::Value(Box::new(Value::Value(Box::new(Value::U32(1))))), true, false), None);
            assert_eq!(decode(STANDARD, &Value::Value(Box::new(Value::U32(1))), false), None);
            assert_eq!(decode(STANDARD, &Value::I32(1), false), None);
            assert_eq!(decode(GNOME, &Value::U32(1), false), None);
            assert_eq!(reply(GNOME, &Value::from("prefer-light"), false, false), Some(Some(Theme::Light)));
        }
        #[test]
        fn gnome_light_setting_reads_as_light_and_zero_is_no_preference_elsewhere() {
            assert_eq!(decode(GNOME, &Value::from("default"), true), Some(Some(Theme::Light)));
            // A GNOME schema installed on another desktop doesn't make that desktop light.
            assert_eq!(decode(GNOME, &Value::from("default"), false), Some(None));
            assert_eq!(decode(GNOME, &Value::from("prefer-dark"), false), Some(Some(Theme::Dark)));
            assert_eq!(decode(GNOME, &Value::from("prefer-light"), false), Some(Some(Theme::Light)));
            assert_eq!(decode(STANDARD, &Value::U32(0), true), Some(Some(Theme::Light)));
            assert_eq!(decode(STANDARD, &Value::U32(0), false), Some(None));
            for gnome in [true, false] {
                assert_eq!(decode(STANDARD, &Value::U32(1), gnome), Some(Some(Theme::Dark)));
                assert_eq!(decode(STANDARD, &Value::U32(2), gnome), Some(Some(Theme::Light)));
            }
            assert!(is_gnome(Some("GNOME")));
            assert!(is_gnome(Some("ubuntu:GNOME")));
            assert!(is_gnome(Some("gnome-classic:gnome")));
            assert!(!is_gnome(Some("KDE")));
            assert!(!is_gnome(Some("X-Cinnamon")));
            assert!(!is_gnome(None));
        }
        fn spawn_idle(source: impl FnOnce(Arc<crate::appearance::Shared>) + Send + 'static) -> Result<Watcher, &'static str> {
            Watcher::spawn(false, || {}, move |shared| async move { source(shared) }).ok_or("worker creation failed")
        }
        #[test]
        fn contrast_and_motion_decode_portal_and_gnome_values_and_reject_malformed_ones() {
            use super::{contrast_reply, decode_contrast, decode_motion, motion_reply};
            assert_eq!(decode_contrast(STANDARD, &Value::U32(0)), Some(Contrast::Standard));
            assert_eq!(decode_contrast(STANDARD, &Value::U32(1)), Some(Contrast::High));
            // Unknown values are no preference, as the portal spec says.
            assert_eq!(decode_contrast(STANDARD, &Value::U32(7)), Some(Contrast::Standard));
            assert_eq!(decode_contrast(STANDARD, &Value::I32(1)), None);
            assert_eq!(decode_contrast(STANDARD, &Value::Bool(true)), None);
            assert_eq!(decode_contrast(STANDARD, &Value::from("high")), None);
            assert_eq!(decode_contrast(GNOME, &Value::U32(1)), None);
            assert_eq!(contrast_reply(&Value::Value(Box::new(Value::U32(1))), true), Some(Contrast::High));
            assert_eq!(contrast_reply(&Value::U32(1), true), None);
            assert_eq!(contrast_reply(&Value::Value(Box::new(Value::U32(1))), false), None);

            assert_eq!(decode_motion(STANDARD, &Value::U32(0)), Some(false));
            assert_eq!(decode_motion(STANDARD, &Value::U32(1)), Some(true));
            assert_eq!(decode_motion(STANDARD, &Value::U32(9)), Some(false));
            assert_eq!(decode_motion(STANDARD, &Value::Bool(true)), None);
            assert_eq!(decode_motion(GNOME, &Value::Bool(false)), Some(true));
            assert_eq!(decode_motion(GNOME, &Value::Bool(true)), Some(false));
            assert_eq!(decode_motion(GNOME, &Value::U32(0)), None);
            assert_eq!(decode_motion(GNOME, &Value::from("false")), None);
            assert_eq!(decode_motion("org.example.other", &Value::U32(1)), None);
            assert_eq!(motion_reply(GNOME, &Value::Value(Box::new(Value::Bool(false))), true), Some(true));
            assert_eq!(motion_reply(STANDARD, &Value::U32(1), true), None);
            assert_eq!(motion_reply(STANDARD, &Value::U32(1), false), Some(true));
        }
        #[test]
        fn contrast_and_motion_stay_unknown_until_published_and_wake_only_on_change() -> Result<(), Box<dyn std::error::Error>> {
            let watcher = spawn_idle(|shared| shared.publish(Some(Theme::Dark)))?;
            assert_eq!(watcher.wait_first(Duration::from_secs(2)), Some(Theme::Dark));
            assert_eq!(watcher.contrast(), None);
            assert_eq!(watcher.reduced_motion(), None);

            let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = Arc::clone(&wakes);
            let watcher = Watcher::spawn(
                true,
                move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                },
                |_| futures_lite::future::pending::<()>(),
            )
            .ok_or("worker creation failed")?;
            watcher.shared.publish_contrast(Contrast::High);
            watcher.shared.publish_contrast(Contrast::High);
            watcher.shared.publish_motion(true);
            watcher.shared.publish_motion(true);
            assert_eq!(watcher.contrast(), Some(Contrast::High));
            assert_eq!(watcher.reduced_motion(), Some(true));
            assert_eq!(wakes.load(Ordering::Relaxed), 2);
            watcher.shared.publish_contrast(Contrast::Standard);
            watcher.shared.publish_motion(false);
            assert_eq!(watcher.contrast(), Some(Contrast::Standard));
            assert_eq!(watcher.reduced_motion(), Some(false));
            assert_eq!(wakes.load(Ordering::Relaxed), 4);
            Ok(())
        }
        #[test]
        fn wait_first_returns_at_once_after_a_publish() -> Result<(), Box<dyn std::error::Error>> {
            let watcher = spawn_idle(|shared| {
                shared.publish(Some(Theme::Light));
                std::thread::sleep(Duration::from_millis(500));
            })?;
            let started = std::time::Instant::now();
            assert_eq!(watcher.wait_first(Duration::from_secs(2)), Some(Theme::Light));
            assert!(started.elapsed() < Duration::from_secs(1));
            Ok(())
        }
        #[test]
        fn wait_first_times_out_without_an_answer_and_drop_stays_bounded() -> Result<(), Box<dyn std::error::Error>> {
            let watcher = Watcher::spawn(false, || {}, |_| futures_lite::future::pending::<()>()).ok_or("worker creation failed")?;
            let started = std::time::Instant::now();
            assert_eq!(watcher.wait_first(Duration::from_millis(50)), None);
            assert!(started.elapsed() >= Duration::from_millis(50));
            drop(watcher);
            Ok(())
        }
        #[test]
        fn wait_first_returns_early_for_no_preference_and_for_a_source_that_ends_empty() -> Result<(), Box<dyn std::error::Error>> {
            let none = spawn_idle(|shared| shared.publish(None))?;
            let ended = spawn_idle(|_| {})?;
            for watcher in [none, ended] {
                let started = std::time::Instant::now();
                assert_eq!(watcher.wait_first(Duration::from_secs(2)), None);
                assert!(started.elapsed() < Duration::from_secs(1));
            }
            Ok(())
        }
        #[test]
        fn manual_cache_changes_do_not_wake_and_drop_cancels_pending_source() -> Result<(), Box<dyn std::error::Error>> {
            let (tx, rx) = std::sync::mpsc::channel();
            let (cancelled, cancellation) = std::sync::mpsc::channel();
            struct Cancelled(std::sync::mpsc::Sender<()>);
            impl Drop for Cancelled {
                fn drop(&mut self) {
                    let _ = self.0.send(());
                }
            }
            let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = Arc::clone(&wakes);
            let watcher = Watcher::spawn(
                false,
                move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                },
                move |shared| async move {
                    let _cancelled = Cancelled(cancelled);
                    shared.publish(Some(Theme::Dark));
                    let _ = tx.send(());
                    futures_lite::future::pending::<()>().await;
                },
            )
            .ok_or("worker creation failed")?;
            rx.recv_timeout(Duration::from_secs(2))?;
            assert_eq!(watcher.current(), Some(Theme::Dark));
            assert_eq!(wakes.load(Ordering::Relaxed), 0);
            watcher.set_active(true);
            watcher.shared.publish(Some(Theme::Dark));
            assert_eq!(wakes.load(Ordering::Relaxed), 0);
            watcher.shared.publish(Some(Theme::Light));
            assert_eq!(watcher.current(), Some(Theme::Light));
            watcher.shared.publish(Some(Theme::Dark));
            assert_eq!(watcher.current(), Some(Theme::Dark));
            watcher.shared.publish(None);
            assert_eq!(watcher.current(), None);
            assert_eq!(wakes.load(Ordering::Relaxed), 3);
            drop(watcher);
            cancellation.recv_timeout(Duration::from_secs(2))?;
            Ok(())
        }
        #[test]
        fn drop_does_not_wait_for_a_synchronously_blocked_source() -> Result<(), Box<dyn std::error::Error>> {
            let (started, ready) = std::sync::mpsc::channel();
            let (release, blocked) = std::sync::mpsc::channel();
            let (exited, exit) = std::sync::mpsc::channel();
            struct Release(Option<std::sync::mpsc::Sender<()>>);
            impl Drop for Release {
                fn drop(&mut self) {
                    if let Some(release) = self.0.take() {
                        let _ = release.send(());
                    }
                }
            }
            // Declared before the watcher: every error/unwind releases the synchronous poll.
            let release = Release(Some(release));
            let watcher = Watcher::spawn(
                false,
                || {},
                move |_| async move {
                    let _ = started.send(());
                    let _ = blocked.recv();
                    let _ = exited.send(());
                },
            )
            .ok_or("worker creation failed")?;
            ready.recv_timeout(Duration::from_secs(2))?;
            let (dropped, drop_result) = std::sync::mpsc::channel();
            let dropper = std::thread::spawn(move || {
                drop(watcher);
                let _ = dropped.send(());
            });
            let bounded = drop_result.recv_timeout(Duration::from_secs(1)).is_ok();
            // Release before asserting, including when an unbounded implementation regresses.
            drop(release);
            exit.recv_timeout(Duration::from_secs(2))?;
            assert!(dropper.join().is_ok(), "watcher drop panicked");
            assert!(bounded, "watcher drop waited for a blocked synchronous poll");
            Ok(())
        }
    }
}
/// Owned callback that supplies an observed theme and updates egui native-window hints.
#[cfg(feature = "egui")]
pub type SystemThemeFn = Box<dyn Fn(&egui::Context) -> Option<egui::Theme>>;
/// Create an egui theme callback over a [`SystemReader`] that wakes the first context it is
/// called with. Return `None` when no desktop watcher can start; the host retains its fallback
/// policy. It answers the colour scheme only: a host that also needs the button layout or icon
/// theme uses [`egui_system_service`], and one that follows contrast or reduced motion uses
/// [`SystemReader`] directly.
#[cfg(feature = "egui")]
pub fn egui_service() -> Option<SystemThemeFn> {
    egui_system_service().map(EguiService::into_theme_fn)
}

/// The desktop watcher an egui app installs once: the colour scheme for the theme callback,
/// plus the button layout and icon theme the same watcher hears. Clones share the watcher; every
/// accessor is non-blocking and callable from any thread (`SystemReader` is `Send`, checked at
/// compile time below). A change in any value repaints the
/// context [`EguiService::theme`] saw first.
#[cfg(feature = "egui")]
#[derive(Clone)]
pub struct EguiService {
    reader: std::sync::Arc<Mutex<SystemReader>>,
}

#[cfg(feature = "egui")]
impl EguiService {
    fn with<T>(&self, f: impl FnOnce(&SystemReader) -> T) -> T {
        f(&self.reader.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The observed colour scheme, as [`SystemThemeFn`] returns it. Call it once per frame: it
    /// registers `ctx` for wake-ups and hands the native window the observed scheme.
    pub fn theme(&self, ctx: &egui::Context) -> Option<egui::Theme> {
        self.with(|reader| reader.observed_theme(ctx))
    }

    /// GNOME's `button-layout`; see [`Watcher::button_layout`].
    pub fn button_layout(&self) -> Option<String> {
        self.with(SystemReader::button_layout)
    }

    /// GNOME's `icon-theme` name; see [`Watcher::icon_theme`].
    pub fn icon_theme(&self) -> Option<String> {
        self.with(SystemReader::icon_theme)
    }

    /// The colour scheme as an owned [`SystemThemeFn`]; the other accessors stay on clones.
    pub fn into_theme_fn(self) -> SystemThemeFn {
        Box::new(move |ctx: &egui::Context| self.theme(ctx))
    }
}

#[cfg(feature = "egui")]
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<EguiService>();
};

/// Start the desktop watcher behind an [`EguiService`], or return `None` when no watcher can
/// start; the host retains its fallback policy.
#[cfg(feature = "egui")]
pub fn egui_system_service() -> Option<EguiService> {
    let reader = SystemReader::for_egui();
    reader.watching().then(|| EguiService { reader: std::sync::Arc::new(Mutex::new(reader)) })
}

/// Supply the portal answer for native window decorations without changing egui's theme choice
/// or either palette. A missing portal answer falls back to the current toolkit system theme;
/// when both are absent the native pin is released. Linux/BSD window backends cannot reliably
/// resolve `SystemDefault` themselves.
/// The output hook runs after egui's own window-theme synchronization, so it cannot be overwritten
/// by a later automatic `SetTheme(SystemDefault)` in the same pass.
#[cfg(feature = "egui")]
pub fn sync_native_window_theme(ctx: &egui::Context, observed: Option<Theme>) {
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    {
        ctx.add_plugin(NativeWindowTheme::default());
        ctx.with_plugin(|plugin: &mut NativeWindowTheme| plugin.observed = observed);
    }
    #[cfg(not(all(unix, not(target_os = "macos"), not(target_arch = "wasm32"))))]
    let _ = (ctx, observed);
}

#[cfg(all(feature = "egui", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
#[derive(Default)]
struct NativeWindowTheme {
    observed: Option<Theme>,
    sent: std::collections::HashMap<egui::ViewportId, egui::SystemTheme>,
}
#[cfg(all(feature = "egui", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
impl egui::Plugin for NativeWindowTheme {
    fn debug_name(&self) -> &'static str {
        "desktop window appearance"
    }
    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        if !ctx.options(|o| o.sync_window_theme) {
            // Another owner may alter the native theme while synchronization is disabled.
            self.sent.clear();
            return;
        }
        let desired = match ctx.options(|o| o.theme_preference) {
            egui::ThemePreference::Dark => egui::SystemTheme::Dark,
            egui::ThemePreference::Light => egui::SystemTheme::Light,
            egui::ThemePreference::System => match self.observed.or_else(|| {
                ctx.system_theme().map(|theme| match theme {
                    egui::Theme::Dark => Theme::Dark,
                    egui::Theme::Light => Theme::Light,
                })
            }) {
                Some(Theme::Dark) => egui::SystemTheme::Dark,
                Some(Theme::Light) => egui::SystemTheme::Light,
                None => egui::SystemTheme::SystemDefault,
            },
        };
        // FullOutput can carry child-window commands too. Rewrite every viewport so a deferred
        // or immediate child cannot reintroduce egui's inherited hint after its own UI pass.
        for (id, viewport) in &mut output.viewport_output {
            let had_sync = viewport.commands.iter().any(|command| matches!(command, egui::ViewportCommand::SetTheme(_)));
            viewport.commands.retain(|command| !matches!(command, egui::ViewportCommand::SetTheme(_)));
            if had_sync || self.sent.get(id) != Some(&desired) {
                viewport.commands.push(egui::ViewportCommand::SetTheme(desired));
                self.sent.insert(*id, desired);
            }
        }
        // Forget destroyed native windows: a reused viewport identifier must receive its hint.
        self.sent.retain(|id, _| output.viewport_output.contains_key(id));
    }
}

#[cfg(all(test, feature = "egui", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
mod native_window_tests {
    use super::{Theme, sync_native_window_theme};
    fn pass(ctx: &egui::Context, observed: Option<Theme>, preference: egui::ThemePreference) -> Vec<egui::SystemTheme> {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            sync_native_window_theme(ui.ctx(), observed);
            // Deliberately apply the host's choice AFTER the observer, as real palette adapters do.
            ui.ctx().set_theme(preference);
        });
        output.textures_delta.clear(); // Headless policy test; no renderer consumes uploads.
        output
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .filter_map(|command| match command {
                egui::ViewportCommand::SetTheme(theme) => Some(*theme),
                _ => None,
            })
            .collect()
    }
    #[test]
    fn actual_end_pass_maps_system_manual_and_reset_without_overwriting_palettes() {
        use egui::{SystemTheme as Native, ThemePreference as Choice};
        let ctx = egui::Context::default();
        let marker = egui::Color32::from_rgb(12, 34, 56);
        ctx.all_styles_mut(|s| s.visuals.panel_fill = marker);
        assert_eq!(pass(&ctx, Some(Theme::Light), Choice::System), vec![Native::Light]);
        assert_eq!(ctx.options(|o| o.theme_preference), Choice::System);
        assert!(pass(&ctx, Some(Theme::Light), Choice::System).is_empty());
        assert_eq!(pass(&ctx, Some(Theme::Dark), Choice::System), vec![Native::Dark]);
        // A portal change cannot override a concrete manual native preference.
        assert_eq!(pass(&ctx, Some(Theme::Light), Choice::Dark), vec![Native::Dark]);
        assert_eq!(pass(&ctx, Some(Theme::Dark), Choice::Light), vec![Native::Light]);
        assert_eq!(pass(&ctx, Some(Theme::Dark), Choice::System), vec![Native::Dark]);
        assert_eq!(pass(&ctx, None, Choice::System), vec![Native::SystemDefault]);
        assert!(pass(&ctx, None, Choice::System).is_empty());
        assert_eq!(ctx.style_of(egui::Theme::Dark).visuals.panel_fill, marker);
        assert_eq!(ctx.style_of(egui::Theme::Light).visuals.panel_fill, marker);
    }
    #[test]
    fn native_system_hint_uses_current_toolkit_fallback_without_retaining_stale_input() {
        let ctx = egui::Context::default();
        for (raw_theme, observed, expected) in [
            (Some(egui::Theme::Light), None, egui::SystemTheme::Light),
            (Some(egui::Theme::Dark), None, egui::SystemTheme::Dark),
            (Some(egui::Theme::Dark), Some(Theme::Light), egui::SystemTheme::Light),
            (None, None, egui::SystemTheme::SystemDefault),
        ] {
            let mut output = ctx.run_ui(egui::RawInput { system_theme: raw_theme, ..Default::default() }, |ui| {
                sync_native_window_theme(ui.ctx(), observed);
                ui.ctx().set_theme(egui::ThemePreference::System);
            });
            let hints: Vec<_> = output
                .viewport_output
                .values()
                .flat_map(|v| &v.commands)
                .filter_map(|command| match command {
                    egui::ViewportCommand::SetTheme(theme) => Some(*theme),
                    _ => None,
                })
                .collect();
            output.textures_delta.clear();
            assert_eq!(hints, vec![expected]);
        }
    }
    #[test]
    fn deferred_windows_receive_the_same_hint_and_reset() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(false);
        let child = egui::ViewportId::from_hash_of("appearance-child");
        for observed in [Some(Theme::Light), Some(Theme::Dark), None] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                sync_native_window_theme(ui.ctx(), observed);
                ui.ctx().set_theme(egui::ThemePreference::System);
                ui.ctx().show_viewport_deferred(child, egui::ViewportBuilder::default(), |_, _| {});
            });
            output.textures_delta.clear();
            let expected = match observed {
                Some(Theme::Light) => egui::SystemTheme::Light,
                Some(Theme::Dark) => egui::SystemTheme::Dark,
                None => egui::SystemTheme::SystemDefault,
            };
            for id in [egui::ViewportId::ROOT, child] {
                let commands: Vec<_> = output
                    .viewport_output
                    .get(&id)
                    .into_iter()
                    .flat_map(|v| &v.commands)
                    .filter_map(|c| match c {
                        egui::ViewportCommand::SetTheme(t) => Some(*t),
                        _ => None,
                    })
                    .collect();
                assert_eq!(commands, vec![expected]);
            }
        }
    }
    #[test]
    fn custom_palette_system_policy_and_explicit_sync_opt_out_are_preserved() {
        use egui::{SystemTheme as Native, ThemePreference as Choice};
        let ctx = egui::Context::default();
        // Photo/PDF/Sound keep System native policy even when their custom palette is manual.
        ctx.all_styles_mut(|s| s.visuals = egui::Visuals::dark());
        assert_eq!(pass(&ctx, Some(Theme::Light), Choice::System), vec![Native::Light]);
        assert!(ctx.style_of(egui::Theme::Light).visuals.dark_mode);
        ctx.options_mut(|o| o.sync_window_theme = false);
        assert!(pass(&ctx, Some(Theme::Dark), Choice::System).is_empty());
        ctx.options_mut(|o| o.sync_window_theme = true);
        assert_eq!(pass(&ctx, Some(Theme::Dark), Choice::System), vec![Native::Dark]);
    }
}
