//! `craft_platform::default_app`.

use std::path::PathBuf;

use crate::default_app::{application_dirs, path_from_arg, rejoin_split_paths};

// file:///a%20b.pdf and file://localhost/x decode to paths; http://, file://host/x, plain paths
// and non-UTF-8 escapes pass through; file:///C:/x.pdf is C:/x.pdf on Windows.
#[test]
fn file_uris_become_paths() {
    assert_eq!(path_from_arg("file:///a%20b.pdf".into()), "/a b.pdf");
    assert_eq!(path_from_arg("file://localhost/x".into()), "/x");
    assert_eq!(path_from_arg("file:///caf%C3%A9.pdf".into()), "/café.pdf");
    assert_eq!(path_from_arg("file:///100%.pdf".into()), "/100%.pdf");
    for same in ["http://example.com/a.pdf", "file://host/x", "/plain/path.pdf", "file:///bad%FF.pdf", "mailto:a@b.c"] {
        assert_eq!(path_from_arg(same.into()), same);
    }
    let drive = path_from_arg("file:///C:/x.pdf".into());
    assert_eq!(drive, if cfg!(windows) { "C:/x.pdf" } else { "/C:/x.pdf" });
}

// ["P:\Docs\01", "Contract.PDF", "other.pdf"] with "P:\Docs\01 Contract.PDF" existing becomes
// two paths; arguments that name files stay as given.
#[test]
fn unquoted_paths_with_spaces_rejoin() {
    let exists = |p: &str| matches!(p, "P:\\Docs\\01 Contract.PDF" | "other.pdf" | "a" | "b");
    let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(rejoin_split_paths(args(&["P:\\Docs\\01", "Contract.PDF", "other.pdf"]), exists), ["P:\\Docs\\01 Contract.PDF", "other.pdf"]);
    assert_eq!(rejoin_split_paths(args(&["a", "b"]), exists), ["a", "b"]);
    assert_eq!(rejoin_split_paths(args(&["missing", "other.pdf"]), exists), ["missing", "other.pdf"]);
}

// XDG_DATA_HOME and XDG_DATA_DIRS each give an applications folder; defaults apply when unset
// and relative entries are ignored.
#[cfg(unix)]
#[test]
fn application_dirs_follow_the_xdg_spec() {
    let p = PathBuf::from;
    assert_eq!(
        application_dirs(None, None, Some("/home/a")),
        [p("/home/a/.local/share/applications"), p("/usr/local/share/applications"), p("/usr/share/applications")]
    );
    assert_eq!(
        application_dirs(Some("/data"), Some("/opt/share::relative:/usr/share"), Some("/home/a")),
        [p("/data/applications"), p("/opt/share/applications"), p("/usr/share/applications")]
    );
    assert_eq!(application_dirs(Some(""), Some(""), None), [p("/usr/local/share/applications"), p("/usr/share/applications")]);
    assert_eq!(application_dirs(Some("rel"), None, Some("also-rel")), [p("/usr/local/share/applications"), p("/usr/share/applications")]);
}
