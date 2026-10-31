//! Reopen last session.

use crate::session::*;
use serde_json::json;

fn doc(path: &str) -> SessionDocument {
    SessionDocument { path: path.to_string(), ..Default::default() }
}

#[test]
// from_json skips entries without a path, defaults a missing view to Null and keeps at most MAX_DOCUMENTS.
fn stored_session_reads_leniently_and_is_capped() {
    let value = json!({
        "documents": [{"path": 5}, {"path": ""}, "x", {"path": "/a", "window": "w2", "view": {"page": 3}}, {"path": "/b"}],
        "active": "/b",
    });
    let session = Session::from_json(&value);
    assert_eq!(session.documents.len(), 2);
    assert_eq!(session.documents[0].window.as_deref(), Some("w2"));
    assert_eq!(session.documents[0].view, json!({"page": 3}));
    assert_eq!(session.documents[1].view, serde_json::Value::Null);
    assert_eq!(session.active.as_deref(), Some("/b"));

    let many: Vec<_> = (0..1000).map(|i| json!({"path": format!("/{i}")})).collect();
    assert_eq!(Session::from_json(&json!({"documents": many})).documents.len(), MAX_DOCUMENTS);
    assert_eq!(Session::from_json(&json!("garbage")), Session::default());
}

#[test]
// to_reopen leaves out a deleted file, a path in skip (compared after absolute) and one is_open reports.
// Cargo tests run in the crate directory, so the relative skip entry resolves against it.
fn reopen_skips_missing_skipped_and_open_files() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let files: Vec<String> = ["Cargo.toml", "src/lib.rs", "src/session.rs"].iter().map(|f| format!("{dir}/{f}")).collect();
    let session = Session { documents: vec![doc(&files[0]), doc(&files[1]), doc(&files[2]), doc(&format!("{dir}/gone.pdf"))], active: None };
    let skip = vec!["Cargo.toml".to_string()];
    let open = files[1].clone();
    let paths: Vec<&str> = session.to_reopen(&skip, |p| p == open).iter().map(|d| d.path.as_str()).collect();
    assert_eq!(paths, [files[2].as_str()]);
}

#[test]
// After quit_started, closing documents doesn't change to_save(true, ..); quit_cancelled makes to_save return the open session again.
fn quit_keeps_the_session_from_its_start() {
    let two = Session { documents: vec![doc("/a"), doc("/b")], active: Some("/a".into()) };
    let one = Session { documents: vec![doc("/b")], active: None };
    let mut keeper = SessionKeeper::default();
    keeper.quit_started(|| two.clone());
    keeper.quit_started(|| one.clone());
    assert_eq!(keeper.to_save(true, one.clone()), two);
    assert_eq!(keeper.to_save(false, one.clone()), one);
    keeper.quit_cancelled();
    assert_eq!(keeper.to_save(true, one.clone()), one);
}
