use serde_json::json;

use crate::*;

const COMMIT: &str = "45e3d5269e4a17d823f6d84470bc084dc5ae46f6";

/// A font with the tables a scan and [`is_font`] need: `name`, `OS/2` and an empty `glyf`.
fn font(family: &str, style: &str, weight: u16) -> Vec<u8> {
    use write_fonts::FontBuilder;
    use write_fonts::tables::name::{Name, NameRecord};
    use write_fonts::tables::os2::Os2;
    use write_fonts::types::{NameId, Tag};
    let ps = format!("{}-{}", family.replace(' ', ""), style.replace(' ', ""));
    let mut records: Vec<NameRecord> = [(1, family.to_string()), (2, style.to_string()), (4, format!("{family} {style}")), (6, ps)]
        .into_iter()
        .map(|(id, text)| NameRecord::new(3, 1, 0x409, NameId::new(id), text.into()))
        .collect();
    records.sort_by_key(|r| r.name_id);
    let mut builder = FontBuilder::new();
    builder.add_table(&Name::new(records)).unwrap();
    builder.add_table(&Os2 { us_weight_class: weight, ul_code_page_range_1: Some(1), ul_code_page_range_2: Some(0), ..Default::default() }).unwrap();
    builder.add_raw(Tag::new(b"glyf"), vec![0u8; 4]);
    builder.build()
}

/// An index and the files it lists, by their URL-encoded google/fonts paths.
struct Fixture {
    files: Vec<(String, Vec<u8>)>,
    index: serde_json::Value,
}

fn file_entry(path: &str, bytes: &[u8], style: &str, weight: u16, axes: serde_json::Value, ps: &[&str]) -> serde_json::Value {
    json!({"path": path, "size": bytes.len(), "sha256": sha256_hex(bytes), "style": style, "weight": weight, "axes": axes, "postscript": ps})
}

/// An index of four families, and the bytes of their files.
fn fixture() -> Fixture {
    let regular = font("Test Sans", "Regular", 400);
    let bold = font("Test Sans", "Bold", 700);
    let italic = font("Test Sans", "Italic", 400);
    let var = font("Test Var", "Regular", 400);
    let one = font("Shared One", "Regular", 400);
    let two = font("Shared Two", "Regular", 400);
    let index = json!({
        "schema": 1, "version": "1.0", "source": "google/fonts", "commit": COMMIT,
        "base_url": format!("https://cdn.jsdelivr.net/gh/google/fonts@{COMMIT}/"),
        "fallback_base_url": format!("https://raw.githubusercontent.com/google/fonts/{COMMIT}/"),
        "a_field_from_a_later_minor_version": true,
        "languages": [{"id": "en_Latn", "name": "English", "autonym": "English", "script": "Latn", "population": 1, "historical": false}],
        "scripts": [{"id": "Latn", "name": "Latin"}],
        "families": [
            {"family": "Shared One", "license": "OFL", "license_path": null, "languages": [0], "scripts": ["Latn"],
             "files": [file_entry("ofl/sharedone/SharedOne-Regular.ttf", &one, "normal", 400, json!([]), &["Shared-Regular"])]},
            {"family": "Shared Two", "license": "OFL", "license_path": null,
             "files": [file_entry("ofl/sharedtwo/SharedTwo-Regular.ttf", &two, "normal", 400, json!([]), &["Shared-Regular"])]},
            {"family": "Test Sans", "designer": "Someone", "categories": ["SANS_SERIF"], "license": "OFL", "license_path": "ofl/testsans/OFL.txt",
             "languages": [0], "scripts": ["Latn"], "files": [
                file_entry("ofl/testsans/TestSans-Bold.ttf", &bold, "normal", 700, json!([]), &["TestSans-Bold"]),
                file_entry("ofl/testsans/TestSans-Italic.ttf", &italic, "italic", 400, json!([]), &["TestSans-Italic"]),
                file_entry("ofl/testsans/TestSans-Regular.ttf", &regular, "normal", 400, json!([]), &["TestSans-Regular"])]},
            {"family": "Test Var", "license": "APACHE2", "license_path": "apache/testvar/LICENSE.txt", "files": [
                file_entry("apache/testvar/TestVar[wght].ttf", &var, "normal", 400, json!([{"tag": "wght", "min": 300.0, "max": 700.0}]),
                    &["TestVar-Light", "TestVar-Regular", "TestVar-SemiBold", "TestVar-Bold"])]},
        ]
    });
    let files = vec![
        ("ofl/sharedone/SharedOne-Regular.ttf".into(), one),
        ("ofl/sharedtwo/SharedTwo-Regular.ttf".into(), two),
        ("ofl/testsans/TestSans-Bold.ttf".into(), bold),
        ("ofl/testsans/TestSans-Italic.ttf".into(), italic),
        ("ofl/testsans/TestSans-Regular.ttf".into(), regular),
        ("ofl/testsans/OFL.txt".into(), b"SIL Open Font License".to_vec()),
        ("apache/testvar/TestVar%5Bwght%5D.ttf".into(), var),
        ("apache/testvar/LICENSE.txt".into(), b"Apache License".to_vec()),
    ];
    Fixture { files, index }
}

fn index_of(v: &serde_json::Value) -> Result<Index, IndexError> {
    Index::from_json(v.to_string().as_bytes())
}

#[test]
fn the_index_is_read_and_checked() {
    let f = fixture();
    let index = index_of(&f.index).unwrap();
    assert_eq!((index.version(), index.commit(), index.families().len()), ("1.0", COMMIT, 4));
    let sans = index.family("test sans").unwrap();
    assert_eq!((sans.dir(), sans.license_file_name(), sans.designer.as_str()), ("testsans", Some("OFL.txt"), "Someone"));
    assert_eq!(sans.languages(&index).map(|l| l.id.as_str()).collect::<Vec<_>>(), ["en_Latn"]);

    let with = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut v = f.index.clone();
        edit(&mut v);
        index_of(&v)
    };
    assert!(with(&|v| v["version"] = json!("1.7")).is_ok(), "a later minor version only adds fields");
    assert!(matches!(with(&|v| v["version"] = json!("2.0")), Err(IndexError::Version(_))));
    assert!(matches!(with(&|v| v["version"] = json!("1")), Err(IndexError::Version(_))));
    assert!(matches!(
        with(&|v| {
            v.as_object_mut().unwrap().remove("version");
        }),
        Err(IndexError::Version(_))
    ));
    // A version 2 index may change any other field: the version is checked first.
    assert!(matches!(
        with(&|v| {
            v["version"] = json!("2.0");
            v["families"] = json!("other");
        }),
        Err(IndexError::Version(_))
    ));
    let invalid = [
        with(&|v| v["base_url"] = json!("http://cdn.jsdelivr.net/gh/google/fonts/")),
        with(&|v| v["commit"] = json!("main")),
        with(&|v| v["families"][2]["files"][0]["path"] = json!("ofl/../TestSans-Bold.ttf")),
        with(&|v| v["families"][2]["files"][0]["path"] = json!("ofl/testsans/.hidden.ttf")),
        with(&|v| v["families"][2]["files"][0]["path"] = json!("ofl/other/TestSans-Bold.ttf")),
        with(&|v| v["families"][2]["files"][0]["sha256"] = json!("00")),
        with(&|v| v["families"][2]["files"][0]["weight"] = json!(0)),
        with(&|v| v["families"][2]["license_path"] = json!("ofl/elsewhere/OFL.txt")),
        with(&|v| v["families"][2]["languages"] = json!([5])),
        with(&|v| v["families"][3]["family"] = json!("TEST SANS")),
    ];
    for (i, r) in invalid.into_iter().enumerate() {
        assert!(matches!(r, Err(IndexError::Invalid(_))), "case {i}: {r:?}");
    }
    assert!(matches!(Index::from_json(b"{"), Err(IndexError::Json(_))));
}

#[cfg(feature = "bundled-index")]
#[test]
fn the_bundled_index_is_the_pinned_one() {
    use std::io::Read;
    let gz = include_bytes!("../index/google-fonts-index.json.gz");
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(&gz[..]).read_to_end(&mut json).unwrap();
    assert_eq!(sha256_hex(&json), PINNED_INDEX.sha256);
    let index = Index::bundled().unwrap();
    assert!(index.families().len() > 1000);
    let face = index.find_face("Lato", "Bold Italic").unwrap();
    assert_eq!(face.file.file_name(), "Lato-BoldItalic.ttf");
}

#[test]
fn lookups_match_exactly_and_refuse_near_matches() {
    let index = index_of(&fixture().index).unwrap();
    let name = |family: &str, style: &str| index.find_face(family, style).map(|f| f.file.file_name().to_string());
    assert_eq!(name("Test Sans", "Bold").unwrap(), "TestSans-Bold.ttf");
    assert_eq!(name("test sans", "700").unwrap(), "TestSans-Bold.ttf");
    assert_eq!(name("Test Sans", "Regular").unwrap(), "TestSans-Regular.ttf");
    assert_eq!(name("Test Sans", "").unwrap(), "TestSans-Regular.ttf");
    assert_eq!(name("Test Sans", "Italic").unwrap(), "TestSans-Italic.ttf");
    // Near matches are not offered.
    for style in ["Medium", "Bold Italic", "Light", "Condensed"] {
        assert!(matches!(name("Test Sans", style), Err(Unavailable::NoStyle { .. })), "{style}");
    }
    assert!(matches!(name("Test San", "Bold"), Err(Unavailable::NoFamily(_))));
    assert!(matches!(name("TestSans", "Bold"), Err(Unavailable::NoFamily(_))));
    // A variable font has the weights its axis reaches, and no others.
    assert_eq!(name("Test Var", "SemiBold").unwrap(), "TestVar[wght].ttf");
    assert_eq!(name("Test Var", "Light").unwrap(), "TestVar[wght].ttf");
    assert!(matches!(name("Test Var", "Thin"), Err(Unavailable::NoStyle { .. })));
    assert!(matches!(name("Test Var", "Bold Italic"), Err(Unavailable::NoStyle { .. })));

    assert_eq!(index.find_postscript("TestSans-Bold").unwrap().family.family, "Test Sans");
    assert_eq!(index.find_postscript("TestVar-SemiBold").unwrap().file.file_name(), "TestVar[wght].ttf");
    assert!(matches!(index.find_postscript("testsans-bold"), Err(Unavailable::NoPostScript(_))));
    assert!(matches!(index.find_postscript("TestSans-Medium"), Err(Unavailable::NoPostScript(_))));
    let Err(Unavailable::Ambiguous { families, .. }) = index.find_postscript("Shared-Regular") else { panic!("not ambiguous") };
    assert_eq!(families, ["Shared One", "Shared Two"]);
    assert!(index.family("Shared Two").unwrap().face_by_postscript("Shared-Regular").is_some());

    assert_eq!(parse_style("Extra Bold Italic"), Some((800, true)));
    assert_eq!(parse_style("semi-bold"), Some((600, false)));
    assert_eq!(parse_style("Condensed Bold"), None);
    assert_eq!(
        index.urls("apache/testvar/TestVar[wght].ttf"),
        [
            format!("https://cdn.jsdelivr.net/gh/google/fonts@{COMMIT}/apache/testvar/TestVar%5Bwght%5D.ttf"),
            format!("https://raw.githubusercontent.com/google/fonts/{COMMIT}/apache/testvar/TestVar%5Bwght%5D.ttf"),
        ]
    );
}

#[test]
fn verification_checks_size_hash_and_font() {
    let index = index_of(&fixture().index).unwrap();
    let file = index.find_face("Test Sans", "Bold").unwrap().file;
    let bytes = font("Test Sans", "Bold", 700);
    assert_eq!(verify_font(file, &bytes), Ok(()));
    let mut changed = bytes.clone();
    *changed.last_mut().unwrap() ^= 1;
    assert_eq!(verify_font(file, &changed), Err(VerifyError::Sha256));
    assert!(matches!(verify_font(file, &bytes[1..]), Err(VerifyError::Size { .. })));
    assert!(!is_font(b"not a font at all"));
    assert!(is_font(&bytes));
}

#[cfg(not(target_arch = "wasm32"))]
mod store {
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use craft_font_catalog::{Catalog, FontEnv, Os};

    use super::*;

    /// What the server answers for a path.
    #[derive(Clone)]
    enum Reply {
        Ok(Vec<u8>),
        Status(u16),
        /// Sends headers and a few bytes, then nothing.
        Stall,
    }

    /// A local HTTP server for `/cdn/` (jsDelivr) and `/raw/` (raw GitHub).
    struct Server {
        base: String,
        routes: Arc<Mutex<HashMap<String, Reply>>>,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Server {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}/", listener.local_addr().unwrap());
            let routes: Arc<Mutex<HashMap<String, Reply>>> = Arc::default();
            let log: Arc<Mutex<Vec<String>>> = Arc::default();
            let (r, l) = (routes.clone(), log.clone());
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let (r, l) = (r.clone(), l.clone());
                    std::thread::spawn(move || answer(stream, &r, &l));
                }
            });
            Self { base, routes, log }
        }

        fn serve(&self, prefix: &str, path: &str, reply: Reply) {
            self.routes.lock().unwrap().insert(format!("/{prefix}/{path}"), reply);
        }

        /// The fixture's files on both hosts.
        fn serve_fixture(&self, f: &Fixture) {
            for (path, bytes) in &f.files {
                self.serve("cdn", &format!("gh/google/fonts@{COMMIT}/{path}"), Reply::Ok(bytes.clone()));
                self.serve("raw", &format!("google/fonts/{COMMIT}/{path}"), Reply::Ok(bytes.clone()));
            }
        }

        fn requests(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        fn client(&self) -> Client {
            let t = Timeouts { connect: Duration::from_secs(2), response: Duration::from_secs(5), overall: Duration::from_secs(10) };
            Client::for_tests(
                t,
                vec![
                    ("https://cdn.jsdelivr.net/".into(), format!("{}cdn/", self.base)),
                    ("https://raw.githubusercontent.com/".into(), format!("{}raw/", self.base)),
                ],
            )
        }
    }

    fn answer(mut s: TcpStream, routes: &Mutex<HashMap<String, Reply>>, log: &Mutex<Vec<String>>) {
        let mut head = Vec::new();
        let mut b = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            match s.read(&mut b) {
                Ok(1) => head.push(b[0]),
                _ => return,
            }
        }
        let text = String::from_utf8_lossy(&head).into_owned();
        let path = text.split_whitespace().nth(1).unwrap_or("/").to_string();
        log.lock().unwrap().push(path.clone());
        let reply = routes.lock().unwrap().get(&path).cloned().unwrap_or(Reply::Status(404));
        match reply {
            Reply::Ok(body) => {
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(&body);
            }
            Reply::Status(code) => {
                let _ = write!(s, "HTTP/1.1 {code} Nope\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnope");
            }
            Reply::Stall => {
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nabc");
                let _ = s.flush();
                std::thread::sleep(Duration::from_secs(20));
            }
        }
    }

    fn temp(name: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!("craft-font-store-{name}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> =
            std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }

    /// No font files, staged downloads or family folders anywhere in the store.
    fn assert_no_family_files(dir: &Path) {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let name = e.file_name().to_string_lossy().into_owned();
                    assert!(["settings.json", ".lock"].contains(&name.as_str()), "left behind: {}", p.display());
                }
            }
        }
        assert!(!dir.join("testsans").exists());
        assert!(entries(&dir.join(".staging")).is_empty(), "{:?}", entries(&dir.join(".staging")));
    }

    fn user_store(server: &Server, dir: &Path) -> FontStore {
        let store = FontStore::new(dir.to_path_buf(), Caller::User).with_client(server.client());
        store.enable_network().unwrap();
        store
    }

    fn none(_: Progress) {}

    #[test]
    fn consent_is_off_by_default_and_only_the_user_turns_it_on() {
        let server = Server::start();
        let f = fixture();
        server.serve_fixture(&f);
        let index = index_of(&f.index).unwrap();
        let dir = temp("consent");
        let user = FontStore::new(dir.clone(), Caller::User).with_client(server.client());
        let agent = FontStore::new(dir.clone(), Caller::Agent).with_client(server.client());
        assert_eq!((user.network(), agent.network()), (Network::Off, Network::Off));

        let face = index.find_face("Test Sans", "Bold").unwrap();
        assert!(matches!(user.install_face(&index, face, &Cancel::new(), &mut none), Err(Error::NetworkOff)));
        assert!(matches!(agent.enable_network(), Err(Error::NotAllowed)));
        assert_eq!(agent.network(), Network::Off);
        assert!(matches!(agent.install_face(&index, face, &Cancel::new(), &mut none), Err(Error::NetworkOff)));
        assert!(server.requests().is_empty(), "no request while the network is off");

        // The user's setting is persisted in the store folder; an agent then may install.
        user.enable_network().unwrap();
        assert_eq!(FontStore::new(dir.clone(), Caller::Agent).network(), Network::On);
        agent.install_face(&index, face, &Cancel::new(), &mut none).unwrap();
        agent.disable_network().unwrap();
        assert_eq!(user.network(), Network::Off);

        // The agent constructor opens the same folder as the user's.
        let env = FontEnv { os: Os::Unix, font_store: Some(dir.clone()), ..Default::default() };
        assert_eq!(FontStore::for_agent(&env).unwrap().caller(), Caller::Agent);
        assert!(matches!(FontStore::for_agent(&env).unwrap().enable_network(), Err(Error::NotAllowed)));
        assert_eq!(FontStore::for_user(&env).unwrap().dir(), dir);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn install_verify_and_remove_a_family() {
        let server = Server::start();
        let f = fixture();
        server.serve_fixture(&f);
        let index = index_of(&f.index).unwrap();
        let dir = temp("install");
        let root = temp("install-root");
        let env = FontEnv { os: Os::Unix, root: root.clone(), font_store: Some(dir.clone()), ..Default::default() };
        let catalog = Arc::new(Catalog::new(env).with_font_service(false));
        catalog.rescan();
        assert!(catalog.by_postscript_name("TestSans-Bold").is_none());
        let store = user_store(&server, &dir).with_catalog(catalog.clone());

        let mut seen = Vec::new();
        let family = index.family("Test Sans").unwrap();
        let installed = store.install_family(&index, family, &Cancel::new(), &mut |p| seen.push(p)).unwrap();
        assert_eq!(entries(&dir.join("testsans")), ["OFL.txt", "TestSans-Bold.ttf", "TestSans-Italic.ttf", "TestSans-Regular.ttf"]);
        assert_eq!(installed.files.len(), 3);
        assert_eq!((installed.dir.as_str(), installed.license_file.as_deref()), ("testsans", Some("OFL.txt")));
        assert!(installed.files.iter().all(|x| x.commit == COMMIT && x.sha256.len() == 64));
        assert_eq!(store.installed(), std::slice::from_ref(&installed));
        let last = seen.last().unwrap();
        assert_eq!((last.done, last.total, last.files), (family.size(), family.size(), 3));
        assert!(seen.windows(2).all(|w| w[0].done <= w[1].done));
        // The catalog was rescanned.
        assert_eq!(catalog.by_postscript_name("TestSans-Bold").unwrap().family, "Test Sans");
        assert!(store.verify().is_empty());
        assert!(store.is_installed(family, index.find_face("Test Sans", "Italic").unwrap().file));

        // Installed files aren't downloaded again.
        let before = server.requests().len();
        store.install_face(&index, index.find_face("Test Sans", "Bold").unwrap(), &Cancel::new(), &mut none).unwrap();
        assert_eq!(server.requests().len(), before);

        // Damaged and missing files are reported.
        std::fs::write(dir.join("testsans/TestSans-Italic.ttf"), b"damaged").unwrap();
        std::fs::remove_file(dir.join("testsans/TestSans-Regular.ttf")).unwrap();
        let problems = store.verify();
        assert_eq!(problems.len(), 2);
        assert!(problems.iter().any(|p| p.file == "TestSans-Italic.ttf" && matches!(p.error, Some(VerifyError::Size { .. }))));
        assert!(problems.iter().any(|p| p.file == "TestSans-Regular.ttf" && p.error.is_none()));

        // A single face of another family, with its licence.
        let face = index.find_face("Test Var", "Bold").unwrap();
        store.install_face(&index, face, &Cancel::new(), &mut none).unwrap();
        assert_eq!(entries(&dir.join("testvar")), ["LICENSE.txt", "TestVar[wght].ttf"]);
        assert_eq!(store.installed().iter().map(|f| f.family.as_str()).collect::<Vec<_>>(), ["Test Sans", "Test Var"]);

        store.remove_family("test sans").unwrap();
        assert!(!dir.join("testsans").exists());
        assert_eq!(store.installed().len(), 1);
        assert!(catalog.by_postscript_name("TestSans-Bold").is_none());
        assert!(matches!(store.remove_family("Test Sans"), Err(Error::NotInstalled(_))));
        assert!(entries(&dir.join(".staging")).is_empty());
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn jsdelivr_failures_fall_back_to_raw_github() {
        let server = Server::start();
        let f = fixture();
        server.serve_fixture(&f);
        let bold = format!("gh/google/fonts@{COMMIT}/ofl/testsans/TestSans-Bold.ttf");
        server.serve("cdn", &bold, Reply::Status(503));
        let index = index_of(&f.index).unwrap();
        let dir = temp("fallback");
        let store = user_store(&server, &dir);
        store.install_face(&index, index.find_face("Test Sans", "Bold").unwrap(), &Cancel::new(), &mut none).unwrap();
        let requests = server.requests();
        assert_eq!(requests.first().map(String::as_str), Some(format!("/cdn/{bold}").as_str()));
        assert_eq!(requests.get(1).map(String::as_str), Some(format!("/raw/google/fonts/{COMMIT}/ofl/testsans/TestSans-Bold.ttf").as_str()));
        assert!(store.verify().is_empty());

        // A corrupted copy on jsDelivr also falls back.
        let italic = format!("gh/google/fonts@{COMMIT}/ofl/testsans/TestSans-Italic.ttf");
        server.serve("cdn", &italic, Reply::Ok(font("Test Sans", "Bold", 700)));
        store.install_face(&index, index.find_face("Test Sans", "Italic").unwrap(), &Cancel::new(), &mut none).unwrap();
        assert!(store.verify().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_downloads_are_rejected_and_leave_nothing() {
        let server = Server::start();
        let f = fixture();
        server.serve_fixture(&f);
        let index = index_of(&f.index).unwrap();
        let dir = temp("corrupt");
        let store = user_store(&server, &dir);

        // Wrong bytes of the right size on both hosts: rejected on the sha256.
        let mut other = font("Test Sans", "Italic", 400);
        *other.last_mut().unwrap() ^= 1;
        for (host, path) in [("cdn", format!("gh/google/fonts@{COMMIT}")), ("raw", format!("google/fonts/{COMMIT}"))] {
            server.serve(host, &format!("{path}/ofl/testsans/TestSans-Italic.ttf"), Reply::Ok(other.clone()));
        }
        let family = index.family("Test Sans").unwrap();
        let r = store.install_family(&index, family, &Cancel::new(), &mut none);
        assert!(matches!(r, Err(Error::Verify { error: VerifyError::Sha256, .. })), "{r:?}");
        assert_no_family_files(&dir);
        assert!(store.installed().is_empty());

        // Bytes that match the index but aren't a font.
        let junk = b"this is not a font file".to_vec();
        let mut v = f.index.clone();
        v["families"][2]["files"][0]["sha256"] = json!(sha256_hex(&junk));
        v["families"][2]["files"][0]["size"] = json!(junk.len());
        server.serve("cdn", &format!("gh/google/fonts@{COMMIT}/ofl/testsans/TestSans-Bold.ttf"), Reply::Ok(junk.clone()));
        server.serve("raw", &format!("google/fonts/{COMMIT}/ofl/testsans/TestSans-Bold.ttf"), Reply::Ok(junk));
        let junk_index = index_of(&v).unwrap();
        let r = store.install_face(&junk_index, junk_index.find_face("Test Sans", "Bold").unwrap(), &Cancel::new(), &mut none);
        assert!(matches!(r, Err(Error::Verify { error: VerifyError::NotAFont, .. })), "{r:?}");
        assert_no_family_files(&dir);

        // A file missing on both hosts, after others downloaded: nothing of the family is written.
        server.serve_fixture(&f);
        for (host, path) in [("cdn", format!("gh/google/fonts@{COMMIT}")), ("raw", format!("google/fonts/{COMMIT}"))] {
            server.serve(host, &format!("{path}/ofl/testsans/TestSans-Regular.ttf"), Reply::Status(404));
        }
        let r = store.install_family(&index, family, &Cancel::new(), &mut none);
        assert!(matches!(&r, Err(Error::Download { reasons, .. }) if reasons.len() == 2), "{r:?}");
        assert_no_family_files(&dir);
        assert!(!dir.join("manifest.json").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_commit_keeps_the_installed_files() {
        use std::os::unix::fs::PermissionsExt;
        let server = Server::start();
        let f = fixture();
        server.serve_fixture(&f);
        let index = index_of(&f.index).unwrap();
        let dir = temp("commit");
        let store = user_store(&server, &dir);
        store.install_face(&index, index.find_face("Test Sans", "Regular").unwrap(), &Cancel::new(), &mut none).unwrap();
        let folder = dir.join("testsans");
        let before = (store.installed(), entries(&folder));

        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).unwrap();
        let writable = std::fs::File::create(folder.join("probe")).is_ok();
        let r = store.install_family(&index, index.family("Test Sans").unwrap(), &Cancel::new(), &mut none);
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
        if writable {
            return; // root ignores permissions
        }
        assert!(matches!(r, Err(Error::Io { .. })), "{r:?}");
        assert_eq!((store.installed(), entries(&folder)), before);
        assert!(entries(&dir.join(".staging")).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn cancel_stops_a_stalled_install() {
        let server = Server::start();
        let f = fixture();
        server.serve_fixture(&f);
        server.serve("cdn", &format!("gh/google/fonts@{COMMIT}/ofl/testsans/TestSans-Bold.ttf"), Reply::Stall);
        let index = index_of(&f.index).unwrap();
        let dir = temp("cancel");
        let store = user_store(&server, &dir);
        let cancel = Cancel::new();
        let c = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            c.cancel();
        });
        let t = Instant::now();
        let r = store.install_family(&index, index.family("Test Sans").unwrap(), &cancel, &mut none);
        assert!(matches!(r, Err(Error::Cancelled)), "{r:?}");
        assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
        assert_no_family_files(&dir);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_index_is_fetched_only_with_consent_and_checked() {
        let server = Server::start();
        let f = fixture();
        let json = f.index.to_string().into_bytes();
        let pin = IndexPin { repo: "owner/index", commit: "abc", sha256: Box::leak(sha256_hex(&json).into_boxed_str()) };
        server.serve("cdn", "gh/owner/index@abc/google-fonts-index.json", Reply::Status(500));
        server.serve("raw", "owner/index/abc/google-fonts-index.json", Reply::Ok(json.clone()));
        let dir = temp("index");
        let store = FontStore::new(dir.clone(), Caller::User).with_client(server.client());
        assert!(store.cached_index(&pin).is_none());
        assert!(matches!(store.fetch_index(&pin, &Cancel::new()), Err(Error::NetworkOff)));
        assert!(server.requests().is_empty());

        store.enable_network().unwrap();
        assert_eq!(store.fetch_index(&pin, &Cancel::new()).unwrap().commit(), COMMIT);
        assert_eq!(store.cached_index(&pin).unwrap().families().len(), 4);

        let wrong = IndexPin { sha256: "0000000000000000000000000000000000000000000000000000000000000000", ..pin };
        assert!(store.cached_index(&wrong).is_none());
        assert!(matches!(store.fetch_index(&wrong, &Cancel::new()), Err(Error::Verify { error: VerifyError::Sha256, .. })));
        let _ = std::fs::remove_dir_all(dir);
    }
}
