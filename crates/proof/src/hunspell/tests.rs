use super::*;

/// CodeRabbit on craft-libs#489: without LibreOffice extension names no drive is probed; with
/// them, only drives where LibreOffice is found give folders.
#[test]
fn libreoffice_drives_are_probed_only_when_wanted() {
    let probes = std::cell::Cell::new(0);
    let count = |_: &Path| {
        probes.set(probes.get() + 1);
        false
    };
    // craft-libs#495 review: the drives are probed once, not on every lookup.
    let cache = std::sync::OnceLock::new();
    let cached = || cache.get_or_init(|| lo_drive_bases(count)).as_slice();
    assert!(lo_drive_dirs(None, cached).is_empty());
    assert_eq!(probes.get(), 0);
    let exts = ["dict-tr".to_string()];
    assert!(lo_drive_dirs(Some(&exts), cached).is_empty());
    assert!(lo_drive_dirs(Some(&exts), cached).is_empty());
    assert_eq!(probes.get(), 48);
    let found = lo_drive_bases(|p: &Path| p.to_string_lossy().starts_with(r"D:\Program Files\"));
    assert_eq!(lo_drive_dirs(Some(&exts), || found.as_slice()), [PathBuf::from(r"D:\Program Files\LibreOffice\share\extensions").join("dict-tr")]);
}

/// Another region's Language pack never beats the system's dictionary of the right region
/// (`pt-pt` pack vs system `pt_BR`; `de-ch` vs `de_DE`; `en-gb` vs `en_US`), but beats the
/// system's other names; the right region's pack beats the system; the dictionaries folder
/// beats everything.
#[test]
fn another_regions_pack_comes_after_the_systems_exact_region() {
    let root = std::env::temp_dir().join(format!("wc-proof-order-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let pair = |dir: &Path, name: &str| {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.aff")), "SET UTF-8\n").unwrap();
        std::fs::write(dir.join(format!("{name}.dic")), "1\nx\n").unwrap();
        dir.join(format!("{name}.dic"))
    };
    let (folder, system) = (root.join("folder"), root.join("system"));
    let en: &[&str] = &["en_US", "en-US", "en", "en_CA", "en_GB", "en-GB", "en_AU"];
    for (names, exact, other, cross) in [
        (&["pt_BR", "pt-BR", "pt"][..], "pt_BR", "pt", "pt-pt"),
        (&["de_DE", "de-DE", "de"][..], "de_DE", "de", "de-ch"),
        (&["nl_NL", "nl-NL", "nl"][..], "nl_NL", "nl", "nl-be"),
        (&["es_ES", "es-ES", "es_ANY", "es"][..], "es_ES", "es", "es-mx"),
        (en, "en_US", "en_GB", "en-gb"),
    ] {
        let sys_exact = pair(&system, exact);
        let sys_other = pair(&system, other);
        let cross_pack = pair(&root.join("packs").join(cross), crate::packs::SPELLING);
        let dics = |v: Vec<(PathBuf, PathBuf)>| v.into_iter().map(|(_, d)| d).collect::<Vec<_>>();
        let got = dics(ordered(names, &[], &[], std::slice::from_ref(&system), std::slice::from_ref(&cross_pack.parent().unwrap().to_path_buf())));
        assert_eq!(got, [sys_exact.clone(), cross_pack.clone(), sys_other.clone()], "{names:?}");
        // The region's own pack first, then the system's exact region.
        let region_pack = pair(&root.join("packs").join(format!("{}-region", names[0])), crate::packs::SPELLING);
        let got = dics(ordered(names, &[], &[region_pack.parent().unwrap().to_path_buf()], std::slice::from_ref(&system), &[]));
        assert_eq!(got.first(), Some(&region_pack), "{names:?}");
        assert_eq!(got.get(1), Some(&sys_exact), "{names:?}");
        // The dictionaries folder before all of them, whatever name.
        let mine = pair(&folder, other);
        let got =
            dics(ordered(names, std::slice::from_ref(&folder), &[region_pack.parent().unwrap().to_path_buf()], std::slice::from_ref(&system), &[]));
        assert_eq!(got.first(), Some(&mine), "{names:?}");
        assert_eq!(find_in(&folder, &[other]).map(|(_, d)| d), Some(mine));
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Review follow-up (craft-libs#485): a pack's `spelling.aff`/`.dic` that is a symbolic link
/// (swapped in after the pack scan) is never a candidate; one in a system folder still is.
#[cfg(unix)]
#[test]
fn pack_files_are_never_read_through_a_link() {
    let root = std::env::temp_dir().join(format!("wc-proof-pack-links-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (pack, system, elsewhere) = (root.join("packs").join("nl"), root.join("system"), root.join("elsewhere"));
    for d in [&pack, &system, &elsewhere] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(elsewhere.join("x.aff"), "SET UTF-8\n").unwrap();
    std::fs::write(elsewhere.join("x.dic"), "1\nx\n").unwrap();
    std::fs::write(pack.join("spelling.aff"), "SET UTF-8\n").unwrap();
    std::os::unix::fs::symlink(elsewhere.join("x.dic"), pack.join("spelling.dic")).unwrap();
    std::os::unix::fs::symlink(elsewhere.join("x.aff"), system.join("nl.aff")).unwrap();
    std::os::unix::fs::symlink(elsewhere.join("x.dic"), system.join("nl.dic")).unwrap();
    let got: Vec<PathBuf> =
        ordered(&["nl"], &[], std::slice::from_ref(&pack), std::slice::from_ref(&system), &[]).into_iter().map(|(_, d)| d).collect();
    assert_eq!(got, [system.join("nl.dic")], "the system's link is followed, the pack's never");
    std::fs::remove_file(pack.join("spelling.dic")).unwrap();
    std::fs::write(pack.join("spelling.dic"), "1\nx\n").unwrap();
    let got: Vec<PathBuf> =
        ordered(&["nl"], &[], std::slice::from_ref(&pack), std::slice::from_ref(&system), &[]).into_iter().map(|(_, d)| d).collect();
    assert_eq!(got, [pack.join("spelling.dic"), system.join("nl.dic")]);
    let _ = std::fs::remove_dir_all(&root);
}

/// WordCraft's own dictionaries folder under the user's application data is searched on every
/// desktop system, after the setting and before the shared system locations (#1063).
#[test]
fn the_app_folder_comes_before_the_system_locations() {
    if cfg!(target_arch = "wasm32") {
        return;
    }
    let user = if cfg!(windows) { "LOCALAPPDATA" } else { "HOME" };
    if std::env::var_os(user).filter(|v| !v.is_empty()).is_none() {
        return; // no user folder to derive it from
    }
    let dirs = search_dirs(None);
    let at = |suffix: &str| dirs.iter().position(|d| d.ends_with(suffix));
    let (app, shared) = if cfg!(windows) {
        ("WordCraft/dictionaries", "hunspell")
    } else if cfg!(target_os = "macos") {
        ("Application Support/WordCraft/dictionaries", "/Library/Spelling")
    } else {
        ("wordcraft/dictionaries", "/usr/share/hunspell")
    };
    assert!(matches!((at(app), at(shared)), (Some(app), Some(shared)) if app < shared), "{dirs:?}");
}

/// `$XDG_DATA_HOME` counts only as an absolute path; otherwise (unset, empty or relative) the
/// spec's default `~/.local/share` applies, and with no home there is no folder.
#[test]
fn xdg_data_home_follows_the_spec() {
    let home = Path::new("/home/a");
    assert_eq!(xdg_data_home(Some("/data".into()), Some(home)), Some(PathBuf::from("/data")));
    assert_eq!(xdg_data_home(Some("/data".into()), None), Some(PathBuf::from("/data")));
    for xdg in [None, Some(""), Some("relative/data")] {
        assert_eq!(xdg_data_home(xdg.map(Into::into), Some(home)), Some(home.join(".local/share")), "{xdg:?}");
        assert_eq!(xdg_data_home(xdg.map(Into::into), None), None, "{xdg:?}");
    }
}

#[test]
fn decodes_latin2_and_utf8() {
    let aff = b"SET ISO8859-2\nTRY a\n";
    let (a, d) = decode(aff, b"2\n\xbf\xf3\xb3w\n").unwrap();
    assert!(a.contains("TRY a"));
    assert_eq!(d, "2\nżółw\n");
    let (_, d) = decode("SET UTF-8\n".as_bytes(), "żółw".as_bytes()).unwrap();
    assert_eq!(d, "żółw");
    let (_, d) = decode(b"SET ISO8859-9\n", b"\xd0\xdd\xde\xf0\xfd\xfe\xd6\xe7").unwrap();
    assert_eq!(d, "ĞİŞğışÖç");
    assert!(decode(b"SET SHIFT_JIS\n", b"").is_err());
    assert!(decode(b"SET UTF-8\n", b"\xff\xfe").is_err());
}

/// #2830: a dictionary that is there but can't be read leaves its reason behind; a good
/// load or no file clears it.
#[test]
fn load_error_is_remembered_until_the_next_good_load() {
    let key = "test_unreadable_2830";
    assert_eq!(load_error(key), None);
    let dic = Path::new("/dicts/test.dic");
    assert!(note_load(key, dic, decode(b"SET SHIFT_JIS\n", b"")).is_err());
    let (file, reason) = load_error(key).expect("error kept");
    assert_eq!(file, dic);
    assert_eq!(reason, "unsupported encoding SHIFT_JIS");
    assert!(note_load(key, dic, decode(b"SET UTF-8\n", b"")).is_ok());
    assert_eq!(load_error(key), None);
    let _ = note_load::<()>(key, dic, Err("too large".into()));
    clear_load_error(key);
    assert_eq!(load_error(key), None);
}

/// Every table covers its 96 (ISO 8859) or 128 (KOI8-R, Windows-1251) high bytes.
#[test]
fn tables_are_complete() {
    for t in [ISO_8859_2_HIGH, ISO_8859_7_HIGH, ISO_8859_15_HIGH] {
        assert_eq!(t.chars().count(), 96, "{t}");
    }
    for t in [KOI8_R_HIGH, CP1251_HIGH] {
        assert_eq!(t.chars().count(), 128, "{t}");
    }
}

#[test]
fn decodes_greek_and_latin9() {
    // `σπίτι νερό` in ISO 8859-7, and a flag after the slash stays ASCII.
    let (a, d) = decode(b"SET ISO8859-7\nTRY \xe1\n", b"2\n\xf3\xf0\xdf\xf4\xe9/A\n\xed\xe5\xf1\xfc\n").unwrap();
    assert!(a.contains("TRY α"), "{a}");
    assert_eq!(d, "2\nσπίτι/A\nνερό\n");
    // Unassigned bytes and the 2003 additions.
    let (_, d) = decode(b"SET ISO-8859-7\n", b"\xae\xd2\xff\xa4\xa5\xaa\xa2").unwrap();
    assert_eq!(d, "\u{fffd}\u{fffd}\u{fffd}€₯ͺ’");
    // `šokolaad õun` in ISO 8859-15, and its eight differences from ISO 8859-1.
    let (_, d) = decode(b"SET ISO8859-15\n", b"\xa8okolaad \xf5un").unwrap();
    assert_eq!(d, "šokolaad õun");
    let (_, d) = decode(b"SET ISO-8859-15\n", b"\xa4\xa6\xa8\xb4\xb8\xbc\xbd\xbe\xe4").unwrap();
    assert_eq!(d, "€ŠšŽžŒœŸä");
}

#[test]
fn decodes_latin1() {
    // `orçamento esperávamos` in ISO 8859-1: ç = 0xE7, á = 0xE1.
    let (a, d) = decode(b"SET ISO8859-1\nTRY \xe7\n", b"1\nor\xe7amento/A\nesper\xe1vamos\n").unwrap();
    assert!(a.contains("TRY ç"), "{a}");
    assert_eq!(d, "1\norçamento/A\nesperávamos\n");
    assert!(decode(b"SET ISO-8859-1\n", b"\xe7").is_ok());
}

#[test]
fn utf8_dictionaries_keep_single_byte_flags() {
    // The Hungarian dictionary is `SET UTF-8` but writes flags as single bytes above 0x7F,
    // which aren't UTF-8; each byte is one flag. Its `NAME` line is ISO 8859-2.
    let aff = b"SET UTF-8\nNAME Magyar helyes\xedr\xe1si\nAF 2\nAF \xd5a # 1\nAF \xe9 # 2\nSFX \xd5 Y 1\nSFX \xd5 0 ok/2 . 7\nSFX \xe9 Y 1\nSFX \xe9 0 ban .\nSFX a Y 1\nSFX a 0 at .\n";
    let dic = "2\nház/1\t1\nkert/\u{e9}\n".as_bytes();
    let (a, d) = decode(aff, dic).unwrap();
    assert!(a.contains("AF \u{d5}a # 1") && a.contains("AF \u{e9} # 2") && a.contains("SFX \u{d5} 0 ok/2 . 7"));
    assert!(d.contains("kert/\u{c3}\u{a9}"), "a UTF-8 é in the flags is two one-byte flags");
    let dict = crate::affix::AffixDict::from_hunspell(&a, &d, &[]).unwrap();
    assert!(dict.check("ház") && dict.check("házok") && dict.check("házokban") && dict.check("házat"));
    assert!(!dict.check("házban"));
    // `FLAG UTF-8` keeps the whole file UTF-8, and words must still be UTF-8.
    let (_, d) = decode(b"SET UTF-8\nFLAG UTF-8\n", "kert/\u{e9}\n".as_bytes()).unwrap();
    assert_eq!(d, "kert/\u{e9}\n");
    assert!(decode(b"SET UTF-8\nSFX a Y 1\nSFX a 0 \xe9 .\n", b"").is_err(), "an affix must be UTF-8");
    assert!(decode(b"SET UTF-8\n", b"1\nh\xe1z/a\n").is_err(), "a stem must be UTF-8");
}

#[test]
fn single_flag_fields_keep_their_first_byte() {
    // Without `FLAG UTF-8`, a UTF-8 `é` (0xC3 0xA9) in a field that names one flag is the
    // flag 0xC3; in a `.dic` flag list it is the two flags 0xC3 and 0xA9.
    let aff = "SET UTF-8\nKEEPCASE é\nSFX é Y 1\nSFX é 0 ok .\nSYLLABLENUM é\n".as_bytes();
    let (a, d) = decode(aff, "2\nkert/é\nNATO/é\n".as_bytes()).unwrap();
    assert!(a.contains("KEEPCASE \u{c3}\n") && a.contains("SFX \u{c3} Y 1") && a.contains("SFX \u{c3} 0 ok ."), "{a}");
    assert!(a.contains("SYLLABLENUM \u{c3}\u{a9}"), "SYLLABLENUM lists flags: {a}");
    assert!(d.contains("kert/\u{c3}\u{a9}"));
    let dict = crate::affix::AffixDict::from_hunspell(&a, &d, &[]).unwrap();
    assert!(dict.check("kert") && dict.check("kertok"));
    assert!(dict.check("NATO") && !dict.check("Nato"), "KEEPCASE é reaches NATO/é");
}

#[test]
fn numeric_and_long_flags_stay_text() {
    // `FLAG num` (the Turkish dictionary) and `FLAG long`: no byte-wise flag reading, so a
    // number with a non-ASCII rule or comment keeps every digit.
    let aff = "SET UTF-8\nFLAG num\nFORBIDDENWORD 6000 # tiltott ő\nSFX 1234 Y 1\nSFX 1234 0 ları .\n";
    let (a, d) = decode(aff.as_bytes(), "1\nkitap/1234\n".as_bytes()).unwrap();
    assert_eq!(a, aff);
    assert_eq!(d, "1\nkitap/1234\n");
    let tr = crate::tr::dict::TrDict::from_hunspell(&a, &d).unwrap();
    assert!(tr.contains("kitapları") && tr.contains("kitap"));
    let long = "SET UTF-8\nFLAG long\nSFX Aa Y 1\nSFX Aa 0 ős .\n";
    assert_eq!(decode(long.as_bytes(), "1\nkő/Aa\n".as_bytes()).unwrap().0, long);
    assert!(decode(b"SET UTF-8\nFLAG num\n", b"1\nh\xe1z/1\n").is_err(), "the .dic must be UTF-8");
    // `FLAG DEFAULT` is the single-byte default.
    let (_, d) = decode(b"SET UTF-8\nFLAG default\n", b"1\nkert/\xe9\n").unwrap();
    assert_eq!(d, "1\nkert/\u{e9}\n");
}

#[test]
fn flag_utf8_counts_anywhere_in_the_aff() {
    // A `FLAG UTF-8` after a long rule table still makes the `.dic` flags Unicode.
    let mut aff = b"SET UTF-8\nSFX a Y 400\n".to_vec();
    for _ in 0..400 {
        aff.extend_from_slice(b"SFX a 0 aaaaaaaaaaaaaaaaaaaaaaaa .\n");
    }
    aff.extend_from_slice(b"FLAG UTF-8\r\n");
    assert!(aff.len() > 8192);
    let (_, d) = decode(&aff, "kert/\u{e9}\n".as_bytes()).unwrap();
    assert_eq!(d, "kert/\u{e9}\n");
}

#[test]
fn a_comment_after_flag_utf8_keeps_unicode_flags() {
    let (_, d) = decode(b"SET UTF-8\nFLAG UTF-8 # Unicode flags\n", "kert/\u{e9}\n".as_bytes()).unwrap();
    assert_eq!(d, "kert/\u{e9}\n");
    let (_, d) = decode(b"SET\tUTF-8  # comment\nFLAG\t\tUTF-8\n", "kert/\u{e9}\n".as_bytes()).unwrap();
    assert_eq!(d, "kert/\u{e9}\n", "repeated spaces and tabs separate the argument");
}

#[test]
fn decodes_russian_single_byte_encodings() {
    // `ёлка` in KOI8-R (0xA3 0xCC 0xCB 0xC1) and in Windows-1251 (0xB8 0xEB 0xEA 0xE0).
    let (a, d) = decode(b"SET KOI8-R\nTRY \xcf\xc5\n", b"1\n\xa3\xcc\xcb\xc1/A\n").unwrap();
    assert!(a.contains("TRY ое"), "{a}");
    assert_eq!(d, "1\nёлка/A\n");
    let (_, d) = decode(b"SET microsoft-cp1251\n", b"1\n\xb8\xeb\xea\xe0 \xc4\xee\xec\n").unwrap();
    assert_eq!(d, "1\nёлка Дом\n");
    assert_eq!(decode_high(&[0x98], CP1251_HIGH), "\u{fffd}");
}

#[test]
fn comments_start_after_the_directive_fields() {
    let f = |line: &'static str| move |k: usize| line.split_whitespace().nth(k).map(str::to_string);
    assert_eq!(comment_field("PFX", f("PFX A Y 3 # count")), 4);
    assert_eq!(comment_field("SFX", f("SFX A 0 # .")), 5, "a rule's condition may be `#`");
    assert_eq!(comment_field("AF", f("AF 1234 # aliases")), 2);
    assert_eq!(comment_field("AF", f("AF Aa # 1")), 2);
    assert_eq!(comment_field("REP", f("REP 12")), 2);
    assert_eq!(comment_field("REP", f("REP # x")), 3);
    assert_eq!(comment_field("CHECKCOMPOUNDPATTERN", f("CHECKCOMPOUNDPATTERN a b c")), 4);
    assert_eq!(comment_field("KEEPCASE", f("KEEPCASE k")), 2);
    // A byte flag after an append text that begins with `#` stays a flag, not a comment.
    let (a, _) = decode(b"SET UTF-8\nSFX \xd5 Y 1 # header\nSFX \xd5 0 #/\xe9 . # rule\n", b"").unwrap();
    assert!(a.contains("SFX \u{d5} 0 #/\u{e9} . # rule"), "{a}");
}

#[test]
fn dic_entries_end_where_hunspell_ends_them() {
    let end = |l: &str| l.get(..dic_entry_end(l.as_bytes())).unwrap().to_string();
    assert_eq!(end("word/A po:noun\tts:x"), "word/A", "the morphological field comes before the tab");
    assert_eq!(end("ad hoc/B\tpo:adv"), "ad hoc/B", "a tab ends a phrase");
    assert_eq!(end("ad hoc/B po:adv"), "ad hoc/B");
    assert_eq!(end("word/A 123"), "word/A 123", "neither: the whole line, as in Hunspell");
    assert_eq!(end("ad hoc/B  "), "ad hoc/B");
    assert_eq!(end("word"), "word");
    assert_eq!(end("a:b/A"), "a:b/A", "a colon inside the word isn't a field");
    // Single-byte flags of a phrase entry are read byte by byte, then the UTF-8 morphology.
    let (_, d) = decode(b"SET UTF-8\n", b"1\nad hoc/\xe9\tpo:adv\n").unwrap();
    assert_eq!(d, "1\nad hoc/\u{e9}\tpo:adv\n");
    // Without a tab or a morphological field the whole line is the entry: its byte flags
    // don't fail the load (the Hungarian dictionary has such lines: `tök jó/396`).
    let (_, d) = decode(b"SET UTF-8\n", b"1\nad hoc/\xe9\n").unwrap();
    assert_eq!(d, "1\nad hoc/\u{e9}\n");
}

#[test]
fn libreoffice_extension_dirs_follow_the_language() {
    // LibreOffice's IDs are per dictionary: Polish is `dict-pl`, Brazilian Portuguese
    // `dict-pt-BR`, so both the primary-language folder and the dashed full name are tried.
    assert_eq!(super::lo_ext_dirs("cs_CZ"), ["dict-cs", "dict-cs-CZ"]);
    assert_eq!(super::lo_ext_dirs("id_ID"), ["dict-id", "dict-id-ID"]);
    assert_eq!(super::lo_ext_dirs("pl_PL"), ["dict-pl", "dict-pl-PL"]);
    assert_eq!(super::lo_ext_dirs("pt_BR"), ["dict-pt", "dict-pt-BR"]);
    assert_eq!(super::lo_ext_dirs("de-DE"), ["dict-de", "dict-de-DE"]);
    let dirs = search_dirs(Some(&super::lo_ext_dirs("pt_BR")));
    assert!(dirs.iter().any(|d| d.ends_with("dict-pt-BR")), "{dirs:?}");
    assert!(search_dirs(None).iter().all(|d| !d.to_string_lossy().contains("dict-")), "no language: no LibreOffice guess");
}

/// craft-libs#482 re-review: a name is looked for in every folder before the next name, so
/// an `en_US` in the dictionaries folder doesn't beat a system `en_GB` for British English.
#[test]
fn candidates_go_name_by_name_across_the_folders() {
    let base = std::env::temp_dir().join(format!("wordcraft-candidates-{}", std::process::id()));
    let (folder, system) = (base.join("folder"), base.join("system"));
    for (dir, name) in [(&folder, "en_US"), (&system, "en_GB"), (&folder, "en-GB")] {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.aff")), "SET UTF-8\n").unwrap();
        std::fs::write(dir.join(format!("{name}.dic")), "1\nx\n").unwrap();
    }
    let dirs = [folder.clone(), system.clone()];
    let dics: Vec<PathBuf> = candidates_in(&dirs, &["en_GB", "en-GB"]).into_iter().map(|(_, d)| d).collect();
    assert_eq!(dics, vec![system.join("en_GB.dic"), folder.join("en-GB.dic")]);
    let us: Vec<PathBuf> = candidates_in(&dirs, &["en_US", "en_GB"]).into_iter().map(|(_, d)| d).collect();
    assert_eq!(us, vec![folder.join("en_US.dic"), system.join("en_GB.dic")]);
    assert!(candidates_in(&dirs, &["en_CA"]).is_empty());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn finds_and_loads_a_pair_in_a_directory() {
    let dir = std::env::temp_dir().join(format!("wordcraft-hunspell-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(find_in(&dir, &["xx_XX"]).is_none(), "no files: nothing found");
    std::fs::write(dir.join("xx_XX.aff"), "SET UTF-8\nSFX A Y 1\nSFX A 0 y .\n").unwrap();
    assert!(find_in(&dir, &["xx_XX"]).is_none(), "an .aff alone isn't a dictionary");
    std::fs::write(dir.join("xx_XX.dic"), "1\nkot/A\n").unwrap();
    let (aff, dic) = find_in(&dir, &["yy", "xx_XX"]).unwrap();
    let d = load(&aff, &dic).unwrap();
    assert!(d.check("kot") && d.check("koty") && !d.check("kotx"));
    assert!(load(&dir.join("missing.aff"), &dic).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reads_are_bounded_and_regular_files_only() {
    let dir = std::env::temp_dir().join(format!("wordcraft-hunspell-cap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let aff = dir.join("xx.aff");
    std::fs::write(&aff, "SET UTF-8\n").unwrap();
    // One byte over the cap (sparse: nothing is written) is refused without reading it.
    let big = dir.join("big.dic");
    std::fs::File::create(&big).unwrap().set_len(MAX_FILE + 1).unwrap();
    assert!(load(&aff, &big).unwrap_err().contains("too large"));
    // A directory, and on Unix an endless device, are not dictionary files. (No clock: a
    // reader that read `/dev/zero` would never return, so the error itself is the proof.)
    assert!(load(&aff, &dir).unwrap_err().contains("not a regular file"));
    #[cfg(unix)]
    assert!(load(&aff, Path::new("/dev/zero")).unwrap_err().contains("not a regular file"));
    // Turkish reads through the same reader.
    let (tr_aff, tr_dic) = (dir.join("tr_TR.aff"), dir.join("tr_TR.dic"));
    std::fs::write(&tr_aff, "SET UTF-8\nSFX 1 Y 1\nSFX 1 0 lar .\n").unwrap();
    std::fs::write(&tr_dic, "1\nkitap/1\n").unwrap();
    assert!(crate::tr::load(&tr_aff, &tr_dic).unwrap().contains("kitaplar"));
    assert!(crate::tr::load(&tr_aff, &big).unwrap_err().contains("too large"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A Turkish dictionary in ISO 8859-9, the encoding older tr_TR dictionaries use, read from
/// disk through the capped reader and decoded per its `SET` line: its words with Turkish
/// letters (ğ ı ş İ, and ç ö ü from Latin-1) are found, as stems and with a suffix. Before the
/// Turkish dictionary went through this reader it was read as UTF-8, which these bytes are not.
#[test]
fn a_turkish_dictionary_in_iso_8859_9_loads_from_disk() {
    let dir = std::env::temp_dir().join(format!("wordcraft-hunspell-latin5-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (aff, dic) = (dir.join("tr_TR.aff"), dir.join("tr_TR.dic"));
    std::fs::write(&aff, b"SET ISO8859-9\nSFX 1 Y 2\nSFX 1 0 lar .\nSFX 1 0 \xfd .\nSFX 2 Y 1\nSFX 2 0 ler .\n").unwrap();
    // ağaç/1 (-lar, -ı), ışık, şişe/2 (-ler), İzmir, öğrenci/2, güneş.
    let words: &[u8] = b"6\na\xf0a\xe7/1\n\xfd\xfe\xfdk\n\xfei\xfee/2\n\xddzmir\n\xf6\xf0renci/2\ng\xfcne\xfe\n";
    std::fs::write(&dic, words).unwrap();
    assert!(String::from_utf8(words.to_vec()).is_err(), "the .dic is not UTF-8");
    let d = crate::tr::load(&aff, &dic).unwrap();
    for w in ["ağaç", "ağaçlar", "ağaçı", "ışık", "şişe", "şişeler", "İzmir", "öğrenci", "öğrenciler", "güneş"] {
        assert!(d.contains(w), "{w}");
    }
    // The six letters ISO 8859-9 puts in place of ISO 8859-1's aren't read as those.
    assert!(!d.contains("aðaç") && !d.contains("ýþýk") && !d.contains("Ýzmir"));
    assert!(!d.contains("ağaçs") && !d.contains("ışıklar") && !d.contains("şişelar"), "only the rules' suffixes, on the stems that take them");
    let _ = std::fs::remove_dir_all(&dir);
}
