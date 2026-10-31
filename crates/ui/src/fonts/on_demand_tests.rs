//! On-demand registration in a harness, with a fixed candidate source.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use egui::FontDefinitions;

use super::*;

/// A test face with 日 (and not the Arabic letters of `ARABIC`).
const CJK: &[u8] = include_bytes!("../../tests/fonts/NotoSansCJKsc-test.otf");
const ARABIC: &[u8] = include_bytes!("../../tests/fonts/NotoSansArabic-test.ttf");

/// A fixed list of candidate files whose character maps are read for real.
struct Fixed {
    files: Vec<PathBuf>,
    covers: AtomicUsize,
}

impl FontCandidates for Fixed {
    fn files(&self, _script: &str, _han: Option<Han>) -> Vec<PathBuf> {
        self.files.clone()
    }

    fn covers(&self, data: &[u8], c: char) -> Option<u32> {
        self.covers.fetch_add(1, Ordering::Relaxed);
        SystemCandidates.covers(data, c)
    }
}

/// Files written to a folder of their own, removed at the end of the test.
struct Files(PathBuf);

impl Files {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("craft-ui-on-demand-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the test folder");
        Files(dir)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).expect("write a test font");
        path
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn pass(ctx: &egui::Context, text: &str) {
    let mut output = ctx.run_ui(Default::default(), |ui| {
        ui.label(text);
    });
    // The headless test never uploads the atlas.
    output.textures_delta.clear();
}

fn registered(ctx: &egui::Context) -> Vec<(String, PathBuf, u32)> {
    ctx.with_plugin::<OnDemandFonts, _>(|plugin| plugin.registered().to_vec()).expect("the loader is installed")
}

// A Japanese character with no glyph makes the loader read the first candidate that covers it and
// register one face in that pass; the next pass draws it.
#[test]
fn a_missing_character_registers_the_first_covering_candidate() {
    assert!(covering_face_of(CJK, '日'), "the test face has 日");
    let files = Files::new("first-covering");
    let paths = vec![files.write("a-arabic.ttf", ARABIC), files.write("b-cjk.otf", CJK), files.write("c-cjk.otf", CJK)];
    let ctx = egui::Context::default();
    OnDemandFonts::new(Arc::new(Fixed { files: paths.clone(), covers: AtomicUsize::new(0) }), None).install(&ctx);

    // The pass after an install only lets the new fonts take effect.
    pass(&ctx, "日");
    assert!(registered(&ctx).is_empty());

    pass(&ctx, "日");
    assert_eq!(registered(&ctx), [("on-demand-0".to_owned(), paths[1].clone(), 0)]);

    pass(&ctx, "日");
    pass(&ctx, "日");
    assert_eq!(registered(&ctx).len(), 1, "the registered face has the glyph, so nothing more is read");
    assert!(ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::proportional(12.0), '日')));
}

fn covering_face_of(data: &[u8], c: char) -> bool {
    SystemCandidates.covers(data, c).is_some()
}

// After MAX_SCRIPT_READS files of a script failed to cover, no further file of that script is read.
#[test]
fn a_script_stops_reading_after_the_cap() {
    let files = Files::new("cap");
    let paths: Vec<PathBuf> = (0..MAX_SCRIPT_READS + 3).map(|i| files.write(&format!("{i}.ttf"), ARABIC)).collect();
    let candidates = Arc::new(Fixed { files: paths, covers: AtomicUsize::new(0) });
    let ctx = egui::Context::default();
    OnDemandFonts::new(candidates.clone(), None).install(&ctx);

    for _ in 0..5 {
        pass(&ctx, "日");
    }
    assert_eq!(candidates.covers.load(Ordering::Relaxed), MAX_SCRIPT_READS);
    assert!(registered(&ctx).is_empty());
}

// A `ja` UI language prefers Japanese forms; an `en` UI with a `zh-TW` OS locale prefers Traditional.
#[test]
fn han_preference_follows_ui_language_then_os_locale() {
    assert_eq!(han_for_locale("ja", Some("zh-TW")), Some(Han::Japanese));
    assert_eq!(han_for_locale("zh-CN", Some("ja_JP.UTF-8")), Some(Han::Simplified));
    assert_eq!(han_for_locale("en", Some("zh-TW")), Some(Han::Traditional));
    assert_eq!(han_for_locale("en", Some("ko_KR.UTF-8")), Some(Han::Korean));
    assert_eq!(han_for_locale("en", Some("en_US.UTF-8")), None);
    assert_eq!(han_for_locale("en", None), None);
}

// `deflated_default_definitions` lists egui's faces in egui's family order.
#[test]
fn deflated_defaults_match_egui_defaults() {
    let egui = FontDefinitions::default();
    assert!(!egui.font_data.is_empty(), "test builds enable egui's default_fonts");
    assert_eq!(deflated_default_definitions(), egui);
}
