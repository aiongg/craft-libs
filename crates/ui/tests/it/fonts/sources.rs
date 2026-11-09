//! The font sources: on-demand faces over a fixed candidate list, the Han preference, egui's deflated default faces and
//! the system font provider.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use craft_host::Wake;
use craft_i18n::Han;
use craft_ui::fonts::{
    FontCandidates, MAX_SCRIPT_READS, OnDemandFonts, SystemCandidates, deflated_default_definitions, han_for_locale, on_demand_registered,
};
use craft_ui::testing::{Frames, trial};
use egui::{FontFamily, FontId};

/// A test face with 日 (and not the Arabic letters of `ARABIC`).
const CJK: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-test.otf");
const ARABIC: &[u8] = include_bytes!("../../fonts/NotoSansArabic-test.ttf");

/// The size a glyph is probed at.
const PROBE_SIZE: f32 = 12.0;

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

fn show(frames: &mut Frames, text: &str) {
    frames.frame(vec![], |ui| {
        ui.label(text);
    });
}

/// A Japanese character with no glyph makes the loader read the first candidate that covers it and register one face
/// in that pass; the next pass draws it.
#[trial]
fn a_missing_character_registers_the_first_covering_candidate() {
    assert!(SystemCandidates.covers(CJK, '日').is_some(), "the test face has 日");
    let files = Files::new("first-covering");
    let paths = vec![files.write("a-arabic.ttf", ARABIC), files.write("b-cjk.otf", CJK), files.write("c-cjk.otf", CJK)];
    let mut frames = Frames::new();
    OnDemandFonts::new(Arc::new(Fixed { files: paths.clone(), covers: AtomicUsize::new(0) }), None).install(&frames.ctx);

    // The pass after an install only lets the new fonts take effect.
    show(&mut frames, "日");
    assert!(on_demand_registered(&frames.ctx).is_empty());

    show(&mut frames, "日");
    assert_eq!(on_demand_registered(&frames.ctx), [("on-demand-0".to_owned(), paths[1].clone(), 0)]);

    show(&mut frames, "日");
    show(&mut frames, "日");
    assert_eq!(on_demand_registered(&frames.ctx).len(), 1, "the registered face has the glyph, so nothing more is read");
    assert!(frames.ctx.fonts_mut(|fonts| fonts.has_glyph(&FontId::new(PROBE_SIZE, FontFamily::Proportional), '日')));
}

/// After `MAX_SCRIPT_READS` files of a script failed to cover, no further file of that script is read.
#[trial]
fn a_script_stops_reading_after_the_cap() {
    let files = Files::new("cap");
    let paths: Vec<PathBuf> = (0..MAX_SCRIPT_READS + 3).map(|i| files.write(&format!("{i}.ttf"), ARABIC)).collect();
    let candidates = Arc::new(Fixed { files: paths, covers: AtomicUsize::new(0) });
    let mut frames = Frames::new();
    OnDemandFonts::new(candidates.clone(), None).install(&frames.ctx);

    for _ in 0..5 {
        show(&mut frames, "日");
    }
    assert_eq!(candidates.covers.load(Ordering::Relaxed), MAX_SCRIPT_READS);
    assert!(on_demand_registered(&frames.ctx).is_empty());
}

/// `deflated_default_definitions` holds egui's default font files, with egui's tweaks and family order.
#[trial]
fn deflated_defaults_match_egui_defaults() {
    let ours = deflated_default_definitions();
    let bytes = |name: &str| -> Vec<u8> { ours.font_data.get(name).unwrap_or_else(|| panic!("{name} is missing")).font.as_ref().as_ref().to_vec() };
    assert_eq!(bytes("Hack"), epaint_default_fonts::HACK_REGULAR);
    assert_eq!(bytes("Ubuntu-Light"), epaint_default_fonts::UBUNTU_LIGHT);
    assert_eq!(bytes("egui-icons"), epaint_default_fonts::EGUI_ICONS);
    let names = |family: FontFamily| ours.families[&family].clone();
    assert_eq!(names(FontFamily::Proportional), ["Ubuntu-Light", "egui-icons"]);
    assert_eq!(names(FontFamily::Monospace), ["Hack", "Ubuntu-Light", "egui-icons"]);
}

/// A source whose lookup is still running: it keeps the wake it was given.
struct Slow {
    wakes: std::sync::Mutex<Vec<Wake>>,
}

impl FontCandidates for Slow {
    fn files(&self, _script: &str, _han: Option<Han>) -> Vec<PathBuf> {
        Vec::new()
    }

    fn covers(&self, _data: &[u8], _c: char) -> Option<u32> {
        None
    }

    fn pending(&self) -> bool {
        true
    }

    fn wake_when_ready(&self, wake: Wake) {
        self.wakes.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(wake);
    }
}

/// While a background lookup runs the loader asks once to be woken and requests no timed repaint; the lookup's wake
/// then requests the frame.
#[trial]
fn a_pending_lookup_wakes_the_ui_instead_of_polling() {
    let slow = Arc::new(Slow { wakes: Default::default() });
    let mut frames = Frames::new();
    let repaints = Arc::new(AtomicUsize::new(0));
    let counter = repaints.clone();
    frames.ctx.set_request_repaint_callback(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    OnDemandFonts::new(slow.clone(), None).install(&frames.ctx);
    for _ in 0..4 {
        show(&mut frames, "日");
    }
    show(&mut frames, "日");
    assert_eq!(frames.repaint_delay(), None, "no timer while the lookup runs");
    let wakes = slow.wakes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(wakes.len(), 1, "asked once per lookup");
    let before = repaints.load(Ordering::Relaxed);
    wakes[0].wake();
    assert!(repaints.load(Ordering::Relaxed) > before, "the wake requests a frame");
}

/// A context asking for the OS's interface face is woken once the index of installed fonts is built, and the face is
/// then known without waiting.
#[cfg(feature = "system_fonts")]
#[trial]
fn the_system_interface_face_resolves_in_the_background_and_wakes_the_context() {
    use craft_ui::fonts::{poll_system_ui_face, system_ui_face};
    let ctx = egui::Context::default();
    let woken = Arc::new(AtomicUsize::new(0));
    let counter = woken.clone();
    ctx.set_request_repaint_callback(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    let ready = (0..6000).find_map(|_| match poll_system_ui_face(&ctx) {
        std::task::Poll::Ready(face) => Some(face),
        std::task::Poll::Pending => {
            std::thread::sleep(std::time::Duration::from_millis(5));
            None
        }
    });
    let face = ready.expect("the face resolves once the index is built");
    assert_eq!(face.map(|face| face.family.as_str()), system_ui_face().map(|face| face.family.as_str()));
    assert!(woken.load(Ordering::Relaxed) > 0, "the context was woken when the face resolved");
}
