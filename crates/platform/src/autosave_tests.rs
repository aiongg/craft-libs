//! `craft_platform::autosave`.
#![allow(clippy::unwrap_used, reason = "tests fail by panicking")]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::autosave::*;

/// A fresh folder under the test temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!("craft-autosave-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn doc(id: u64, revision: u64, dirty: bool) -> AutosaveDoc {
    AutosaveDoc { id, revision, dirty }
}

fn meta(name: &str, revision: u64) -> RecoveryMeta {
    RecoveryMeta { name: name.into(), original: None, extension: "pcraft".into(), revision }
}

// Nothing is due before the interval; then only dirty documents with a new revision and no
// autosave in flight; a failure returns "Autosave failed: …".
#[test]
fn due_picks_dirty_changed_documents_each_interval() {
    let minute = Duration::from_secs(60);
    let t0 = Instant::now();
    let mut timer = Autosave::new(Some(minute));
    let docs = |rev| [doc(1, rev, true), doc(2, 1, false)];
    assert!(timer.due(t0, docs(1)).save.is_empty(), "the first call starts the clock");
    assert_eq!(timer.deadline(), Some(t0 + minute));
    assert!(timer.due(t0 + minute / 2, docs(1)).save.is_empty());
    assert_eq!(timer.due(t0 + minute, docs(1)).save, vec![1]);
    assert!(timer.due(t0 + minute * 2, docs(1)).save.is_empty(), "still in flight");
    assert_eq!(timer.finished(1, 1, Ok(())), None);
    assert!(timer.due(t0 + minute * 3, docs(1)).save.is_empty(), "revision already autosaved");
    assert_eq!(timer.due(t0 + minute * 4, docs(2)).save, vec![1]);
    assert_eq!(timer.finished(1, 2, Err("disk full".into())).as_deref(), Some("Autosave failed: disk full"));
    assert_eq!(timer.due(t0 + minute * 5, docs(2)).save, vec![1], "a failed revision is tried again");
    timer.forget(1);
    assert_eq!(timer.finished(1, 2, Ok(())), None, "a result after forget is ignored");

    // A document that turns clean without a save is handed back for discarding, once.
    let mut timer = Autosave::new(Some(minute));
    timer.due(t0, docs(1));
    assert_eq!(timer.due(t0 + minute, docs(1)).save, vec![1]);
    assert_eq!(timer.finished(1, 1, Ok(())), None);
    let clean = timer.due(t0 + minute * 2, [doc(1, 1, false)]);
    assert_eq!(clean.discard, vec![1]);
    assert!(timer.due(t0 + minute * 3, [doc(1, 1, false)]).discard.is_empty());

    // An old job's result does not complete the new job of the same document.
    let mut timer = Autosave::new(Some(minute));
    timer.due(t0, docs(1));
    assert_eq!(timer.due(t0 + minute, docs(1)).save, vec![1]);
    timer.forget(1);
    assert_eq!(timer.due(t0 + minute * 2, docs(2)).save, vec![1]);
    assert_eq!(timer.finished(1, 1, Err("late".into())), None, "revision 1 is not the one in flight");
    assert!(timer.due(t0 + minute * 3, docs(2)).save.is_empty(), "revision 2 is still in flight");
    assert_eq!(timer.finished(1, 2, Ok(())), None);

    timer.set_interval(None);
    assert!(timer.due(t0 + minute * 9, docs(3)).save.is_empty());
    assert_eq!(timer.deadline(), None);
}

// Platform mode: LOCALAPPDATA on Windows, an absolute XDG_STATE_HOME or ~/.local/state on Linux,
// the config folder on macOS; self-contained mode always uses the config folder.
#[test]
fn recovery_dir_follows_the_platform_rules() {
    let config = Path::new("/cfg/photocraft");
    let env = |vars: &'static [(&'static str, &'static str)]| move |k: &str| vars.iter().find(|(n, _)| *n == k).map(|(_, v)| OsString::from(*v));
    let platform = |vars| recovery_dir("Photocraft", "photocraft", Some(config), SettingsMode::Platform, env(vars));
    let own = |vars| recovery_dir("Photocraft", "photocraft", Some(config), SettingsMode::SelfContained, env(vars));
    let all = &[("LOCALAPPDATA", "/local"), ("XDG_STATE_HOME", "/state"), ("HOME", "/home/u")];
    assert_eq!(own(all), Some(config.join("Recovery")));
    assert_eq!(recovery_dir("Photocraft", "photocraft", None, SettingsMode::SelfContained, env(all)), None);
    if cfg!(windows) {
        assert_eq!(platform(all), Some(PathBuf::from("/local").join("Photocraft").join("Recovery")));
        assert_eq!(platform(&[]), Some(config.join("Recovery")));
    } else if cfg!(target_os = "macos") {
        assert_eq!(platform(all), Some(config.join("Recovery")));
    } else {
        assert_eq!(platform(all), Some(Path::new("/state/photocraft/Recovery").to_path_buf()));
        assert_eq!(
            platform(&[("XDG_STATE_HOME", "rel"), ("HOME", "/home/u")]),
            Some(Path::new("/home/u/.local/state/photocraft/Recovery").to_path_buf())
        );
        assert_eq!(platform(&[]), Some(config.join("Recovery")));
    }
}

// A copy written by a run that is gone is a leftover; one of a live instance (its lock held) is
// not; discard removes copy and sidecar.
#[test]
fn leftovers_skip_live_instances() {
    let scratch = Scratch::new();
    let dir = scratch.0.join("Recovery");
    let first = RecoveryStore::open(&dir).unwrap();
    first.write(1, &meta("a.psd", 3), b"first").unwrap();
    first.write(2, &meta("b.psd", 1), b"second").unwrap();
    first.discard(2).unwrap();
    assert!(!dir.join(format!("{}.pcraft", first.key(2))).exists());
    assert!(!dir.join(format!("{}.json", first.key(2))).exists());

    let second = RecoveryStore::open(&dir).unwrap();
    assert!(second.leftovers().is_empty(), "the first run is still alive");
    assert!(first.leftovers().is_empty(), "a store never offers its own copies");

    let key = first.key(1);
    drop(first);
    let found = second.leftovers();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key, key);
    assert_eq!(found[0].meta, meta("a.psd", 3));
    assert_eq!(std::fs::read(&found[0].path).unwrap(), b"first");

    // Recovering adopts the key: the next autosave replaces the copy in place.
    second.adopt(7, &found[0]);
    assert_eq!(second.key(7), key);
    second.write(7, &meta("a.psd", 4), b"third").unwrap();
    assert!(second.leftovers().is_empty(), "adopted copies now belong to the live run");
    second.remove(&found[0]).unwrap();
    assert!(!found[0].path.exists());
}

// The instance name carries the session, so a run that gets a crashed run's PID and counter
// still sees that run's copies as leftovers.
#[test]
fn instance_names_carry_the_session() {
    let scratch = Scratch::new();
    let dir = scratch.0.join("Recovery");
    let store = RecoveryStore::open(&dir).unwrap();
    store.write(1, &meta("a.psd", 1), b"x").unwrap();
    let sidecar = std::fs::read_to_string(dir.join(format!("{}.json", store.key(1)))).unwrap();
    let session = store.key(1).strip_prefix("doc-").unwrap().strip_suffix("-1").unwrap().to_owned();
    let name = format!("instance-{}.lock", session.replace('_', "-"));
    assert!(sidecar.contains(&name), "{sidecar} lacks {name}");

    // The crashed run of the same PID and counter, with a lock nobody holds.
    let pid = std::process::id();
    let crashed = format!("instance-0-{pid:x}-0.lock");
    std::fs::write(dir.join(&crashed), b"").unwrap();
    std::fs::write(dir.join("doc-old-1.pcraft"), b"old").unwrap();
    std::fs::write(dir.join("doc-old-1.json"), format!(r#"{{"name":"o.psd","extension":"pcraft","revision":1,"instance":"{crashed}"}}"#)).unwrap();
    assert_eq!(store.leftovers().len(), 1);
}

// Another live run's half-written entry (copy without its sidecar) is never listed; once that
// run is gone, the copy is a leftover.
#[test]
fn half_written_entries_of_live_runs_are_not_listed() {
    let scratch = Scratch::new();
    let dir = scratch.0.join("Recovery");
    let first = RecoveryStore::open(&dir).unwrap();
    let second = RecoveryStore::open(&dir).unwrap();
    let key = first.key(1);
    std::fs::write(dir.join(format!("{key}.pcraft")), b"half").unwrap();
    assert!(second.leftovers().is_empty());
    assert!(first.leftovers().is_empty(), "nor its own");
    drop(first);
    let found = second.leftovers();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key, key);
}

// An interrupted legacy move (.moving entries) is finished by the next launch.
#[test]
fn legacy_recovery_move_resumes() {
    let scratch = Scratch::new();
    let legacy = scratch.0.join("legacy");
    let dir = scratch.0.join("state").join("Recovery");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(legacy.join("k.pcraft"), b"copy").unwrap();
    std::fs::write(legacy.join("k.json"), b"{}").unwrap();
    // A process died after setting the sidecar aside and staging another folder.
    std::fs::rename(legacy.join("k.json"), legacy.join(".k.json.4242-0.held")).unwrap();
    std::fs::create_dir(dir.parent().unwrap().join(".Recovery.4242-1.moving")).unwrap();
    std::fs::write(dir.join("other.pcraft"), b"x").unwrap();

    let used = adopt_legacy_recovery(&legacy, &dir);
    assert_eq!(used, dir);
    assert_eq!(std::fs::read(dir.join("k.pcraft")).unwrap(), b"copy");
    assert_eq!(std::fs::read(dir.join("k.json")).unwrap(), b"{}");
    assert_eq!(std::fs::read(dir.join("other.pcraft")).unwrap(), b"x");
    assert!(!legacy.exists());
    assert!(!dir.parent().unwrap().join(".Recovery.4242-1.moving").exists());
}

// Keys keep letters, digits, '-' and '_'; others become '_'; empty is "untitled".
#[test]
fn keys_are_sanitized() {
    assert_eq!(sanitize_key("doc-1_A"), "doc-1_A");
    assert_eq!(sanitize_key("../a b.c"), "___a_b_c");
    assert_eq!(sanitize_key(""), "untitled");
}
