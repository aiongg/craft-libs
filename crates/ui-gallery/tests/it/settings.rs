//! The gallery's settings: a store that cannot write is retried on its own clock.

use craft_settings::store::retry_delay;
use craft_settings::{Location, OpenOptions, SettingsFiles, SettingsStore};
use craft_ui::shell::app::ShellApp as _;
use craft_ui::testing::trial;
use craft_ui_gallery::chrome::Gallery;
use craft_ui_gallery::viewer::{Catalog, GallerySettings};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Files that hold nothing and refuse every write, counting the attempts.
#[derive(Default)]
struct FullDisk {
    writes: AtomicUsize,
}

impl SettingsFiles for FullDisk {
    fn read(&self, _: &Path) -> io::Result<Option<String>> {
        Ok(None)
    }

    fn write(&self, _: &Path, _: &str) -> io::Result<()> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::other("the disk is full"))
    }

    fn move_aside(&self, path: &Path) -> io::Result<PathBuf> {
        Ok(path.to_path_buf())
    }

    fn write_backup(&self, _: &Path, _: &str) -> io::Result<()> {
        Ok(())
    }

    fn check_writable(&self, _: &Path) -> io::Result<()> {
        Ok(())
    }
}

#[trial]
fn a_save_that_fails_is_retried_on_its_backoff_and_only_then() {
    let files = Arc::new(FullDisk::default());
    let options = OpenOptions::new(Location::Dir(PathBuf::from("settings"))).files(files.clone());
    let (mut store, report) = SettingsStore::<GallerySettings>::open_with(options);
    let changed = Instant::now();
    assert!(store.update_at(changed, |settings| settings.filter = "link".to_owned()));
    let mut gallery = Gallery::new(Catalog::listing(Vec::new()), store, &report);

    let due = gallery.tick(changed).expect("a change waits for the debounce and names when it ends");
    assert!(due > changed);
    assert_eq!(files.writes.load(Ordering::SeqCst), 0, "nothing is written before the debounce ends");

    let retry = gallery.tick(due).expect("a failed save names its retry");
    assert_eq!(files.writes.load(Ordering::SeqCst), 1);
    assert!(retry >= due + retry_delay(1));

    for pass in 1..=5 {
        let between = due + Duration::from_millis(pass);
        assert_eq!(gallery.tick(between), Some(retry), "a pass before the retry only repeats its time");
    }
    assert_eq!(files.writes.load(Ordering::SeqCst), 1, "passes before the retry do not write");

    gallery.tick(retry);
    assert_eq!(files.writes.load(Ordering::SeqCst), 2, "the retry writes once");
}
