//! Link status, identity, candidates, remaps, folder matches and the search, over a fake
//! `LinkFiles` and (desktop) a temporary folder.

use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use super::*;

/// A fake file system: files by path with their bytes and modification time.
#[derive(Default)]
struct Fake {
    files: BTreeMap<PathBuf, (Vec<u8>, Option<SystemTime>)>,
    unreadable: Vec<PathBuf>,
    calls: AtomicUsize,
}

impl Fake {
    fn with(mut self, path: &str, bytes: &[u8], secs: u64) -> Self {
        self.files.insert(PathBuf::from(path), (bytes.to_vec(), Some(stamp(secs))));
        self
    }
}

fn stamp(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

impl LinkFiles for Fake {
    fn stat(&self, path: &Path) -> Option<FileInfo> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.files.get(path).map(|(b, m)| FileInfo { size: b.len() as u64, modified: *m })
    }

    fn identity(&self, path: &Path) -> io::Result<LinkIdentity> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.unreadable.iter().any(|p| p == path) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
        }
        let (bytes, modified) = self.files.get(path).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        identity_of(&mut Cursor::new(bytes), bytes.len() as u64, *modified)
    }

    fn list(&self, _dir: &Path) -> io::Result<Vec<FolderEntry>> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

fn link(stored: &str, identity: Option<LinkIdentity>) -> Link<u32> {
    Link { key: 1, name: "a".into(), stored: stored.into(), identity, page: None, embedded: false }
}

fn identity_for(bytes: &[u8], secs: u64) -> LinkIdentity {
    identity_of(&mut Cursor::new(bytes), bytes.len() as u64, Some(stamp(secs))).unwrap()
}

// Network (`\\host\s\a.png`), device (`\\.\COM1`) and URL (`https://h/a.png`) links are Refused, and the fake saw no call.
#[test]
fn a_status_is_missing_for_a_link_that_may_not_be_followed() {
    let fake = Fake::default();
    for (stored, why) in [(r"\\host\s\a.png", Refused::Network), (r"\\.\COM1", Refused::Device), ("https://h/a.png", Refused::Url)] {
        assert_eq!(status(&link(stored, None), None, &fake), LinkStatus::Refused(why), "{stored}");
    }
    assert_eq!(fake.calls.load(Ordering::Relaxed), 0);
    assert!(LinkStatus::Refused(Refused::Url).is_missing());
    assert!(!LinkStatus::Modified.is_missing());
}

// Same stamp: Ok without `identity`; new mtime, same fingerprint: Ok; new bytes: Modified; read error: Unreadable.
#[test]
fn a_touched_file_is_ok_and_a_changed_file_is_modified() {
    let recorded = identity_for(b"hello world", 10);
    let l = link("/m/a.png", Some(recorded));
    let at = |bytes: &[u8], secs: u64| Fake::default().with("/m/a.png", bytes, secs);

    let same = at(b"hello world", 10);
    assert_eq!(status(&l, None, &same), LinkStatus::Ok);
    assert_eq!(same.calls.load(Ordering::Relaxed), 1, "the stamp matched, so nothing was hashed");
    assert_eq!(status(&l, None, &at(b"hello world", 99)), LinkStatus::Ok);
    assert_eq!(status(&l, None, &at(b"hello w0rld", 99)), LinkStatus::Modified);
    assert_eq!(status(&l, None, &at(b"hello", 99)), LinkStatus::Modified);
    let mut locked = at(b"hello world", 99);
    locked.unreadable.push("/m/a.png".into());
    assert_eq!(status(&l, None, &locked), LinkStatus::Unreadable);
    assert_eq!(status(&l, None, &Fake::default()), LinkStatus::Missing);
    assert_eq!(status(&link("/m/a.png", None), None, &at(b"x", 1)), LinkStatus::Ok);
    let embedded = Link { embedded: true, ..l };
    assert_eq!(status(&embedded, None, &Fake::default()), LinkStatus::Embedded);
}

// Same name, other bytes: not ok, `same_file == Some(false)`, a DifferentFile problem; the same file scores highest.
#[test]
fn a_different_file_with_the_same_name_is_refused_by_identity() {
    let l = link(r"C:\shoot\A.PNG", Some(identity_for(b"original", 1)));
    let fake = Fake::default().with("/x/a.png", b"original", 5).with("/y/a.png", b"other!!!", 5).with("/z/b.jpg", b"original", 5);
    let c = MatchCriteria::default();

    let mut found = vec![
        check_candidate(&l, Path::new("/y/a.png"), &c, &fake),
        check_candidate(&l, Path::new("/z/b.jpg"), &c, &fake),
        check_candidate(&l, Path::new("/x/a.png"), &c, &fake),
        check_candidate(&l, Path::new("/missing/a.png"), &c, &fake),
    ];
    assert!(!found[0].ok);
    assert_eq!(found[0].same_file, Some(false));
    assert_eq!(found[0].problems, vec![CandidateProblem::DifferentFile { found: 8, wanted: 8 }]);
    assert_eq!(found[1].same_file, Some(true));
    assert_eq!(found[1].problems.len(), 2, "name and extension differ");
    assert!(found[2].ok && found[2].problems.is_empty());
    assert_eq!(found[2].score, 15);
    assert!(matches!(found[3].problems.as_slice(), [CandidateProblem::Unreadable(_)]));
    assert_eq!(found[3].identity, None);

    rank(&mut found);
    let order: Vec<_> = found.iter().map(|c| c.path.to_str().unwrap()).collect();
    // The last two tie on score (name and extension), so the path decides.
    assert_eq!(order, ["/x/a.png", "/z/b.jpg", "/missing/a.png", "/y/a.png"]);

    let none = MatchCriteria { file_name: false, extension: false, same_file: false };
    assert!(check_candidate(&l, Path::new("/y/a.png"), &none, &fake).ok);
}

// `fingerprint` of a 3 MiB file equals FilmCraft's for the same bytes; a 2 MiB file is hashed whole.
#[test]
fn the_fingerprint_reads_the_head_and_the_tail() {
    let span = FINGERPRINT_SPAN;
    let big: Vec<u8> = (0..3 * span).map(|i| (i * 7 % 251) as u8).collect();
    let id = identity_of(&mut Cursor::new(&big), big.len() as u64, None).unwrap();
    assert_eq!(id.fingerprint, fingerprint(big.len() as u64, &big[..span], &big[big.len() - span..]));

    // A byte in the middle of a file over twice the span is outside what is hashed.
    let mut changed = big.clone();
    changed[span + 10] ^= 0xff;
    assert_eq!(identity_of(&mut Cursor::new(&changed), changed.len() as u64, None).unwrap().fingerprint, id.fingerprint);

    // Up to twice the span, the whole file is hashed: head plus the rest.
    let whole = &big[..2 * span];
    let id = identity_of(&mut Cursor::new(whole), whole.len() as u64, None).unwrap();
    assert_eq!(id.fingerprint, fingerprint(whole.len() as u64, &whole[..span], &whole[span..]));
    let mut changed = whole.to_vec();
    changed[span + 10] ^= 0xff;
    assert_ne!(identity_of(&mut Cursor::new(&changed), changed.len() as u64, None).unwrap().fingerprint, id.fingerprint);

    // A file shorter than it claims is an error.
    assert!(identity_of(&mut Cursor::new(&big[..100]), 200, None).is_err());
}

// `derive_remap(C:\shoot\day1\a.mov, /Volumes/T7/shoot/day1/a.mov)` then `apply` maps `C:\shoot\day2\b.mov`; other prefixes give None.
#[test]
fn a_remap_carries_to_the_other_missing_links() {
    let remap = derive_remap(r"C:\shoot\day1\a.mov", "/Volumes/T7/shoot/day1/a.mov").unwrap();
    assert_eq!(remap, Remap { from: "C:".into(), to: "/Volumes/T7".into() });
    assert_eq!(remap.apply(r"C:\shoot\day2\b.mov").as_deref(), Some("/Volumes/T7/shoot/day2/b.mov"));
    assert_eq!(remap.apply("D:/shoot/b.mov"), None);
    assert_eq!(remap.apply("C:other/b.mov"), None, "the prefix ends at a separator");
    assert_eq!(derive_remap("/a/b.mov", "/a/b.mov"), None);
    assert_eq!(derive_remap("/a/b.mov", "/a/c.mov"), None);
}

// `folder_match` takes only a plain name: `file:%2Fx.png`, `C:x.png`, `NUL`, `..` give None; an extension swap works.
#[test]
fn relink_to_folder_matches_only_the_plain_name_in_that_folder() {
    let folder = Path::new("/new");
    let m = |stored: &str, ext: Option<&str>| folder_match(&link(stored, None), folder, ext);
    assert_eq!(m(r"C:\old\a.png", None), Some(PathBuf::from("/new/a.png")));
    assert_eq!(m("/old/a.png", Some("jpg")), Some(PathBuf::from("/new/a.jpg")));
    for bad in ["file:%2Ftmp%2Fx.png", "C:x.png", "NUL", "..", "/old/.", ""] {
        assert_eq!(m(bad, None), None, "{bad}");
    }
    assert_eq!(m("/old/a.png", Some("j/pg")), None);
    assert_eq!(m("/old/a.png", Some(r"j\pg")), None);

    let fake = Fake::default().with("/new/a.png", b"x", 1);
    let links = [Link { key: 1, ..link("/old/a.png", None) }, Link { key: 2, ..link("/old/b.png", None) }, Link { key: 3, ..link("NUL", None) }];
    let (found, missing) = folder_matches(&links, folder, None, &fake);
    assert_eq!(found, vec![(1, PathBuf::from("/new/a.png"))]);
    assert_eq!(missing, vec![2, 3]);
}

// A folder link loop is not followed; dot files are skipped; the limit sets `truncated`.
#[cfg(unix)]
#[test]
fn the_search_never_follows_directory_links_and_stops_at_the_limit() {
    let root = std::env::temp_dir().join(format!("craft-docpath-index-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("A.png"), b"1").unwrap();
    std::fs::write(root.join(".hidden.png"), b"1").unwrap();
    std::fs::write(root.join("sub/a.PNG"), b"2").unwrap();
    std::os::unix::fs::symlink(&root, root.join("sub/loop")).unwrap();
    std::os::unix::fs::symlink(root.join("A.png"), root.join("sub/link.png")).unwrap();

    let index = index_folder(&root, 100, &SystemLinkFiles, &|| false);
    assert!(!index.truncated);
    assert_eq!(index.files, 3);
    assert_eq!(index.by_name["a.png"], vec![root.join("A.png"), root.join("sub/a.PNG")]);
    assert_eq!(index.by_name["link.png"], vec![root.join("sub/link.png")]);
    assert!(!index.by_name.contains_key(".hidden.png"));

    let cut = index_folder(&root, 2, &SystemLinkFiles, &|| false);
    assert!(cut.truncated);
    assert_eq!(cut.files, 2);

    let id = SystemLinkFiles.identity(&root.join("A.png")).unwrap();
    assert_eq!(id.size, 1);
    assert!(SystemLinkFiles.identity(&root.join("sub")).is_err());
    assert!(SystemLinkFiles.stat(&root.join("sub")).is_none());
    let _ = std::fs::remove_dir_all(&root);
}

// `cancelled` turning true mid-walk ends the run as Cancelled and no update follows it (task B5).
#[test]
#[ignore = "task B5: LinkSearch::run"]
#[expect(clippy::todo, reason = "B5 writes the body")]
fn a_cancelled_search_stops_and_reports_nothing_more() {
    todo!()
}

// `Link<String>` with an identity (and a `None` modified time) and `MatchCriteria` come back equal through serde_json; `{}` reads as `MatchCriteria::default()`.
#[test]
fn a_link_and_the_match_criteria_survive_a_serde_round_trip() {
    let l = Link {
        key: "asset-1".to_string(),
        name: "a.png".into(),
        stored: "../a.png".into(),
        identity: Some(LinkIdentity { size: 3, fingerprint: u64::MAX, modified: None }),
        page: Some(2),
        embedded: false,
    };
    let back: Link<String> = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
    assert_eq!(back, l);
    let timed = LinkIdentity { size: 1, fingerprint: 2, modified: Some(stamp(1_700_000_000)) };
    assert_eq!(serde_json::from_str::<LinkIdentity>(&serde_json::to_string(&timed).unwrap()).unwrap(), timed);

    let c = MatchCriteria { file_name: false, extension: true, same_file: false };
    assert_eq!(serde_json::from_str::<MatchCriteria>(&serde_json::to_string(&c).unwrap()).unwrap(), c);
    assert_eq!(serde_json::from_str::<MatchCriteria>("{}").unwrap(), MatchCriteria::default());
}
