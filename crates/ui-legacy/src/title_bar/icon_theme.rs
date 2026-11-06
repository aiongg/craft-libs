//! Linux: the caption glyphs from the desktop's freedesktop icon theme (the Icon Theme
//! Specification). A theme is searched, then the themes it inherits, then `hicolor`; within each,
//! its directories from the best fit for a 16 px icon down. Only SVG files are used: symbolic
//! icons are drawn as a coverage mask in the button's ink colour.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use super::{CaptionIcon, CaptionIcons};

/// Bounds on what a theme's files can make us do: themes in an inheritance chain, directories a
/// theme lists, and the bytes read from one file.
const MAX_THEMES: usize = 32;
const MAX_DIRS: usize = 4096;
/// How deep an index's unlisted directories are looked for (`symbolic/ui`, `16x16@2/actions`).
const SCAN_DEPTH: usize = 3;
const MAX_FILE: u64 = 1 << 20;
/// File-existence probes one theme load makes while looking for the four icons, over all themes
/// of the chain. A theme listing thousands of directories hits it and gets the family's glyphs.
const MAX_PROBES: usize = 20_000;
/// Elements and text nodes an icon SVG may have; a glyph has a handful.
const MAX_SVG_NODES: u32 = 10_000;
/// The size the caption glyphs are drawn at, in the spec's units.
const ICON_SIZE: i64 = 16;

/// A theme or icon name that stays inside its directory.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// At most [`MAX_FILE`] bytes of a regular file (settings, theme indexes and icons are small).
/// A FIFO or device in a theme directory would block `open` or never end, so it is not opened.
fn read_small(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    if !std::fs::metadata(path).ok()?.is_file() {
        return None;
    }
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

/// The `[sections]` of an `index.theme` as (key, value) pairs, parsed once. A section named
/// twice keeps its first body.
fn sections(index: &str) -> HashMap<&str, Vec<(&str, &str)>> {
    let mut map: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    let mut current: Option<&str> = None;
    for line in index.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = if map.contains_key(name) { None } else { Some(name) };
            if let Some(name) = current {
                map.insert(name, vec![]);
            }
        } else if let Some(name) = current
            && let Some((k, v)) = line.split_once('=')
            && let Some(pairs) = map.get_mut(name)
        {
            pairs.push((k.trim(), v.trim()));
        }
    }
    map
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
    /// Probes left for [`IconTheme::find_svg`], over all four icons of a load.
    probes: std::cell::Cell<usize>,
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
            let sections = sections(&index);
            let head = sections.get("Icon Theme").map(Vec::as_slice).unwrap_or_default();
            let get = |k: &str| head.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
            let mut subdirs: Vec<((i64, i64), &str)> = list(get("Directories"))
                .chain(list(get("ScaledDirectories")))
                .filter(|d| !d.split('/').any(|part| part == ".." || part.is_empty()))
                .take(MAX_DIRS)
                .map(|d| (distance(sections.get(d).map(Vec::as_slice).unwrap_or_default()), d))
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
        (!themes.is_empty()).then_some(Self { themes, probes: std::cell::Cell::new(MAX_PROBES) })
    }

    /// The first `<icon>.svg` in search order: each theme's listed directories, then its unlisted
    /// ones, before its parent.
    fn find_svg(&self, icon: &str) -> Option<PathBuf> {
        if !valid_name(icon) {
            return None;
        }
        let file = format!("{icon}.svg");
        let hit = |dirs: &[PathBuf]| {
            dirs.iter().map(|d| d.join(&file)).find(|p| {
                let Some(left) = self.probes.get().checked_sub(1) else { return false };
                self.probes.set(left);
                p.is_file()
            })
        };
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
    // `Tree::from_data` would gunzip with no size limit and allow DTDs, so parse the XML here.
    if svg.starts_with(&[0x1f, 0x8b]) {
        return None;
    }
    let xml = roxmltree::ParsingOptions { allow_dtd: false, nodes_limit: MAX_SVG_NODES };
    let doc = roxmltree::Document::parse_with_options(std::str::from_utf8(svg).ok()?, xml).ok()?;
    let tree = usvg::Tree::from_xmltree(&doc, &opts).ok()?;
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

const ICONS: [CaptionIcon; 4] = [CaptionIcon::Minimize, CaptionIcon::Maximize, CaptionIcon::Restore, CaptionIcon::Close];
type Files = [Option<Arc<[u8]>>; 4];

impl CaptionIcons {
    /// The caption glyphs of icon theme `name` (`window-*-symbolic.svg`), read now and rendered at
    /// whatever size the bar asks for, on the calling thread. `None` when the theme (with its
    /// parents) has none of them. A frame loop uses [`CaptionIconCache`] instead.
    pub fn from_icon_theme(name: &str) -> Option<CaptionIcons> {
        load_icons(name, &base_dirs(&|k: &str| std::env::var(k).ok()))
    }
}

/// The four glyph files of theme `name`, in [`ICONS`] order. `None` when it has none of them.
fn load_files(name: &str, bases: &[PathBuf]) -> Option<Files> {
    let found = IconTheme::load(name, bases)?;
    let svg = |icon: &str| found.find_svg(icon).and_then(|p| read_small(&p)).map(Arc::<[u8]>::from);
    let files = [svg("window-minimize-symbolic"), svg("window-maximize-symbolic"), svg("window-restore-symbolic"), svg("window-close-symbolic")];
    files.iter().any(Option::is_some).then_some(files)
}

fn index_of(icon: CaptionIcon) -> usize {
    ICONS.iter().position(|i| *i == icon).unwrap_or_default()
}

fn load_icons(name: &str, bases: &[PathBuf]) -> Option<CaptionIcons> {
    let files = load_files(name, bases)?;
    let render = move |icon: CaptionIcon, px: u32| files.get(index_of(icon))?.as_deref().and_then(|svg| rasterize_mask(svg, px));
    Some(CaptionIcons { theme: name.to_string(), generation: 0, render: Arc::new(render) })
}

/// The glyph files of one loaded theme and the masks rendered from them, per size.
struct Masks {
    files: Files,
    sizes: Mutex<MaskSizes>,
    generation: Arc<AtomicU64>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

#[derive(Default)]
struct MaskSizes {
    done: HashMap<u32, Arc<[Option<Vec<u8>>; 4]>>,
    pending: HashSet<u32>,
}

impl Masks {
    /// The mask of `icon` at `px`, or `None` while the size is still rendering (a background
    /// thread renders all four icons of a size once, bumps the generation and wakes the host).
    fn get(self: &Arc<Self>, icon: CaptionIcon, px: u32) -> Option<Vec<u8>> {
        let mut sizes = self.sizes.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(done) = sizes.done.get(&px) {
            return done[index_of(icon)].clone();
        }
        if sizes.pending.insert(px) {
            let this = Arc::clone(self);
            let started = std::thread::Builder::new().name("caption-icon-masks".into()).spawn(move || {
                let masks = this.files.each_ref().map(|f| f.as_deref().and_then(|svg| rasterize_mask(svg, px)));
                let mut sizes = this.sizes.lock().unwrap_or_else(PoisonError::into_inner);
                sizes.pending.remove(&px);
                sizes.done.insert(px, Arc::new(masks));
                drop(sizes);
                this.generation.fetch_add(1, Ordering::AcqRel);
                (this.wake)();
            });
            if started.is_err() {
                sizes.pending.remove(&px);
                sizes.done.insert(px, Arc::new([None, None, None, None]));
            }
        }
        None
    }
}

/// The icon theme the glyphs come from: the portal's answer, the desktop's setting, or Adwaita.
/// The portal's answer is the GNOME key, which the watcher reports on GNOME-like desktops only
/// (`craft_platform::appearance::Watcher::icon_theme`); elsewhere the desktop's own setting
/// (`craft_window::Desktop::icon_theme`) is the one that counts.
pub fn caption_icon_theme(portal: Option<&str>, desktop: Option<&str>) -> String {
    portal.or(desktop).unwrap_or("Adwaita").to_string()
}

#[derive(Default)]
struct CacheState {
    /// The theme last asked for, loaded or loading.
    wanted: Option<String>,
    /// How many loads were started; a finished load that is not the latest is dropped.
    loads: u64,
    /// The last finished load: its name and masks.
    loaded: Option<(String, Arc<Masks>)>,
}

/// [`CaptionIcons::from_icon_theme`] for the last theme asked for, kept, and loaded on a
/// background thread: [`get`](Self::get) never reads a file or renders on the caller's thread.
///
/// Asked for a new theme, it starts the load and returns the previous theme's icons (or `None`)
/// until the load is done, then repaints the context. A size's masks render the same way:
/// the first draw at a size shows the family's own glyphs, and the context repaints when the
/// theme's are ready. [`CaptionIcons::generation`] changes with each of those steps.
#[derive(Default)]
pub struct CaptionIconCache {
    state: Arc<Mutex<CacheState>>,
    generation: Arc<AtomicU64>,
}

impl CaptionIconCache {
    /// The icons of `theme` once loaded, the previous theme's (or `None`) until then.
    pub fn get(&self, theme: &str, ctx: &egui::Context) -> Option<CaptionIcons> {
        self.get_with(theme, || {
            let ctx = ctx.clone();
            (base_dirs(&|k: &str| std::env::var(k).ok()), Arc::new(move || ctx.request_repaint()))
        })
    }

    /// [`get`](Self::get) with the search directories and the wake-up supplied by `start`, which
    /// runs only when a load begins.
    fn get_with(&self, theme: &str, start: impl FnOnce() -> (Vec<PathBuf>, Arc<dyn Fn() + Send + Sync>)) -> Option<CaptionIcons> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.wanted.as_deref() != Some(theme) {
            state.wanted = Some(theme.to_string());
            state.loads += 1;
            let id = state.loads;
            let (bases, wake) = start();
            let (shared, generation, name) = (Arc::clone(&self.state), Arc::clone(&self.generation), theme.to_string());
            let spawned = std::thread::Builder::new().name("caption-icon-theme".into()).spawn(move || {
                let files = load_files(&name, &bases);
                let mut state = shared.lock().unwrap_or_else(PoisonError::into_inner);
                if state.loads != id {
                    return;
                }
                let found = files.is_some();
                let masks =
                    Masks { files: files.unwrap_or_default(), sizes: Mutex::default(), generation: Arc::clone(&generation), wake: Arc::clone(&wake) };
                state.loaded = Some(name).zip(found.then(|| Arc::new(masks)));
                drop(state);
                generation.fetch_add(1, Ordering::AcqRel);
                wake();
            });
            if spawned.is_err() {
                state.loaded = None;
            }
        }
        let (name, masks) = state.loaded.as_ref()?;
        let masks = Arc::clone(masks);
        Some(CaptionIcons {
            theme: name.clone(),
            generation: self.generation.load(Ordering::Acquire),
            render: Arc::new(move |icon, px| masks.get(icon, px)),
        })
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

    /// A gzip stream's start: the data after it is never looked at.
    fn gzip(rest: &[u8]) -> Vec<u8> {
        [&[0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff][..], rest].concat()
    }

    #[test]
    fn hostile_svgs_are_refused() {
        assert!(rasterize_mask(&gzip(SQUARE.as_bytes()), 16).is_none(), "gzip is never expanded");
        assert!(rasterize_mask(&gzip(&[0; 64]), 16).is_none());
        let dtd = r##"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "x">]><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M4 4h8v8H4z"/></svg>"##;
        assert!(rasterize_mask(dtd.as_bytes(), 16).is_none());
        let many = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">{}</svg>"#, "<g/>".repeat(20_000));
        assert!(rasterize_mask(many.as_bytes(), 16).is_none());
    }

    #[test]
    fn a_fifo_index_is_not_opened() {
        let base = tree("fifo", &[("T/s/window-close-symbolic.svg", SQUARE)]);
        let fifo = base.join("T/index.theme");
        assert!(std::process::Command::new("mkfifo").arg(&fifo).status().unwrap().success());
        // Opening it for reading would block until a writer appears.
        assert!(read_small(&fifo).is_none());
        assert!(load_icons("T", std::slice::from_ref(&base)).is_none());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn the_index_is_parsed_once_into_sections() {
        let map = sections("junk=1\n[A]\nx = 1\n[B]\ny=2\n[A]\nz=3\n");
        assert_eq!(map.get("A").map(Vec::as_slice), Some(&[("x", "1")][..]), "the first body of a repeated section");
        assert_eq!(map.get("B").map(Vec::as_slice), Some(&[("y", "2")][..]));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn probes_are_capped_per_load() {
        let base = tree("probes", &[("T/index.theme", "[Icon Theme]\nDirectories=s\n[s]\nSize=16\n"), ("T/s/a.svg", SQUARE)]);
        let theme = IconTheme::load("T", std::slice::from_ref(&base)).unwrap();
        assert!(theme.find_svg("a").is_some());
        theme.probes.set(0);
        assert!(theme.find_svg("a").is_none(), "no probes left");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn the_cache_shows_the_old_icons_until_the_new_theme_has_loaded() {
        let base = tree(
            "cache",
            &[
                ("T/index.theme", "[Icon Theme]\nDirectories=s\n[s]\nSize=16\nType=Fixed\n"),
                ("T/s/window-close-symbolic.svg", SQUARE),
                ("U/index.theme", "[Icon Theme]\nDirectories=s\n[s]\nSize=16\nType=Fixed\n"),
                ("U/s/window-close-symbolic.svg", SQUARE),
            ],
        );
        let cache = CaptionIconCache::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let start = |tx: std::sync::mpsc::Sender<()>| {
            let bases = vec![base.clone()];
            move || {
                (
                    bases,
                    Arc::new(move || {
                        let _ = tx.send(());
                    }) as Arc<dyn Fn() + Send + Sync>,
                )
            }
        };
        assert!(cache.get_with("T", start(tx.clone())).is_none(), "nothing before the first load ends");
        rx.recv().unwrap();
        let t = cache.get_with("T", start(tx.clone())).expect("T loaded");
        assert_eq!(t.theme, "T");
        assert!(cache.get_with("T", || panic!("the same theme starts no load")).is_some());
        // A size renders in the background: nothing at first, the mask after the wake-up.
        assert!((t.render)(CaptionIcon::Close, 16).is_none());
        rx.recv().unwrap();
        let t2 = cache.get_with("T", || panic!("no load")).unwrap();
        assert_ne!(t2.generation, t.generation);
        assert!((t2.render)(CaptionIcon::Close, 16).is_some_and(|m| m.iter().any(|a| *a > 0)));
        assert!((t2.render)(CaptionIcon::Minimize, 16).is_none(), "the theme lacks it");
        // A new theme: T's icons until U has loaded.
        let during = cache.get_with("U", start(tx.clone())).expect("the old icons");
        assert_eq!(during.theme, "T");
        rx.recv().unwrap();
        assert_eq!(cache.get_with("U", || panic!("no load")).unwrap().theme, "U");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn theme_name_order_is_portal_then_desktop_then_adwaita() {
        assert_eq!(caption_icon_theme(Some("A"), Some("B")), "A");
        assert_eq!(caption_icon_theme(None, Some("B")), "B");
        assert_eq!(caption_icon_theme(None, None), "Adwaita");
    }
}
