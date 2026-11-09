//! The gallery's settings: a store that cannot write is retried on its own clock.

use craft_settings::store::retry_delay;
use craft_settings::{Location, OpenOptions, SettingsFiles, SettingsStore};
use craft_ui::runtime::motion;
use craft_ui::shell::app::ShellApp as _;
use craft_ui::testing::trial;
use craft_ui_gallery::chrome::Gallery;
use craft_ui_gallery::viewer::{Catalog, GallerySettings};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

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

