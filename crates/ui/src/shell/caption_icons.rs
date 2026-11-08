//! The caption glyphs of a desktop icon theme (the freedesktop Icon Theme Specification). A theme is searched, then
//! the themes it inherits, then `hicolor`; within each, its directories from the best fit for a 16 px icon down. Only
//! SVG files are used: symbolic icons are drawn as a coverage mask in the button's ink colour.
//!
//! The search reads no file itself. It asks a [`ThemeFiles`], the seam a platform service implements, so the pure
//! parts (index parsing, inheritance, name lookup, rasterising) are the same on every system and run on in-memory
//! trees.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::primitives::svg_tree;

/// Bounds on what a theme's files can make the search do: themes in an inheritance chain, directories a theme lists,
/// and the bytes read from one file.
const MAX_THEMES: usize = 32;
const MAX_DIRS: usize = 4096;
/// How deep an index's unlisted directories are looked for (`symbolic/ui`, `16x16@2/actions`).
const SCAN_DEPTH: usize = 3;
/// The most a [`ThemeFiles::read`] returns.
pub const MAX_FILE: u64 = 1 << 20;
/// File probes one theme load makes while looking for the four icons, over all themes of the chain. A theme listing
/// thousands of directories hits it and gets the family's own glyphs.
const MAX_PROBES: usize = 20_000;
/// The largest side a mask is rendered at, in device pixels.
const MAX_MASK_SIDE: u32 = 256;
/// The size the caption glyphs are drawn at, in the spec's units.
const ICON_SIZE: i64 = 16;
/// The largest number an index's size keys may hold.
const MAX_INDEX_NUMBER: i64 = 1 << 16;
/// The threshold a `Threshold` directory has when its index gives none.
const DEFAULT_THRESHOLD: i64 = 2;
/// The theme the glyphs come from when neither the portal nor the desktop names one.
const FALLBACK_THEME: &str = "Adwaita";

/// A caption glyph, as icon themes name them (`window-restore-symbolic`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CaptionIcon {
    /// The minimize glyph.
    Minimize,
    /// The maximize glyph.
    Maximize,
    /// The restore glyph, shown while the window is maximized.
    Restore,
    /// The close glyph.
    Close,
}

impl CaptionIcon {
    /// Every glyph, in the order [`CaptionIcons::from_files`] takes its files.
    pub const ALL: [CaptionIcon; 4] = [CaptionIcon::Minimize, CaptionIcon::Maximize, CaptionIcon::Restore, CaptionIcon::Close];

    /// The icon name in a theme.
    pub const fn theme_name(self) -> &'static str {
        match self {
            CaptionIcon::Minimize => "window-minimize-symbolic",
            CaptionIcon::Maximize => "window-maximize-symbolic",
            CaptionIcon::Restore => "window-restore-symbolic",
            CaptionIcon::Close => "window-close-symbolic",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|icon| *icon == self).unwrap_or_default()
    }
}

/// A coverage mask: `side` x `side` bytes, row by row.
pub type CaptionMask = Arc<[u8]>;

/// Renders a glyph as a square coverage mask at a pixel side, or `None`.
pub type RenderIconFn = dyn Fn(CaptionIcon, u32) -> Option<CaptionMask> + Send + Sync;

/// The round buttons' glyphs from the desktop's icon theme (Linux: `window-*-symbolic`), drawn in the glyph colour
/// as symbolic icons are. A glyph the theme lacks draws the family's own.
#[derive(Clone)]
pub struct CaptionIcons {
    theme: String,
    generation: u64,
    render: Arc<RenderIconFn>,
}

impl CaptionIcons {
    /// Icons of `theme` rendered by `render`. `generation` changes whenever `render` may answer differently (a
    /// reinstalled theme, a size that finished rendering); the textures made from it follow. Use 0 for icons that
    /// never change.
    pub fn new(theme: impl Into<String>, generation: u64, render: Arc<RenderIconFn>) -> Self {
        CaptionIcons { theme: theme.into(), generation, render }
    }

    /// Icons rendered from the four SVG files of a theme, in [`CaptionIcon::ALL`] order; `None` where the theme lacks
    /// one. The masks are kept per glyph and size.
    pub fn from_files(theme: impl Into<String>, generation: u64, files: [Option<Arc<[u8]>>; 4]) -> Self {
        type Memo = Mutex<HashMap<(CaptionIcon, u32), Option<CaptionMask>>>;
        let memo: Memo = Mutex::default();
        let render = move |icon: CaptionIcon, px: u32| {
            let mut memo = memo.lock().unwrap_or_else(PoisonError::into_inner);
            memo.entry((icon, px)).or_insert_with(|| files[icon.index()].as_deref().and_then(|svg| rasterize_mask(svg, px))).clone()
        };
        CaptionIcons::new(theme, generation, Arc::new(render))
    }

    /// The caption glyphs of icon theme `name` (`window-*-symbolic.svg`), searched through `files` in `bases`. `None`
    /// when the theme, its parents and `hicolor` have none of them.
    pub fn from_theme(name: &str, bases: &[PathBuf], files: &dyn ThemeFiles) -> Option<CaptionIcons> {
        let chain = IconThemeChain::load(name, bases, files)?;
        let svgs = CaptionIcon::ALL.map(|icon| chain.find_svg(icon.theme_name()).and_then(|path| files.read(&path, MAX_FILE)).map(Arc::<[u8]>::from));
        svgs.iter().any(Option::is_some).then(|| CaptionIcons::from_files(name, 0, svgs))
    }

    /// The theme's name.
    pub fn theme(&self) -> &str {
        &self.theme
    }

    /// Changes whenever [`mask`](Self::mask) may answer differently.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The coverage mask of `icon` at `px` device pixels, or `None` while the theme lacks it or it does not render.
    pub fn mask(&self, icon: CaptionIcon, px: u32) -> Option<CaptionMask> {
        (self.render)(icon, px)
    }
}

impl PartialEq for CaptionIcons {
    fn eq(&self, other: &Self) -> bool {
        self.theme == other.theme && self.generation == other.generation && Arc::ptr_eq(&self.render, &other.render)
    }
}

impl std::fmt::Debug for CaptionIcons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptionIcons").field("theme", &self.theme).field("generation", &self.generation).finish_non_exhaustive()
    }
}

/// The icon theme the glyphs come from: the portal's answer, the desktop's setting, or Adwaita. The portal's answer
/// is the GNOME key, which the appearance watcher reports on GNOME-like desktops only; elsewhere the desktop's own
/// setting is the one that counts.
pub fn caption_icon_theme(portal: Option<&str>, desktop: Option<&str>) -> String {
    portal.or(desktop).unwrap_or(FALLBACK_THEME).to_string()
}

/// What the theme search asks of the file system. An implementation answers for regular files and directories only,
/// and a path it cannot read is simply absent.
pub trait ThemeFiles {
    /// Whether `path` is a directory.
    fn is_dir(&self, path: &Path) -> bool;
    /// Whether `path` is a regular file. A FIFO or device is none: opening it could block.
    fn is_file(&self, path: &Path) -> bool;
    /// The directories directly inside `dir`; the search looks at the first few thousand.
    fn subdirectories(&self, dir: &Path) -> Vec<PathBuf>;
    /// At most `limit` bytes of the regular file `path`.
    fn read(&self, path: &Path, limit: u64) -> Option<Vec<u8>>;
}

/// Where themes live, in the spec's order: `~/.icons`, `$XDG_DATA_HOME/icons`, then each `$XDG_DATA_DIRS/icons`.
/// Relative paths in the variables are ignored, as the XDG spec says.
pub fn base_dirs(var: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
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

/// A theme or icon name that stays inside its directory.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// The `[sections]` of an `index.theme` as (key, value) pairs, parsed once. A section named twice keeps its first
/// body.
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

/// How far a theme directory is from a 16 px icon at scale 1 (the spec's `DirectorySizeDistance`), then its scale:
/// 0 is an exact fit.
fn distance(keys: &[(&str, &str)]) -> (i64, i64) {
    let get = |k: &str| keys.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
    let num = |k: &str, default: i64| get(k).and_then(|v| v.parse::<i64>().ok()).unwrap_or(default).clamp(0, MAX_INDEX_NUMBER);
    let size = num("Size", 0);
    let scale = num("Scale", 1).max(1);
    let d = match get("Type").unwrap_or("Threshold") {
        "Fixed" => (size * scale - ICON_SIZE).abs(),
        "Scalable" => {
            let (min, max) = (num("MinSize", size) * scale, num("MaxSize", size) * scale);
            (min - ICON_SIZE).max(ICON_SIZE - max).max(0)
        }
        _ => ((size * scale - ICON_SIZE).abs() - num("Threshold", DEFAULT_THRESHOLD) * scale).max(0),
    };
    (d, scale)
}

/// Every directory under `root`, at most [`SCAN_DEPTH`] levels down and [`MAX_DIRS`] in all, in path order: scalable
/// and symbolic ones first, as icons there fit any size.
fn subdirectories(files: &dyn ThemeFiles, root: &Path) -> Vec<PathBuf> {
    let mut found = vec![];
    let mut level = vec![root.to_path_buf()];
    for _ in 0..SCAN_DEPTH {
        let mut next = vec![];
        for dir in &level {
            for path in files.subdirectories(dir).into_iter().take(MAX_DIRS) {
                if found.len() + next.len() < MAX_DIRS {
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

/// One theme of a chain: its listed directories, best fit first, in every base directory that has the theme, and
/// (looked for only when an icon is not in those) the directories its index forgot.
struct Theme {
    roots: Vec<PathBuf>,
    listed: Vec<PathBuf>,
    unlisted: std::cell::OnceCell<Vec<PathBuf>>,
}

impl Theme {
    /// The directories the index forgot to list: GTK still finds icons there, and custom themes often add
    /// `symbolic/ui` without listing it.
    fn unlisted(&self, files: &dyn ThemeFiles) -> &[PathBuf] {
        self.unlisted.get_or_init(|| {
            let listed: std::collections::HashSet<&PathBuf> = self.listed.iter().collect();
            self.roots.iter().flat_map(|root| subdirectories(files, root)).filter(|dir| !listed.contains(dir)).collect()
        })
    }
}

/// A theme and the themes it inherits, in search order.
pub struct IconThemeChain<'a> {
    themes: Vec<Theme>,
    files: &'a dyn ThemeFiles,
    /// Probes left for [`find_svg`](Self::find_svg), over all icons of a load.
    probes: std::cell::Cell<usize>,
}

impl<'a> IconThemeChain<'a> {
    /// `name` and the themes it inherits, then `hicolor`, looked up in `bases`. `None` when no theme in the chain has
    /// an `index.theme`.
    pub fn load(name: &str, bases: &[PathBuf], files: &'a dyn ThemeFiles) -> Option<Self> {
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
            let roots: Vec<PathBuf> = bases.iter().map(|base| base.join(&theme)).filter(|root| files.is_dir(root)).collect();
            // The first index wins; a theme can still be spread over several base directories.
            let Some(index) = roots.iter().find_map(|root| files.read(&root.join("index.theme"), MAX_FILE)) else { continue };
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
            let listed = subdirs.iter().flat_map(|(_, sub)| roots.iter().map(move |root| root.join(sub))).collect();
            for parent in list(get("Inherits")) {
                if chain.len() < MAX_THEMES && !chain.iter().any(|t| t == parent) {
                    chain.push(parent.to_string());
                }
            }
            themes.push(Theme { roots, listed, unlisted: std::cell::OnceCell::new() });
        }
        (!themes.is_empty()).then_some(Self { themes, files, probes: std::cell::Cell::new(MAX_PROBES) })
    }

    /// The first `<icon>.svg` in search order: each theme's listed directories, then its unlisted ones, before its
    /// parent.
    pub fn find_svg(&self, icon: &str) -> Option<PathBuf> {
        if !valid_name(icon) {
            return None;
        }
        let file = format!("{icon}.svg");
        let hit = |dirs: &[PathBuf]| {
            dirs.iter().map(|dir| dir.join(&file)).find(|path| {
                let Some(left) = self.probes.get().checked_sub(1) else { return false };
                self.probes.set(left);
                self.files.is_file(path)
            })
        };
        self.themes.iter().find_map(|theme| hit(&theme.listed).or_else(|| hit(theme.unlisted(self.files))))
    }
}

/// An SVG's coverage (alpha) as a `px` x `px` mask, the drawing fitted and centred. `None` for an unreadable SVG or
/// an absurd size.
fn rasterize_mask(svg: &[u8], px: u32) -> Option<CaptionMask> {
    use resvg::tiny_skia;
    if !(1..=MAX_MASK_SIDE).contains(&px) {
        return None;
    }
    let tree = svg_tree::parse(svg).ok()?;
    let size = tree.size();
    let side = size.width().max(size.height());
    if !side.is_finite() || side <= 0.0 {
        return None;
    }
    let scale = px as f32 / side;
    let (dx, dy) = ((px as f32 - size.width() * scale) / 2.0, (px as f32 - size.height() * scale) / 2.0);
    let mut pixmap = tiny_skia::Pixmap::new(px, px)?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale).post_translate(dx, dy), &mut pixmap.as_mut());
    Some(pixmap.data().as_chunks::<4>().0.iter().map(|[_, _, _, a]| *a).collect())
}
