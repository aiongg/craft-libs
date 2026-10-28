//! Private-bus regression based on PdfCraft's queued-update test (storytold/pdfcraft#881).
use super::{ANIMATIONS_KEY, CONTRAST_KEY, DEST, Fallbacks, GNOME, IFACE, KEY, LAYOUT_KEY, MOTION_KEY, PATH, STANDARD, WM, watch_connection};
use crate::appearance::{Contrast, Theme, Watcher};
use std::time::Duration;
use zbus::zvariant::{OwnedValue, Value};

struct Daemon(std::process::Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
/// A private session bus, and its address.
fn private_bus(tag: &str) -> Result<(Daemon, String), Box<dyn std::error::Error>> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let mut daemon = Daemon(
        Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .arg(format!("--address=unix:abstract={tag}-{}", std::process::id()))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut address = String::new();
    std::io::BufReader::new(daemon.0.stdout.take().ok_or("no daemon stdout")?).read_line(&mut address)?;
    Ok((daemon, address.trim().to_string()))
}

#[test]
#[ignore = "requires dbus-daemon; private session bus only"]
fn legacy_read_keeps_queued_signals_and_drop_cancels_idle_subscription() -> Result<(), Box<dyn std::error::Error>> {
    struct Settings {
        captured: std::sync::mpsc::Sender<()>,
        released: async_channel::Receiver<()>,
    }
    #[zbus::interface(name = "org.freedesktop.portal.Settings")]
    impl Settings {
        async fn read(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
            if (namespace, key) != (STANDARD, KEY) {
                return Err(zbus::fdo::Error::InvalidArgs("unknown setting".into()));
            }
            self.captured.send(()).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            self.released.recv().await.map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            OwnedValue::try_from(Value::Value(Box::new(Value::U32(2)))).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
        }
    }
    let (_daemon, address) = private_bus("craft-appearance-test")?;
    let (captured, seen) = std::sync::mpsc::channel();
    let (release, released) = async_channel::bounded(1);
    let server = futures_lite::future::block_on(
        zbus::connection::Builder::address(address.trim())?.name(DEST)?.serve_at(PATH, Settings { captured, released })?.build(),
    )?;
    let client = futures_lite::future::block_on(zbus::connection::Builder::address(address.trim())?.build())?;
    let (woke, wakes) = std::sync::mpsc::channel();
    let tracker = Watcher::spawn(
        true,
        move || {
            let _ = woke.send(());
        },
        move |shared| async move {
            let _ = watch_connection(&shared, &mut Fallbacks::used(), &mut false, client).await;
        },
    )
    .ok_or("could not start observer")?;
    seen.recv_timeout(Duration::from_secs(2))?;
    futures_lite::future::block_on(server.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(STANDARD, KEY, OwnedValue::from(1u32))))?;
    release.send_blocking(())?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), Some(Theme::Dark));
    // A malformed update must not clear the valid value or cause a wake.
    futures_lite::future::block_on(server.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(STANDARD, KEY, OwnedValue::from(true))))?;
    assert!(wakes.recv_timeout(Duration::from_millis(50)).is_err());
    assert_eq!(tracker.current(), Some(Theme::Dark));
    let before = std::time::Instant::now();
    drop(tracker);
    assert!(before.elapsed() < Duration::from_secs(1));
    // Drop also cancels a backend that has entered Read and never replies.
    let client = futures_lite::future::block_on(zbus::connection::Builder::address(address.trim())?.build())?;
    let pending = Watcher::spawn(
        true,
        || {},
        move |shared| async move {
            let _ = watch_connection(&shared, &mut Fallbacks::used(), &mut false, client).await;
        },
    )
    .ok_or("could not start pending-read observer")?;
    seen.recv_timeout(Duration::from_secs(2))?;
    let before = std::time::Instant::now();
    drop(pending);
    assert!(before.elapsed() < Duration::from_secs(1));
    // A newly available portal uses ReadOne, and a replacement owner must trigger a reread.
    futures_lite::future::block_on(server.release_name(DEST))?;
    struct ModernSettings(u32);
    #[zbus::interface(name = "org.freedesktop.portal.Settings")]
    impl ModernSettings {
        fn read_one(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
            if (namespace, key) == (STANDARD, KEY) {
                Ok(OwnedValue::from(self.0))
            } else {
                Err(zbus::fdo::Error::InvalidArgs("unknown setting".into()))
            }
        }
    }
    let client = futures_lite::future::block_on(zbus::connection::Builder::address(address.trim())?.build())?;
    let (woke, wakes) = std::sync::mpsc::channel();
    let (attempted, attempts) = std::sync::mpsc::channel();
    let tracker = Watcher::spawn(
        true,
        move || {
            let _ = woke.send(());
        },
        move |shared| async move {
            // The production observer repeats this session after its bounded reconnect backoff.
            loop {
                let _ = watch_connection(&shared, &mut Fallbacks::used(), &mut false, client.clone()).await;
                let _ = attempted.send(());
                async_io::Timer::after(Duration::from_millis(10)).await;
            }
        },
    )
    .ok_or("could not start modern observer")?;
    attempts.recv_timeout(Duration::from_secs(2))?;
    let modern =
        futures_lite::future::block_on(zbus::connection::Builder::address(address.trim())?.name(DEST)?.serve_at(PATH, ModernSettings(2))?.build())?;

    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), Some(Theme::Light));
    futures_lite::future::block_on(modern.release_name(DEST))?;
    let replacement =
        futures_lite::future::block_on(zbus::connection::Builder::address(address.trim())?.name(DEST)?.serve_at(PATH, ModernSettings(1))?.build())?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), Some(Theme::Dark));
    futures_lite::future::block_on(replacement.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(STANDARD, KEY, OwnedValue::from(0u32))))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), None);
    // GNOME's string schema fills the empty standard preference. Manual mode still caches it,
    // and opting back into System sees that latest cache without a redundant wake.
    tracker.set_active(false);
    futures_lite::future::block_on(replacement.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(GNOME, KEY, Value::from("prefer-dark"))))?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while tracker.current() != Some(Theme::Dark) {
        assert!(std::time::Instant::now() < deadline, "GNOME change was not cached");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(wakes.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    tracker.set_active(true);
    assert_eq!(tracker.current(), Some(Theme::Dark));
    assert!(matches!(wakes.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    futures_lite::future::block_on(replacement.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(GNOME, KEY, Value::from("default"))))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), None);
    drop(tracker);
    Ok(())
}

#[test]
#[ignore = "requires dbus-daemon; private session bus only"]
fn button_layout_is_read_and_followed_on_the_theme_connection() -> Result<(), Box<dyn std::error::Error>> {
    struct Settings;
    #[zbus::interface(name = "org.freedesktop.portal.Settings")]
    impl Settings {
        fn read_one(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
            match (namespace, key) {
                (STANDARD, KEY) => Ok(OwnedValue::from(1u32)),
                (WM, LAYOUT_KEY) => OwnedValue::try_from(Value::from("appmenu:close")).map_err(|e| zbus::fdo::Error::Failed(e.to_string())),
                _ => Err(zbus::fdo::Error::InvalidArgs("unknown setting".into())),
            }
        }
    }
    let (_daemon, address) = private_bus("craft-appearance-layout-test")?;
    let server = futures_lite::future::block_on(zbus::connection::Builder::address(address.as_str())?.name(DEST)?.serve_at(PATH, Settings)?.build())?;
    let client = futures_lite::future::block_on(zbus::connection::Builder::address(address.as_str())?.build())?;
    let (woke, wakes) = std::sync::mpsc::channel();
    let tracker = Watcher::spawn(
        true,
        move || {
            let _ = woke.send(());
        },
        move |shared| async move {
            let _ = watch_connection(&shared, &mut Fallbacks::used(), &mut false, client).await;
        },
    )
    .ok_or("could not start observer")?;
    // The theme, then the layout: one wake each.
    wakes.recv_timeout(Duration::from_secs(2))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), Some(Theme::Dark));
    assert_eq!(tracker.button_layout().as_deref(), Some("appmenu:close"));
    let emit = |namespace: &str, key: &str, value: Value<'_>| {
        futures_lite::future::block_on(server.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(namespace, key, value)))
    };
    emit(WM, LAYOUT_KEY, Value::from("close,minimize:"))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.button_layout().as_deref(), Some("close,minimize:"));
    // A malformed value, another namespace's key of the same name and a repeat change nothing.
    emit(WM, LAYOUT_KEY, Value::U32(3))?;
    emit("org.example.other", LAYOUT_KEY, Value::from("close:"))?;
    emit(WM, LAYOUT_KEY, Value::from("close,minimize:"))?;
    assert!(wakes.recv_timeout(Duration::from_millis(100)).is_err());
    assert_eq!(tracker.button_layout().as_deref(), Some("close,minimize:"));
    // The theme still follows its own signal on the same connection.
    emit(STANDARD, KEY, Value::U32(2))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.current(), Some(Theme::Light));
    assert_eq!(tracker.button_layout().as_deref(), Some("close,minimize:"));
    let before = std::time::Instant::now();
    drop(tracker);
    assert!(before.elapsed() < Duration::from_secs(1));
    Ok(())
}

#[test]
#[ignore = "requires dbus-daemon; private session bus only"]
fn contrast_and_gnome_animations_settle_with_the_theme_and_follow_signals() -> Result<(), Box<dyn std::error::Error>> {
    // A portal without `reduced-motion`: GNOME's `enable-animations` stands in until it appears.
    struct Settings;
    #[zbus::interface(name = "org.freedesktop.portal.Settings")]
    impl Settings {
        fn read_one(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
            match (namespace, key) {
                (STANDARD, KEY) => Ok(OwnedValue::from(2u32)),
                (STANDARD, CONTRAST_KEY) => Ok(OwnedValue::from(1u32)),
                (GNOME, ANIMATIONS_KEY) => Ok(OwnedValue::from(false)),
                _ => Err(zbus::fdo::Error::InvalidArgs("unknown setting".into())),
            }
        }
    }
    let (_daemon, address) = private_bus("craft-appearance-a11y-test")?;
    let server = futures_lite::future::block_on(zbus::connection::Builder::address(address.as_str())?.name(DEST)?.serve_at(PATH, Settings)?.build())?;
    let client = futures_lite::future::block_on(zbus::connection::Builder::address(address.as_str())?.build())?;
    let (woke, wakes) = std::sync::mpsc::channel();
    let tracker = Watcher::spawn(
        false,
        move || {
            let _ = woke.send(());
        },
        move |shared| async move {
            let _ = watch_connection(&shared, &mut Fallbacks::used(), &mut false, client).await;
        },
    )
    .ok_or("could not start observer")?;
    assert_eq!(tracker.wait_first(Duration::from_secs(2)), Some(Theme::Light));
    assert_eq!(tracker.contrast(), Some(Contrast::High));
    assert_eq!(tracker.reduced_motion(), Some(true));
    tracker.set_active(true);
    let emit = |namespace: &str, key: &str, value: Value<'_>| {
        futures_lite::future::block_on(server.emit_signal(None::<&str>, PATH, IFACE, "SettingChanged", &(namespace, key, value)))
    };
    emit(GNOME, ANIMATIONS_KEY, Value::Bool(true))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.reduced_motion(), Some(false));
    // The portal key, once it answers, wins over GNOME's.
    emit(STANDARD, MOTION_KEY, Value::U32(1))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.reduced_motion(), Some(true));
    emit(GNOME, ANIMATIONS_KEY, Value::Bool(true))?;
    emit(STANDARD, CONTRAST_KEY, Value::from("high"))?;
    assert!(wakes.recv_timeout(Duration::from_millis(100)).is_err());
    assert_eq!(tracker.reduced_motion(), Some(true));
    assert_eq!(tracker.contrast(), Some(Contrast::High));
    emit(STANDARD, CONTRAST_KEY, Value::U32(0))?;
    wakes.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(tracker.contrast(), Some(Contrast::Standard));
    drop(tracker);
    Ok(())
}
