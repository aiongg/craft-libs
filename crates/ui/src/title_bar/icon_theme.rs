//! Linux: the caption glyphs from the desktop's freedesktop icon theme (the Icon Theme
//! Specification). A theme is searched, then the themes it inherits, then `hicolor`; within each,
//! its directories from the best fit for a 16 px icon down. Only SVG files are used: symbolic
//! icons are drawn as a coverage mask in the button's ink colour.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{CaptionIcon, CaptionIcons};

/// Bounds on what a theme's files can make us do: themes in an inheritance chain, directories a
/// theme lists, and the bytes read from one file.
const MAX_THEMES: usize = 32;
const MAX_DIRS: usize = 4096;
/// How deep an index's unlisted directories are looked for (`symbolic/ui`, `16x16@2/actions`).
const SCAN_DEPTH: usize = 3;
const MAX_FILE: u64 = 1 << 20;
/// The size the caption glyphs are drawn at, in the spec's units.
const ICON_SIZE: i64 = 16;

/// A theme or icon name that stays inside its directory.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// At most [`MAX_FILE`] bytes of a file (settings, theme indexes and icons are small).
fn read_small(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut bytes = vec![];
    std::fs::File::open(path).ok()?.take(MAX_FILE).read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

/// Where themes live, in the spec's order: `~/.icons`, `$XDG_DATA_HOME/icons`, then each
/// `$XDG_DATA_DIRS/icons`. Relative paths in the variables are ignored, as the XDG spec says.
fn base_dirs(var: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let absolute = |p: &String| Path::new(p).is_absolute();
    let home = var("HOME").filter(absolute).map(PathBuf::from);
    let mut dirs = vec![];
    if let Some(home) = &home {
        dirs.push(home.join(".icons"));
    }
    if let Some(data) = var("XDG_DATA_HOME").filter(absolute).map(PathBuf::from).or_else(|| home.map(|h| h.join(".local").join("share"))) {
        dirs.push(data.join("icons"));
    }
    let data_dirs = var("XDG_DATA_DIRS").filter(|d| !d.trim().is_empty()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(data_dirs.split(':').filter(|d| Path::new(d).is_absolute()).map(|d| Path::new(d).join("icons")));
    dirs
}

/// One `[section]` of an `index.theme`, as (key, value) pairs.
fn section<'a>(index: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let mut inside = false;
    let mut pairs = vec![];
    for line in index.lines().map(str::trim) {
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if inside {
                break;
            }
            inside = s == name;
        } else if inside && let Some((k, v)) = line.split_once('=') {
            pairs.push((k.trim(), v.trim()));
        }
    }
    pairs
}

fn list(value: Option<&str>) -> impl Iterator<Item = &str> {
    value.unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty())
}

/// How far a theme directory is from a 16 px icon at scale 1 (the spec's
/// `DirectorySizeDistance`), then its scale: 0 is an exact fit.
fn distance(keys: &[(&str, &str)]) -> (i64, i64) {
    let get = |k: &str| keys.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
    let num = |k: &str, default: i64| get(k).and_then(|v| v.parse::<i64>().ok()).unwrap_or(default).clamp(0, 1 << 16);
    let size = num("Size", 0);
    let scale = num("Scale", 1).max(1);
    let d = match get("Type").unwrap_or("Threshold") {
        "Fixed" => (size * scale - ICON_SIZE).abs(),
        "Scalable" => {
            let (min, max) = (num("MinSize", size) * scale, num("MaxSize", size) * scale);
            (min - ICON_SIZE).max(ICON_SIZE - max).max(0)
        }
        _ => ((size * scale - ICON_SIZE).abs() - num("Threshold", 2) * scale).max(0),
    };
    (d, scale)
}

/// Every directory under `root`, at most [`SCAN_DEPTH`] levels down and [`MAX_DIRS`] in all, in
/// path order: scalable and symbolic ones first, as icons there fit any size.
fn subdirectories(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![];
    let mut level = vec![root.to_path_buf()];
    for _ in 0..SCAN_DEPTH {
        let mut next = vec![];
        for dir in &level {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for entry in entries.flatten().take(MAX_DIRS) {
                let path = entry.path();
                if found.len() + next.len() < MAX_DIRS && path.is_dir() {
                    next.push(path);
                }
            }
        }
        found.extend(next.iter().cloned());
        level = next;
    }
    let vector = |p: &PathBuf| !p.components().any(|c| matches!(c.as_os_str().to_str(), Some("scalable" | "symbolic")));
    found.sort_by_key(|p| (vector(p), p.clone()));
    found
}

/// One theme of a chain: its listed directories, best fit first, in every base directory that has
/// the theme, and (looked for only when an icon isn't in those) the directories its index forgot.
struct Theme {
    roots: Vec<PathBuf>,
    listed: Vec<PathBuf>,
    unlisted: std::cell::OnceCell<Vec<PathBuf>>,
}

impl Theme {
    /// The directories the index forgot to list: GTK still finds icons there, and custom themes
    /// often add `symbolic/ui` without listing it.
    fn unlisted(&self) -> &[PathBuf] {
        self.unlisted.get_or_init(|| {
            let listed: std::collections::HashSet<&PathBuf> = self.listed.iter().collect();
            self.roots.iter().flat_map(|r| subdirectories(r)).filter(|d| !listed.contains(d)).collect()
        })
    }
}

/// A theme and the themes it inherits, in search order.
struct IconTheme {
    themes: Vec<Theme>,
}

impl IconTheme {
    /// `name` and the themes it inherits, then `hicolor`. `None` when no theme in the chain has an
    /// `index.theme`.
    fn load(name: &str, bases: &[PathBuf]) -> Option<Self> {
        let mut chain: Vec<String> = vec![name.to_string()];
        let mut themes = vec![];
        let mut i = 0;
        loop {
            let Some(theme) = chain.get(i).cloned() else {
                // Every chain ends in hicolor, whether or not the themes before it exist.
                if chain.iter().any(|t| t == "hicolor") {
                    break;
                }
                chain.push("hicolor".into());
                continue;
            };
            i += 1;
            if !valid_name(&theme) {
                continue;
            }
            let roots: Vec<PathBuf> = bases.iter().map(|b| b.join(&theme)).filter(|r| r.is_dir()).collect();
            // The first index wins; a theme can still be spread over several base directories.
            let Some(index) = roots.iter().find_map(|r| read_small(&r.join("index.theme"))) else { continue };
            let index = String::from_utf8_lossy(&index);
            let head = section(&index, "Icon Theme");
            let get = |k: &str| head.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
            let mut subdirs: Vec<((i64, i64), &str)> = list(get("Directories"))
                .chain(list(get("ScaledDirectories")))
                .filter(|d| !d.split('/').any(|part| part == ".." || part.is_empty()))
                .take(MAX_DIRS)
                .map(|d| (distance(&section(&index, d)), d))
                .collect();
            subdirs.sort_by_key(|(key, _)| *key);
            // Not checked for existence: a theme lists hundreds, and the search stops at a hit.
            let listed = subdirs.iter().flat_map(|(_, sub)| roots.iter().map(move |r| r.join(sub))).collect();
            for parent in list(get("Inherits")) {
                if chain.len() < MAX_THEMES && !chain.iter().any(|t| t == parent) {
                    chain.push(parent.to_string());
                }
            }
            themes.push(Theme { roots, listed, unlisted: std::cell::OnceCell::new() });
        }
        (!themes.is_empty()).then_some(Self { themes })
    }

    /// The first `<icon>.svg` in search order: each theme's listed directories, then its unlisted
    /// ones, before its parent.
    fn find_svg(&self, icon: &str) -> Option<PathBuf> {
        if !valid_name(icon) {
            return None;
        }
        let file = format!("{icon}.svg");
        let hit = |dirs: &[PathBuf]| dirs.iter().map(|d| d.join(&file)).find(|p| p.is_file());
        self.themes.iter().find_map(|t| hit(&t.listed).or_else(|| hit(t.unlisted())))
    }
}

/// An SVG's coverage (alpha) as a `px` × `px` mask, the drawing fitted and centred. `None` for an
/// unreadable SVG or an absurd size.
fn rasterize_mask(svg: &[u8], px: u32) -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};
    if !(1..=256).contains(&px) {
        return None;
    }
    // Glyphs are paths: no `<image>` is loaded, so an icon can't make us read another file
    // (`/dev/zero`, a huge picture) or nest an SVG.
    let opts = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver { resolve_data: Box::new(|_, _, _| None), resolve_string: Box::new(|_, _| None) },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(svg, &opts).ok()?;
    let size = tree.size();
    let side = size.width().max(size.height());
    if !side.is_finite() || side <= 0.0 {
        return None;
    }
    let s = px as f32 / side;
    let (dx, dy) = ((px as f32 - size.width() * s) / 2.0, (px as f32 - size.height() * s) / 2.0);
    let mut pixmap = tiny_skia::Pixmap::new(px, px)?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(s, s).post_translate(dx, dy), &mut pixmap.as_mut());
    Some(pixmap.data().as_chunks::<4>().0.iter().map(|[_, _, _, a]| *a).collect())
}

impl CaptionIcons {
    /// The caption glyphs of icon theme `name` (`window-*-symbolic.svg`), read now and rendered at
    /// whatever size the bar asks for. `None` when the theme (with its parents) has none of them.
    pub fn from_icon_theme(name: &str) -> Option<CaptionIcons> {
        load_icons(name, &base_dirs(&|k: &str| std::env::var(k).ok()))
    }
}

fn load_icons(name: &str, bases: &[PathBuf]) -> Option<CaptionIcons> {
    let found = IconTheme::load(name, bases)?;
    let svg = |icon: &str| found.find_svg(icon).and_then(|p| read_small(&p)).map(Arc::<[u8]>::from);
    let files = [svg("window-minimize-symbolic"), svg("window-maximize-symbolic"), svg("window-restore-symbolic"), svg("window-close-symbolic")];
    if files.iter().all(Option::is_none) {
        return None;
    }
    let render = move |icon: CaptionIcon, px: u32| {
        let i = match icon {
            CaptionIcon::Minimize => 0,
            CaptionIcon::Maximize => 1,
            CaptionIcon::Restore => 2,
            CaptionIcon::Close => 3,
        };
        files.get(i)?.as_deref().and_then(|svg| rasterize_mask(svg, px))
    };
    Some(CaptionIcons { theme: name.to_string(), render: Arc::new(render) })
}

/// The icon theme the glyphs come from: the portal's answer, the desktop's setting, or Adwaita.
pub fn caption_icon_theme(portal: Option<&str>, desktop: Option<&str>) -> String {
    portal.or(desktop).unwrap_or("Adwaita").to_string()
}

/// [`CaptionIcons::from_icon_theme`] for the last theme asked for, kept, so a repeated request for
/// the same theme (the portal re-sends its setting) doesn't reread its files.
#[derive(Default)]
pub struct CaptionIconCache(std::sync::Mutex<Option<(String, Option<CaptionIcons>)>>);

impl CaptionIconCache {
    pub fn get(&self, theme: &str) -> Option<CaptionIcons> {
        let mut last = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((name, icons)) = last.as_ref()
            && name == theme
        {
            return icons.clone();
        }
        let icons = CaptionIcons::from_icon_theme(theme);
        *last = Some((theme.to_string(), icons.clone()));
        icons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway icon tree: `files` are (path under the base, contents).
    fn tree(tag: &str, files: &[(&str, &str)]) -> PathBuf {
        let base = std::env::temp_dir().join(format!("craft-ui-icons-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for (path, text) in files {
            let p = base.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        base
    }

    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M4 4h8v8H4z" fill="#2e3436"/></svg>"##;

    #[test]
    fn icons_come_from_the_theme_then_its_parents_then_hicolor_best_size_first() {
        let base = tree(
            "chain",
            &[
                (
                    "Child/index.theme",
                    "[Icon Theme]\nInherits=Parent, Child\nDirectories=48,16\n[48]\nSize=48\nType=Fixed\n[16]\nSize=16\nType=Fixed\n",
                ),
                ("Child/48/a-symbolic.svg", "big"),
                // Not in the index: still found, and before the parent's copy.
                ("Child/symbolic/ui/d-symbolic.svg", SQUARE),
                ("Parent/ui/d-symbolic.svg", SQUARE),
                ("Child/16/a-symbolic.svg", SQUARE),
                ("Parent/index.theme", "[Icon Theme]\nDirectories=ui\n[ui]\nSize=16\nMinSize=8\nMaxSize=512\nType=Scalable\n"),
                ("Parent/ui/b-symbolic.svg", SQUARE),
                ("hicolor/index.theme", "[Icon Theme]\nDirectories=s\n[s]\nSize=16\nType=Scalable\n"),
                ("hicolor/s/c-symbolic.svg", SQUARE),
            ],
        );
        let theme = IconTheme::load("Child", std::slice::from_ref(&base)).expect("the theme loads");
        assert_eq!(theme.find_svg("a-symbolic"), Some(base.join("Child/16/a-symbolic.svg")), "16 px before 48 px");
        assert_eq!(theme.find_svg("b-symbolic"), Some(base.join("Parent/ui/b-symbolic.svg")));
        assert_eq!(theme.find_svg("c-symbolic"), Some(base.join("hicolor/s/c-symbolic.svg")));
        assert_eq!(theme.find_svg("d-symbolic"), Some(base.join("Child/symbolic/ui/d-symbolic.svg")), "unlisted, but the theme's own");
        assert_eq!(theme.find_svg("missing"), None);
        assert_eq!(theme.find_svg("../Parent/ui/b-symbolic"), None);
        assert!(IconTheme::load("Nowhere", std::slice::from_ref(&base)).is_some(), "hicolor still answers");
        assert!(IconTheme::load("Nowhere", &[base.join("empty")]).is_none());
        let _ = std::fs::remove_dir_all(base);
    }

    /// Hostile theme files: cycles, escapes and junk sizes end the search, never hang or panic.
    #[test]
    fn cyclic_or_hostile_indexes_are_harmless() {
        let base = tree(
            "hostile",
            &[
                (
                    "A/index.theme",
                    "[Icon Theme]\nInherits=B,../etc,\nDirectories=../../x,,/abs,ok\n[ok]\nSize=99999999999999999999\nScale=-3\nType=Threshold\n",
                ),
                ("A/ok/x.svg", SQUARE),
                ("B/index.theme", "[Icon Theme]\nInherits=A\n"),
            ],
        );
        let theme = IconTheme::load("A", std::slice::from_ref(&base)).expect("loads");
        assert_eq!(theme.find_svg("x"), Some(base.join("A/ok/x.svg")));
        assert!(IconTheme::load("..", std::slice::from_ref(&base)).is_none());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn distance_follows_the_spec() {
        assert_eq!(distance(&[("Size", "16"), ("Type", "Fixed")]), (0, 1));
        assert_eq!(distance(&[("Size", "22"), ("Type", "Fixed")]), (6, 1));
        assert_eq!(distance(&[("Size", "16"), ("Scale", "2"), ("Type", "Fixed")]), (16, 2));
        assert_eq!(distance(&[("Size", "32"), ("MinSize", "8"), ("MaxSize", "512"), ("Type", "Scalable")]), (0, 1));
        assert_eq!(distance(&[("Size", "48"), ("Type", "Scalable")]), (32, 1));
        assert_eq!(distance(&[("Size", "18")]), (0, 1), "Threshold 2 by default");
    }

    #[test]
    fn masks_are_square_coverage() {
        let mask = rasterize_mask(SQUARE.as_bytes(), 32).expect("renders");
        assert_eq!(mask.len(), 32 * 32);
        assert_eq!(mask[16 * 32 + 16], 255, "inside the square");
        assert_eq!(mask[0], 0, "outside it");
        assert!(rasterize_mask(b"not svg", 16).is_none());
        assert!(rasterize_mask(SQUARE.as_bytes(), 0).is_none());
        assert!(rasterize_mask(SQUARE.as_bytes(), 100_000).is_none());
    }

    /// An icon's `<image>` is never loaded: not a device file, not an embedded picture.
    #[test]
    fn masks_load_no_images() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="16" height="16">
            <image href="/dev/zero" width="16" height="16"/>
            <image xlink:href="data:image/svg+xml;utf8,&lt;svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'&gt;&lt;rect width='16' height='16'/&gt;&lt;/svg&gt;" width="16" height="16"/>
        </svg>"##;
        let mask = rasterize_mask(svg.as_bytes(), 16).expect("renders");
        assert!(mask.iter().all(|a| *a == 0), "nothing drawn");
    }

    #[test]
    fn base_dirs_follow_xdg() {
        let var = |k: &str| match k {
            "HOME" => Some("/home/u".to_string()),
            "XDG_DATA_DIRS" => Some("/a:rel:/b".to_string()),
            _ => None,
        };
        let dirs = base_dirs(&var);
        let want: Vec<PathBuf> = ["/home/u/.icons", "/home/u/.local/share/icons", "/a/icons", "/b/icons"].iter().map(PathBuf::from).collect();
        assert_eq!(dirs, want);
        assert_eq!(base_dirs(&|_| None), vec![PathBuf::from("/usr/local/share/icons"), PathBuf::from("/usr/share/icons")]);
    }

    #[test]
    fn caption_icons_render_the_themes_glyph() {
        let base = tree(
            "caption",
            &[("T/index.theme", "[Icon Theme]\nDirectories=s\n[s]\nSize=16\nType=Fixed\n"), ("T/s/window-close-symbolic.svg", SQUARE)],
        );
        let icons = load_icons("T", std::slice::from_ref(&base)).expect("the theme has a close glyph");
        assert_eq!(icons.theme, "T");
        let mask = (icons.render)(CaptionIcon::Close, 16).expect("renders");
        assert!(mask.iter().any(|a| *a > 0));
        assert!((icons.render)(CaptionIcon::Minimize, 16).is_none(), "the theme lacks it");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn theme_name_order_is_portal_then_desktop_then_adwaita() {
        assert_eq!(caption_icon_theme(Some("A"), Some("B")), "A");
        assert_eq!(caption_icon_theme(None, Some("B")), "B");
        assert_eq!(caption_icon_theme(None, None), "Adwaita");
    }
}
