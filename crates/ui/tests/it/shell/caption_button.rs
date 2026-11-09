//! Caption buttons: the exclusive window parts they declare, the click they report, the slot layout of a bar and the
//! icon-theme search behind the round families' glyphs.

use crate::windows_kit::Windows;
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::runtime::windows::{WindowHit, WindowHost};
use craft_ui::shell::{
    Caption, CaptionButton, CaptionEnd, CaptionFamily, CaptionIcon, CaptionIcons, CaptionPlan, Captions, ThemeFiles, base_dirs, caption_icon_theme,
};
use craft_ui::testing::{Fixture, trial};
use craft_window::Layout;
use egui::{Key, Modifiers, Rect, ViewportId, pos2};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

const BAR: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(600.0, 32.0));
const GLYPH_PX: u32 = 16;

fn captions(family: CaptionFamily, left: &[Caption], right: &[Caption]) -> Captions {
    Captions { family, layout: Layout { left: left.to_vec(), right: right.to_vec() } }
}

/// A shown button is an exclusive part of its window with its caption, so the Windows hit test answers a press on it
/// as that caption.
#[trial]
fn shown_buttons_are_the_windows_caption_record() {
    let windows = Windows::new();
    let host = WindowHost::of(&windows.ctx);
    let plan = CaptionPlan::new(&captions(CaptionFamily::Windows, &[], &[Caption::Minimize, Caption::Maximize, Caption::Close]), false);
    let slots = plan.place(BAR, BAR.left(), BAR.right());
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| {
            for slot in &slots {
                let button = CaptionButton::new(slot.caption, CaptionFamily::Windows).id_salt(format!("{:?}", slot.caption));
                match slot.end {
                    Some(end) => button.outermost(end).show_at(ui, slot.rect),
                    None => button.show_at(ui, slot.rect),
                };
            }
        });
    }
    let regions = host.caption_regions(ViewportId::ROOT).expect("the buttons registered their parts");
    assert_eq!(regions.buttons.iter().map(|(caption, _)| *caption).collect::<Vec<_>>(), [Caption::Minimize, Caption::Maximize, Caption::Close]);
    let close = slots.iter().find(|slot| slot.caption == Caption::Close).expect("a close slot");
    assert_eq!(regions.hit(close.rect.center()), Some(WindowHit::Caption(Caption::Close)));
}

/// A press reports a click on the button; the button is not a Tab stop.
#[trial]
fn a_press_clicks_the_button_and_tab_skips_it() {
    let clicks = Rc::new(Cell::new(0));
    let id = Rc::new(Cell::new(None));
    let (shown_clicks, shown_id) = (clicks.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let response = CaptionButton::new(Caption::Close, CaptionFamily::Adwaita).show(ui);
        shown_id.set(Some(response.id));
        if craft_ui::testing::contract::clicked(&response) {
            shown_clicks.set(shown_clicks.get() + 1);
        }
    });
    fixture.settle();
    let id = id.get().expect("the button showed");
    let button = fixture.node_of(id).expect("the button has a node");
    fixture.click(&button);
    assert_eq!(clicks.get(), 1);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    assert_ne!(fixture.focused(), Some(id), "caption buttons are no Tab stops");
}

/// Windows captions mirror under right-to-left; the Linux layouts are the user's setting and stay as they are.
#[trial]
fn only_the_windows_captions_mirror_under_right_to_left() {
    let layout = [Caption::Minimize, Caption::Maximize, Caption::Close];
    let windows = CaptionPlan::new(&captions(CaptionFamily::Windows, &[], &layout), true);
    assert_eq!(windows.left(), [Caption::Close, Caption::Maximize, Caption::Minimize]);
    assert!(windows.right().is_empty());
    let adwaita = CaptionPlan::new(&captions(CaptionFamily::Adwaita, &[Caption::Close], &[Caption::Minimize]), true);
    assert_eq!((adwaita.left(), adwaita.right()), (&[Caption::Close][..], &[Caption::Minimize][..]));
}

/// An icon tree held in memory: path to file contents.
#[derive(Default)]
struct Tree(BTreeMap<PathBuf, Vec<u8>>);

impl Tree {
    fn with(files: &[(&str, &str)]) -> Self {
        Tree(files.iter().map(|(path, text)| (PathBuf::from(path), text.as_bytes().to_vec())).collect())
    }
}

impl ThemeFiles for Tree {
    fn is_dir(&self, path: &Path) -> bool {
        self.0.keys().any(|file| file.starts_with(path) && file != path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.0.contains_key(path)
    }

    fn subdirectories(&self, dir: &Path) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = self
            .0
            .keys()
            .filter_map(|file| file.strip_prefix(dir).ok())
            .filter(|rest| rest.components().count() > 1)
            .filter_map(|rest| rest.components().next())
            .map(|first| dir.join(first))
            .collect();
        found.dedup();
        found
    }

    fn read(&self, path: &Path, limit: u64) -> Option<Vec<u8>> {
        self.0.get(path).map(|bytes| bytes.iter().copied().take(limit as usize).collect())
    }
}

const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M4 4h8v8H4z" fill="#2e3436"/></svg>"##;
const FIXED_16: &str = "[Icon Theme]\nDirectories=s\n[s]\nSize=16\nType=Fixed\n";

fn base() -> PathBuf {
    PathBuf::from("/icons")
}

/// A glyph file drawing a square of `side` units, so a test tells which file a theme search chose.
fn drawn_square(side: u32) -> String {
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M0 0h{side}v{side}H0z" fill="#2e3436"/></svg>"##)
}

fn covered(icons: &CaptionIcons, icon: CaptionIcon) -> Option<usize> {
    icons.mask(icon, GLYPH_PX).map(|mask| mask.iter().filter(|coverage| **coverage > 0).count())
}

/// Icons come from the theme, then its parents, then `hicolor`, the best-fitting directory first; a directory the
/// index forgot still counts.
#[trial]
fn icons_come_from_the_theme_then_its_parents_then_hicolor_best_size_first() {
    let (small, medium, large) = (drawn_square(4), drawn_square(8), drawn_square(12));
    let files = Tree::with(&[
        (
            "/icons/Child/index.theme",
            "[Icon Theme]\nInherits=Parent, Child\nDirectories=48,16\n[48]\nSize=48\nType=Fixed\n[16]\nSize=16\nType=Fixed\n",
        ),
        ("/icons/Child/48/window-close-symbolic.svg", large.as_str()),
        ("/icons/Child/16/window-close-symbolic.svg", small.as_str()),
        ("/icons/Child/symbolic/ui/window-restore-symbolic.svg", small.as_str()),
        ("/icons/Parent/index.theme", "[Icon Theme]\nDirectories=ui\n[ui]\nSize=16\nMinSize=8\nMaxSize=512\nType=Scalable\n"),
        ("/icons/Parent/ui/window-minimize-symbolic.svg", medium.as_str()),
        ("/icons/Parent/ui/window-restore-symbolic.svg", large.as_str()),
        ("/icons/hicolor/index.theme", "[Icon Theme]\nDirectories=s\n[s]\nSize=16\nType=Scalable\n"),
        ("/icons/hicolor/s/window-maximize-symbolic.svg", large.as_str()),
    ]);
    let icons = CaptionIcons::from_theme("Child", &[base()], &files).expect("the theme has glyphs");
    let area = |side: usize| Some(side * side);
    assert_eq!(covered(&icons, CaptionIcon::Close), area(4), "16 px before 48 px");
    assert_eq!(covered(&icons, CaptionIcon::Minimize), area(8), "from the parent theme");
    assert_eq!(covered(&icons, CaptionIcon::Maximize), area(12), "from hicolor");
    assert_eq!(covered(&icons, CaptionIcon::Restore), area(4), "an unlisted directory, but the theme's own");
    assert!(CaptionIcons::from_theme("Nowhere", &[base()], &files).is_some(), "hicolor still answers");
}

/// The theme's glyph renders as a coverage mask; one the theme lacks answers `None` so the family draws its own.
#[trial]
fn caption_icons_render_the_themes_glyph() {
    let files = Tree::with(&[("/icons/T/index.theme", FIXED_16), ("/icons/T/s/window-close-symbolic.svg", SQUARE)]);
    let icons = CaptionIcons::from_theme("T", &[base()], &files).expect("the theme has a close glyph");
    assert_eq!(icons.theme(), "T");
    let mask = icons.mask(CaptionIcon::Close, GLYPH_PX).expect("renders");
    let side = GLYPH_PX as usize;
    assert_eq!(mask.len(), side * side);
    assert_eq!((mask[side * side / 2 + side / 2], mask[0]), (255, 0), "covered inside the square, clear outside it");
    assert!(icons.mask(CaptionIcon::Minimize, GLYPH_PX).is_none(), "the theme lacks it");
    assert!(CaptionIcons::from_theme("T", &[PathBuf::from("/empty")], &files).is_none());
}

/// A theme is searched in `XDG_DATA_DIRS` order, and the setting that names it is the portal's, then the desktop's.
#[trial]
fn theme_places_and_names_follow_the_desktop() {
    let var = |key: &str| match key {
        "HOME" => Some("/home/u".to_string()),
        "XDG_DATA_DIRS" => Some("/a:rel:/b".to_string()),
        _ => None,
    };
    let want: Vec<PathBuf> = ["/home/u/.icons", "/home/u/.local/share/icons", "/a/icons", "/b/icons"].iter().map(PathBuf::from).collect();
    assert_eq!(base_dirs(&var), want);
    assert_eq!(base_dirs(&|_| None), [PathBuf::from("/usr/local/share/icons"), PathBuf::from("/usr/share/icons")]);
    assert_eq!(caption_icon_theme(Some("A"), Some("B")), "A");
    assert_eq!(caption_icon_theme(None, Some("B")), "B");
    assert_eq!(caption_icon_theme(None, None), "Adwaita");
}

/// Neighbours' hit areas touch, the outermost button's runs to the window's edge and is the widest, and Windows
/// buttons are all as wide as each other.
#[trial]
fn slots_fill_the_window_edges() {
    {
        let plan = CaptionPlan::new(&captions(CaptionFamily::Adwaita, &[Caption::Close], &[Caption::Minimize, Caption::Maximize]), false);
        let slots = plan.place(BAR, BAR.left(), BAR.right());
        let [close, minimize, maximize] = slots[..] else { panic!("three slots: {slots:?}") };
        assert_eq!((close.rect.left(), close.end), (BAR.left(), Some(CaptionEnd::Left)));
        assert_eq!((maximize.rect.right(), maximize.end), (BAR.right(), Some(CaptionEnd::Right)));
        assert_eq!((minimize.rect.right(), minimize.end), (maximize.rect.left(), None));
        assert!(slots.iter().all(|slot| slot.rect.top() == BAR.top() && slot.rect.bottom() == BAR.bottom()), "full bar height");
        assert!(close.rect.width() > minimize.rect.width(), "the outermost hit area includes the gap to the window's edge");

        let windows = CaptionPlan::new(&captions(CaptionFamily::Windows, &[], &[Caption::Minimize, Caption::Close]), false);
        let slots = windows.place(BAR, BAR.left(), BAR.right());
        let width = slots[0].rect.width();
        assert!(slots.iter().all(|slot| slot.rect.width() == width), "Windows buttons share one width");
        assert_eq!(windows.right_width(), 2.0 * width);
    }
    {
        let plan = CaptionPlan::new(&captions(CaptionFamily::Windows, &[], &[Caption::Close]), false);
        let slots = plan.place(BAR, 40.0, 500.0);
        assert_eq!(slots.iter().map(|slot| slot.rect.right()).collect::<Vec<_>>(), [500.0]);
    }
}

/// Hostile theme files: cycles, escapes and junk sizes end the search, never hang or panic.
#[trial]
fn hostile_theme_files_render_nothing_and_never_hang() {
    {
        let files = Tree::with(&[
            (
                "/icons/A/index.theme",
                "[Icon Theme]\nInherits=B,../etc,\nDirectories=../../x,,/abs,ok\n[ok]\nSize=99999999999999999999\nScale=-3\nType=Threshold\n",
            ),
            ("/icons/A/ok/window-close-symbolic.svg", SQUARE),
            ("/icons/B/index.theme", "[Icon Theme]\nInherits=A\n"),
        ]);
        let icons = CaptionIcons::from_theme("A", &[base()], &files).expect("loads");
        assert!(covered(&icons, CaptionIcon::Close).is_some_and(|area| area > 0));
        assert!(CaptionIcons::from_theme("..", &[base()], &files).is_none());
    }
    {
        let gzip = [&[0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff][..], SQUARE.as_bytes()].concat();
        let dtd = r##"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "x">]><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M4 4h8v8H4z"/></svg>"##;
        let many = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">{}</svg>"#, "<g/>".repeat(20_000));
        let image = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><image href="/dev/zero" width="16" height="16"/></svg>"##;
        let mask_of = |svg: &[u8], px: u32| {
            let files = [Some(Arc::<[u8]>::from(svg)), None, None, None];
            CaptionIcons::from_files("T", 0, files).mask(CaptionIcon::Minimize, px)
        };
        for hostile in [&gzip[..], dtd.as_bytes(), many.as_bytes(), b"not svg"] {
            assert!(mask_of(hostile, GLYPH_PX).is_none());
        }
        assert!(mask_of(SQUARE.as_bytes(), 0).is_none() && mask_of(SQUARE.as_bytes(), 100_000).is_none());
        assert!(mask_of(image.as_bytes(), GLYPH_PX).is_some_and(|mask| mask.iter().all(|coverage| *coverage == 0)), "nothing drawn");
    }
}
