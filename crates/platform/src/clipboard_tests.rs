//! `craft_platform::clipboard`.
#![allow(clippy::unwrap_used, reason = "tests fail by panicking")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use craft_host::Wake;

use crate::clipboard::{rgba8_flavor, rgba8_image, *};

/// A backend that holds whatever the last write or `held` assignment left, and reads without
/// enforcing any limit (the [`Clipboard`] must).
#[derive(Clone, Default)]
struct Fake(Arc<Mutex<Vec<Flavor>>>);

impl Fake {
    fn copy_elsewhere(&self, flavors: Vec<Flavor>) {
        *self.0.lock().unwrap() = flavors;
    }
}

impl ClipboardBackend for Fake {
    fn write(&mut self, flavors: &[Flavor]) -> Result<(), ClipboardError> {
        *self.0.lock().unwrap() = flavors.to_vec();
        Ok(())
    }
    fn read(&mut self, mimes: &[&str], _limit: u64) -> Result<Option<Flavor>, ClipboardError> {
        let held = self.0.lock().unwrap();
        Ok(mimes.iter().find_map(|mime| held.iter().find(|f| f.mime == *mime).cloned()))
    }
    fn read_image(&mut self, _limit: u64) -> Result<Option<ClipboardImage>, ClipboardError> {
        Ok(self.0.lock().unwrap().iter().find(|f| f.mime == RGBA8).and_then(|f| rgba8_image(&f.data)))
    }
    fn read_files(&mut self, _limit: usize) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        Ok(None)
    }
    fn has(&mut self, mimes: &[&str]) -> bool {
        self.0.lock().unwrap().iter().any(|f| mimes.contains(&&*f.mime))
    }
}

fn image(seed: u8) -> ClipboardImage {
    ClipboardImage { width: 4, height: 2, rgba: (0..32).map(|i| i as u8 ^ seed).collect() }
}

// While synthetic, write_text and write_image answer Fenced and the backend keeps the user's
// copy; reads still work; after set_synthetic(false) writes go through.
#[test]
fn synthetic_input_never_replaces_what_the_user_copied() {
    let fake = Fake::default();
    fake.copy_elsewhere(vec![Flavor::text("the user's")]);
    let mut clipboard = Clipboard::new(fake.clone());
    clipboard.set_synthetic(true);
    assert_eq!(clipboard.write_text("injected").unwrap(), Written::Fenced);
    assert_eq!(clipboard.write_image(&image(1)).unwrap(), Written::Fenced);
    assert_eq!(clipboard.read_text().unwrap().as_deref(), Some("the user's"));
    clipboard.set_synthetic(false);
    assert_eq!(clipboard.write_text("mine").unwrap(), Written::Written);
    assert_eq!(clipboard.read_text().unwrap().as_deref(), Some("mine"));
}

// An image over ClipboardLimits::image_bytes and text over text_bytes fail with TooLarge and the
// limit; an image whose rgba length doesn't match its size reads as None.
#[test]
fn reads_over_the_limits_fail_and_broken_images_are_dropped() {
    let fake = Fake::default();
    let mut clipboard = Clipboard::new(fake.clone()).limits(ClipboardLimits { image_bytes: 16, text_bytes: 4, files: 1 });
    fake.copy_elsewhere(vec![rgba8_flavor(&image(0))]);
    assert_eq!(clipboard.read_image(), Err(ClipboardError::TooLarge { limit: 16 }));
    fake.copy_elsewhere(vec![Flavor::text("hello")]);
    assert_eq!(clipboard.read_text(), Err(ClipboardError::TooLarge { limit: 4 }));
    fake.copy_elsewhere(vec![Flavor::text("hi")]);
    assert_eq!(clipboard.read_text().unwrap().as_deref(), Some("hi"));

    let mut clipboard = Clipboard::new(fake.clone());
    let mut broken = image(0);
    broken.rgba.pop();
    fake.copy_elsewhere(vec![rgba8_flavor(&broken)]);
    assert_eq!(clipboard.read_image(), Ok(None));
}

// After write_image, holds_ours and is_ours(read image) are true while the backend returns the
// same pixels and false once another image (or text) replaced them; forget_ours clears it.
#[test]
fn holds_ours_tells_our_copy_from_another_apps() {
    let fake = Fake::default();
    let mut clipboard = Clipboard::new(fake.clone());
    assert!(!clipboard.holds_ours());
    let mine = image(3);
    assert_eq!(clipboard.write_image(&mine).unwrap(), Written::Written);
    assert!(clipboard.holds_ours());
    let read = clipboard.read_image().unwrap().unwrap();
    assert!(clipboard.is_ours(&read));
    assert!(!clipboard.is_ours(&image(9)));

    fake.copy_elsewhere(vec![rgba8_flavor(&image(9))]);
    assert!(!clipboard.holds_ours());
    fake.copy_elsewhere(vec![Flavor::text("other")]);
    assert!(!clipboard.holds_ours());

    assert_eq!(clipboard.write_text("mine").unwrap(), Written::Written);
    assert!(clipboard.holds_ours());
    fake.copy_elsewhere(vec![Flavor::text("theirs")]);
    assert!(!clipboard.holds_ours());

    assert_eq!(clipboard.write_image(&mine).unwrap(), Written::Written);
    clipboard.forget_ours();
    assert!(!clipboard.holds_ours() && !clipboard.is_ours(&mine));
}

// An EventClipboard reads text, an RGBA8 image and a text/uri-list (CRLF line ends trimmed) from
// the last offer_paste, and hands writes to its out closure.
#[test]
fn event_clipboard_reads_the_offered_paste() {
    let out = Arc::new(Mutex::new(Vec::new()));
    let sink = out.clone();
    let mut clipboard = Clipboard::new(EventClipboard::new(move |flavors| {
        sink.lock().unwrap().extend_from_slice(flavors);
        Ok(())
    }));
    assert_eq!(clipboard.read_text(), Ok(None));
    clipboard.offer_paste(vec![
        Flavor::text("pasted"),
        rgba8_flavor(&image(5)),
        Flavor::new(URI_LIST, b"# files\r\nfile:///tmp/a%20b.png\r\nhttps://example.com/x\r\nfile:///tmp/c.png\r\n".to_vec()),
    ]);
    assert_eq!(clipboard.read_text().unwrap().as_deref(), Some("pasted"));
    assert_eq!(clipboard.read_image().unwrap(), Some(image(5)));
    assert_eq!(clipboard.read_files().unwrap(), Some(vec![PathBuf::from("/tmp/a b.png"), PathBuf::from("/tmp/c.png")]));
    assert!(clipboard.has(&[BITMAP]));
    clipboard.offer_paste(vec![Flavor::text("next")]);
    assert_eq!(clipboard.read_image(), Ok(None));
    assert!(!clipboard.has(&[BITMAP]));

    assert_eq!(clipboard.write_text("copied").unwrap(), Written::Written);
    assert_eq!(*out.lock().unwrap(), vec![Flavor::text("copied")]);
}

struct Slow {
    has: Arc<AtomicBool>,
    delay: Duration,
    released: Arc<AtomicBool>,
}

impl ClipboardBackend for Slow {
    fn write(&mut self, _: &[Flavor]) -> Result<(), ClipboardError> {
        Ok(())
    }
    fn read(&mut self, _: &[&str], _: u64) -> Result<Option<Flavor>, ClipboardError> {
        Ok(None)
    }
    fn read_image(&mut self, _: u64) -> Result<Option<ClipboardImage>, ClipboardError> {
        Ok(None)
    }
    fn read_files(&mut self, _: usize) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        Ok(None)
    }
    fn has(&mut self, _: &[&str]) -> bool {
        std::thread::sleep(self.delay);
        self.has.load(Ordering::SeqCst)
    }
}

impl Drop for Slow {
    fn drop(&mut self) {
        self.released.store(true, Ordering::SeqCst);
    }
}

fn wait_for(probe: &mut ClipboardProbe, wanted: bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while probe.pasteable(true) != Some(wanted) {
        assert!(Instant::now() < end, "the probe never reported {wanted}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

// The probe reports a backend's has() verdict without blocking the caller, sends again only on a
// change, and dropping it ends the worker within PROBE_STOP_WAIT.
#[test]
fn probe_answers_off_the_ui_thread_and_stops_on_drop() {
    let (has, released, wakes) = (Arc::new(AtomicBool::new(true)), Arc::new(AtomicBool::new(false)), Arc::new(AtomicUsize::new(0)));
    let backend = Slow { has: has.clone(), delay: Duration::ZERO, released: released.clone() };
    let count = wakes.clone();
    let mut probe = ClipboardProbe::start(
        Box::new(move || Box::new(backend)),
        vec![TEXT],
        Wake::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        }),
    )
    .unwrap();
    assert_eq!(probe.pasteable(false), Some(false));
    wait_for(&mut probe, true);
    has.store(false, Ordering::SeqCst);
    wait_for(&mut probe, false);
    std::thread::sleep(PROBE_INTERVAL * 2);
    assert_eq!(wakes.load(Ordering::SeqCst), 2, "one wake per change of verdict");
    drop(probe);
    assert!(released.load(Ordering::SeqCst), "drop waits for the worker to release its handle");

    let slow = Slow { has: Arc::new(AtomicBool::new(true)), delay: Duration::from_secs(1), released: Arc::default() };
    let mut probe = ClipboardProbe::start(Box::new(move || Box::new(slow)), vec![TEXT], Wake::none()).unwrap();
    let start = Instant::now();
    assert_eq!(probe.pasteable(true), Some(false));
    drop(probe);
    assert!(start.elapsed() < PROBE_STOP_WAIT + Duration::from_millis(500), "a stuck owner doesn't hold the UI");
}
