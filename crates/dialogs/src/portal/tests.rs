use super::*;

fn filters(list: &[(&str, &[&str])]) -> Vec<Filter> {
    list.iter().map(|(name, exts)| Filter::new(*name, exts)).collect()
}

fn photo_filters() -> Vec<Filter> {
    filters(&[("PSD Document", &["psd", "psb"]), ("PNG", &["png"]), ("JPEG", &["jpg"])])
}

/// The file type picked in the save dialog gives the name its extension where the dialog left the
/// one it opened with, and only there.
// pc: apps/photocraft/src/services/tests.rs:32 the_picked_file_type_gives_the_name_its_extension
#[test]
fn the_picked_file_type_gives_the_name_its_extension() {
    let exts = |e: &[&str]| e.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let (png, psd) = (exts(&["png"]), exts(&["psd", "psb"]));
    let mut all = photo_filters();
    all.push(Filter::new("Project", &["pcraft"]));
    let opened = Some("Untitled-1.psd");
    let cases = [
        // PNG picked, the name as the dialog opened (or renamed, its extension kept).
        ("/pics/Untitled-1.psd", opened, &png, "/pics/Untitled-1.png"),
        ("/pics/art.psd", opened, &png, "/pics/art.png"),
        ("/pics/ART.PSD", opened, &png, "/pics/ART.png"),
        // No extension: the type's. One no filter lists is part of the name.
        ("/pics/art", opened, &png, "/pics/art.png"),
        ("/pics/scan.2024", opened, &png, "/pics/scan.2024.png"),
        // Typed, an extension a filter lists is the user's: kept, whatever the type says.
        ("/pics/art.png", opened, &psd, "/pics/art.png"),
        ("/pics/art.jpg", opened, &png, "/pics/art.jpg"),
        ("/pics/art.pcraft", opened, &png, "/pics/art.pcraft"),
        // Already the type's: kept, including its other extensions.
        ("/pics/art.png", opened, &png, "/pics/art.png"),
        ("/pics/art.PNG", opened, &png, "/pics/art.PNG"),
        ("/pics/big.psb", opened, &psd, "/pics/big.psb"),
        // The dialog opened on a .pcraft: that is the extension the type replaces.
        ("/pics/art.pcraft", Some("art.pcraft"), &png, "/pics/art.png"),
        ("/pics/art.psd", Some("art.pcraft"), &png, "/pics/art.psd"),
    ];
    for (typed, opened, chosen, want) in cases {
        assert_eq!(with_chosen_type(Path::new(typed), opened, chosen, &all), PathBuf::from(want), "{typed} (opened {opened:?})");
    }
    // No type picked: the name as typed.
    assert_eq!(with_chosen_type(Path::new("/pics/art.psd"), opened, &[], &all), PathBuf::from("/pics/art.psd"));
}

#[test]
fn file_uris_become_local_paths() {
    assert_eq!(file_uri_path("file:///home/me/Untitled-1.psd"), Some(PathBuf::from("/home/me/Untitled-1.psd")));
    assert_eq!(file_uri_path("file:///home/me/A%20b%C3%A9.png"), Some(PathBuf::from("/home/me/A bé.png")));
    assert_eq!(file_uri_path("file://localhost/tmp/x.png"), Some(PathBuf::from("/tmp/x.png")));
    assert_eq!(file_uri_path("sftp://host/x.png"), None);
    assert_eq!(file_uri_path("file://host/x.png"), None);
}

/// The request starts on the type that lists the suggested extension, shows the stem, and keeps
/// the suggested name for the extension rule.
#[test]
fn the_request_starts_on_the_suggested_type() {
    let mut dialog = FileDialog::save().file_name("test.PNG").directory("/pics");
    dialog.filters = photo_filters();
    let req = SaveRequest::new(&dialog, "x11:1".into());
    assert_eq!((req.current, req.suggested.as_deref(), req.folder.as_deref()), (Some(1), Some("test.PNG"), Some(Path::new("/pics"))));
    assert_eq!(req.parent, "x11:1");
    // No listed type has the extension, or the name has none: the first type.
    assert_eq!(SaveRequest::new(&FileDialog { file_name: Some("a.xyz".into()), ..dialog.clone() }, String::new()).current, Some(0));
    assert_eq!(SaveRequest::new(&FileDialog { file_name: None, ..dialog.clone() }, String::new()).current, Some(0));
    // No types: none to start on.
    assert_eq!(SaveRequest::new(&FileDialog::save().file_name("a.png"), String::new()).current, None);
}

/// The `SaveFile` options carry the name, the folder as a NUL-terminated byte string, every type
/// as case-insensitive globs, and the type the dialog starts on.
#[test]
fn the_save_options_carry_the_request() {
    let req = SaveRequest {
        name: Some("test".into()),
        folder: Some(PathBuf::from("/pics")),
        filters: filters(&[("PNG", &["png"])]),
        current: Some(0),
        ..SaveRequest::default()
    };
    let o = options(&req, "tok");
    let text = |key: &str| o.get(key).and_then(|v| <&str>::try_from(v).ok());
    assert_eq!((text("handle_token"), text("current_name")), (Some("tok"), Some("test")));
    assert_eq!(o.get("modal"), Some(&Value::from(true)));
    assert_eq!(o.get("current_folder"), Some(&Value::from(b"/pics\0".to_vec())));
    let png = (String::from("PNG"), vec![(0u32, "*.png".to_string()), (0, "*.PNG".to_string()), (0, "*.[pP][nN][gG]".to_string())]);
    assert_eq!(o.get("filters"), Some(&Value::from(vec![png.clone()])));
    assert_eq!(o.get("current_filter"), Some(&Value::from(png)));
    // Nothing optional is sent when the request has none of it.
    let request = SaveRequest::default();
    let bare = options(&request, "t");
    assert_eq!(bare.keys().copied().collect::<std::collections::BTreeSet<_>>(), ["handle_token", "modal"].into());
}

/// Cancel (1) and a dialog ended some other way (2) are no file, not a failure that shows rfd's
/// dialog; the picked file and type come from a successful answer.
#[test]
fn the_portal_response_is_read() {
    let none = HashMap::new();
    assert_eq!(answer(1, &none), Ok(None));
    assert_eq!(answer(2, &none), Ok(None));
    assert!(answer(3, &none).is_err());
    assert!(answer(0, &none).is_err());
    let owned = |v: Value<'_>| OwnedValue::try_from(v).unwrap();
    let results = HashMap::from([
        ("uris".to_string(), owned(Value::from(vec!["file:///tmp/a%20b.psd".to_string()]))),
        ("current_filter".to_string(), owned(Value::from(("PNG".to_string(), vec![(0u32, "*.png".to_string())])))),
    ]);
    assert_eq!(answer(0, &results), Ok(Some(Saved { path: PathBuf::from("/tmp/a b.psd"), filter: Some("PNG".into()) })));
}
