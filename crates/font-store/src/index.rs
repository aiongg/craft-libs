//! The Google Fonts index (aiongg/google-fonts-index, format 1.x): reading, checks and exact
//! lookup.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

/// The largest index read.
pub const MAX_INDEX_BYTES: usize = 32 << 20;
/// The largest font file an index may list.
pub const MAX_FILE_BYTES: u64 = 256 << 20;
/// The index format's major version this crate reads.
pub const MAJOR_VERSION: u32 = 1;

const MAX_STR: usize = 1024;
const MAX_LIST: usize = 4096;
const MAX_FAMILIES: usize = 50_000;
const MAX_FILES: usize = 200_000;
const MAX_DIR: usize = 64;
const MAX_FILE_NAME: usize = 200;

/// Why an index can't be used.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// More than [`MAX_INDEX_BYTES`].
    #[error("the font index is larger than {max} bytes")]
    TooLarge {
        /// The limit.
        max: usize,
    },
    /// Not JSON, or a field has the wrong type.
    #[error("the font index can't be read: {0}")]
    Json(String),
    /// A major version this crate doesn't know.
    #[error("the font index has version {0}; this build reads version {MAJOR_VERSION}.x")]
    Version(String),
    /// An entry breaks the format's rules.
    #[error("the font index has an invalid entry: {0}")]
    Invalid(String),
    /// A gzipped index that doesn't decompress.
    #[error("the gzipped font index can't be read: {0}")]
    Gzip(String),
    /// The bytes aren't the pinned index.
    #[error("the font index doesn't have the sha256 this build expects")]
    Checksum,
}

/// A language some family supports.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Language {
    /// gflanguages ID: `<ISO 639>_<ISO 15924>`, sometimes with `_<region>` (`vi_Latn`).
    pub id: String,
    /// English name.
    pub name: String,
    /// The name in the language itself.
    #[serde(default)]
    pub autonym: Option<String>,
    /// ISO 15924 code.
    #[serde(default)]
    pub script: String,
    /// Speakers; 0 when unknown.
    #[serde(default)]
    pub population: u64,
    /// A historical language.
    #[serde(default)]
    pub historical: bool,
}

/// A script some family uses.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Script {
    /// ISO 15924 code (`Zmth`, `Zsym` and `Zsye` for maths, symbols and emoji).
    pub id: String,
    /// English name.
    pub name: String,
}

/// A variation axis of a font file.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Axis {
    /// The axis tag (`wght`).
    pub tag: String,
    /// The smallest value.
    pub min: f32,
    /// The largest value.
    pub max: f32,
}

/// One font file of a family.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct FontFile {
    /// Path in google/fonts, relative to the index's base URL: `<licence dir>/<family dir>/<file>`.
    pub path: String,
    /// Bytes.
    pub size: u64,
    /// 64 lowercase hex characters.
    pub sha256: String,
    /// `normal` or `italic`.
    #[serde(default = "normal")]
    pub style: String,
    /// CSS weight; for a variable font, the default instance's.
    pub weight: u16,
    /// Variation axes; empty for a static font.
    #[serde(default)]
    pub axes: Vec<Axis>,
    /// PostScript names: the face's and, for a variable font, each named instance's.
    #[serde(default)]
    pub postscript: Vec<String>,
}

fn normal() -> String {
    "normal".into()
}

impl FontFile {
    /// The file's name, the last part of its path.
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Whether the file's faces are italic.
    pub fn is_italic(&self) -> bool {
        self.style.eq_ignore_ascii_case("italic")
    }

    /// Whether it is a variable font.
    pub fn is_variable(&self) -> bool {
        !self.axes.is_empty()
    }

    fn axis(&self, tag: &str) -> Option<&Axis> {
        self.axes.iter().find(|a| a.tag == tag)
    }

    /// Whether the file has the face of CSS `weight`, italic or not: its own weight, or a weight
    /// its `wght` axis reaches.
    pub fn has_weight(&self, weight: u16, italic: bool) -> bool {
        let italic_ok = self.is_italic() == italic || self.axis("ital").is_some_and(|a| a.min <= 0.0 && a.max >= 1.0);
        let w = f32::from(weight);
        italic_ok && (self.weight == weight || self.axis("wght").is_some_and(|a| a.min <= w && w <= a.max))
    }
}

/// A font family of the index.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Family {
    /// The family name, unique in the index ignoring case.
    pub family: String,
    /// The designer; empty when not given.
    #[serde(default)]
    pub designer: String,
    /// `SANS_SERIF`, `SERIF`, `DISPLAY`, `HANDWRITING` or `MONOSPACE`.
    #[serde(default)]
    pub categories: Vec<String>,
    /// google/fonts subsets (`latin`, `japanese`, ...).
    #[serde(default)]
    pub subsets: Vec<String>,
    /// Positions in [`Index::languages`].
    #[serde(default)]
    pub languages: Vec<usize>,
    /// ISO 15924 codes.
    #[serde(default)]
    pub scripts: Vec<String>,
    /// `OFL`, `APACHE2` or `UFL`.
    #[serde(default)]
    pub license: String,
    /// The licence file, relative to the base URL, in the same folder as the font files.
    #[serde(default)]
    pub license_path: Option<String>,
    /// The font files, sorted by path.
    pub files: Vec<FontFile>,
    /// The family's folder in google/fonts, and in the font store.
    #[serde(skip)]
    dir: String,
}

impl Family {
    /// The family's folder name (`notosansjp`): its folder in google/fonts and in the font store.
    pub fn dir(&self) -> &str {
        &self.dir
    }

    /// The licence file's name (`OFL.txt`).
    pub fn license_file_name(&self) -> Option<&str> {
        self.license_path.as_deref().and_then(|p| p.rsplit('/').next())
    }

    /// The file with exactly this style: a style name (`Bold Italic`, `SemiBold`, `Regular`), a
    /// CSS weight (`700`), or the part of a PostScript name after its `-` (`BoldItalic`). A
    /// variable font has every weight its `wght` axis reaches. A static file is preferred over a
    /// variable one. No nearer style is offered: a style the family doesn't have is
    /// [`Unavailable::NoStyle`].
    pub fn face(&self, style: &str) -> Result<&FontFile, Unavailable> {
        let no_style = || Unavailable::NoStyle { family: self.family.clone(), style: style.to_string() };
        if let Some((weight, italic)) = parse_style(style) {
            let mut found = self.files.iter().filter(|f| f.has_weight(weight, italic));
            let first = found.next();
            let static_one = first.into_iter().chain(found).find(|f| !f.is_variable() && f.weight == weight && f.is_italic() == italic);
            if let Some(f) = static_one.or(first) {
                return Ok(f);
            }
        }
        let wanted = squash(style);
        if wanted.is_empty() {
            return Err(no_style());
        }
        self.files.iter().find(|f| f.postscript.iter().any(|p| p.split_once('-').is_some_and(|(_, s)| squash(s) == wanted))).ok_or_else(no_style)
    }

    /// The file with this PostScript name, matched exactly.
    pub fn face_by_postscript(&self, name: &str) -> Option<&FontFile> {
        self.files.iter().find(|f| f.postscript.iter().any(|p| p == name))
    }

    /// The family's languages.
    pub fn languages<'a>(&'a self, index: &'a Index) -> impl Iterator<Item = &'a Language> + 'a {
        self.languages.iter().filter_map(|&i| index.languages.get(i))
    }

    /// The total size of the family's files.
    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

/// Letters and digits only, lowercased: `Bold Italic`, `bold-italic` and `BoldItalic` are equal.
fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// A style name's CSS weight and whether it is italic: `Bold Italic` is (700, true), `Regular`
/// and the empty style are (400, false), `300` is (300, false). `None` for names that aren't a
/// weight (`Condensed Bold`).
pub fn parse_style(style: &str) -> Option<(u16, bool)> {
    let s = squash(style);
    let (base, italic) = match s.strip_suffix("italic").or_else(|| s.strip_suffix("oblique")) {
        Some(b) => (b, true),
        None => (s.as_str(), false),
    };
    let weight = match base {
        "" | "regular" | "normal" | "book" | "roman" => 400,
        "thin" | "hairline" => 100,
        "extralight" | "ultralight" => 200,
        "light" => 300,
        "medium" => 500,
        "semibold" | "demibold" => 600,
        "bold" => 700,
        "extrabold" | "ultrabold" => 800,
        "black" | "heavy" => 900,
        n => n.parse::<u16>().ok().filter(|w| (1..=1000).contains(w))?,
    };
    Some((weight, italic))
}

/// Why the index has no exact match. Near matches are never offered as installable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Unavailable {
    /// No family of this name.
    #[error("{0} is not in the font index")]
    NoFamily(String),
    /// The family exists, without this style.
    #[error("the font index has {family} but not its {style} style")]
    NoStyle {
        /// The family's name in the index.
        family: String,
        /// The style asked for.
        style: String,
    },
    /// No file has this PostScript name.
    #[error("no font in the index has the PostScript name {0}")]
    NoPostScript(String),
    /// Files of several families have this PostScript name.
    #[error("the PostScript name {name} belongs to several families: {}", families.join(", "))]
    Ambiguous {
        /// The PostScript name.
        name: String,
        /// The families that have it.
        families: Vec<String>,
    },
}

/// An exact match: a family and the file that has the face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face<'a> {
    /// The family.
    pub family: &'a Family,
    /// The file.
    pub file: &'a FontFile,
}

#[derive(Deserialize)]
struct Raw {
    version: String,
    #[serde(default)]
    source: String,
    commit: String,
    base_url: String,
    #[serde(default)]
    fallback_base_url: Option<String>,
    #[serde(default)]
    languages: Vec<Language>,
    #[serde(default)]
    scripts: Vec<Script>,
    families: Vec<Family>,
}

/// A checked font index.
#[derive(Clone, Debug)]
pub struct Index {
    version: String,
    source: String,
    commit: String,
    base_url: String,
    fallback_base_url: Option<String>,
    languages: Vec<Language>,
    scripts: Vec<Script>,
    families: Vec<Family>,
    by_name: HashMap<String, usize>,
    by_postscript: HashMap<String, Vec<(usize, usize)>>,
}

impl Index {
    /// Reads an index from its JSON, checking every entry.
    pub fn from_json(bytes: &[u8]) -> Result<Self, IndexError> {
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(IndexError::TooLarge { max: MAX_INDEX_BYTES });
        }
        let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| IndexError::Json(e.to_string()))?;
        // The version first: a newer major version may change any other field.
        let version = value.get("version").and_then(|v| v.as_str()).ok_or_else(|| IndexError::Version("(none)".into()))?;
        check_version(version)?;
        let raw: Raw = serde_json::from_value(value).map_err(|e| IndexError::Json(e.to_string()))?;
        Self::check(raw)
    }

    /// Reads an index from gzipped JSON.
    #[cfg(feature = "gzip")]
    pub fn from_gzip(bytes: &[u8]) -> Result<Self, IndexError> {
        use std::io::Read;
        let mut json = Vec::new();
        let limit = (MAX_INDEX_BYTES as u64).saturating_add(1);
        flate2::read::GzDecoder::new(bytes).take(limit).read_to_end(&mut json).map_err(|e| IndexError::Gzip(e.to_string()))?;
        Self::from_json(&json)
    }

    /// Reads an index from JSON or, with the `gzip` feature, gzipped JSON.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IndexError> {
        #[cfg(feature = "gzip")]
        if bytes.starts_with(&[0x1f, 0x8b]) {
            return Self::from_gzip(bytes);
        }
        Self::from_json(bytes)
    }

    /// The pinned index compiled into this build ([`crate::PINNED_INDEX`]).
    #[cfg(feature = "bundled-index")]
    pub fn bundled() -> Result<Self, IndexError> {
        Self::from_gzip(include_bytes!("../index/google-fonts-index.json.gz"))
    }

    fn check(raw: Raw) -> Result<Self, IndexError> {
        check_str("commit", &raw.commit)?;
        if raw.commit.len() != 40 || !raw.commit.bytes().all(|b| b.is_ascii_hexdigit()) {
            return invalid(format!("commit `{}` is not 40 hex characters", raw.commit));
        }
        check_str("source", &raw.source)?;
        check_base_url(&raw.base_url)?;
        if let Some(u) = &raw.fallback_base_url {
            check_base_url(u)?;
        }
        if raw.families.len() > MAX_FAMILIES || raw.languages.len() > MAX_FAMILIES * 4 || raw.scripts.len() > MAX_LIST {
            return invalid("too many families, languages or scripts");
        }
        let mut families = raw.families;
        let mut by_name = HashMap::new();
        let mut dirs = HashSet::new();
        let mut total = 0usize;
        for (i, fam) in families.iter_mut().enumerate() {
            check_family(fam, raw.languages.len())?;
            total = total.saturating_add(fam.files.len());
            if total > MAX_FILES {
                return invalid("too many files");
            }
            if by_name.insert(fam.family.to_lowercase(), i).is_some() {
                return invalid(format!("family `{}` is listed twice", fam.family));
            }
            if !dirs.insert(fam.dir.clone()) {
                return invalid(format!("folder `{}` holds more than one family", fam.dir));
            }
        }
        let mut by_postscript: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        for (fi, fam) in families.iter().enumerate() {
            for (xi, file) in fam.files.iter().enumerate() {
                for ps in &file.postscript {
                    let hits = by_postscript.entry(ps.clone()).or_default();
                    if !hits.contains(&(fi, xi)) {
                        hits.push((fi, xi));
                    }
                }
            }
        }
        Ok(Index {
            version: raw.version,
            source: raw.source,
            commit: raw.commit,
            base_url: raw.base_url,
            fallback_base_url: raw.fallback_base_url,
            languages: raw.languages,
            scripts: raw.scripts,
            families,
            by_name,
            by_postscript,
        })
    }

    /// The format version, `"MAJOR.MINOR"`.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Where the fonts come from (`google/fonts`).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The google/fonts commit the index describes.
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Every family, sorted by name.
    pub fn families(&self) -> &[Family] {
        &self.families
    }

    /// Every language a family supports, sorted by ID.
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }

    /// Every script a family uses, sorted by code.
    pub fn scripts(&self) -> &[Script] {
        &self.scripts
    }

    /// The family of this name, ignoring case. Only the whole name matches.
    pub fn family(&self, name: &str) -> Option<&Family> {
        self.by_name.get(&name.to_lowercase()).and_then(|&i| self.families.get(i))
    }

    /// The exact face for a family and style ([`Family::face`]).
    pub fn find_face(&self, family: &str, style: &str) -> Result<Face<'_>, Unavailable> {
        let fam = self.family(family).ok_or_else(|| Unavailable::NoFamily(family.to_string()))?;
        Ok(Face { family: fam, file: fam.face(style)? })
    }

    /// The face with this PostScript name, matched exactly (case included). When files of several
    /// families have the name, it is [`Unavailable::Ambiguous`]: look it up in the document's
    /// family with [`Family::face_by_postscript`].
    pub fn find_postscript(&self, name: &str) -> Result<Face<'_>, Unavailable> {
        let hits = self.by_postscript.get(name).map(Vec::as_slice).unwrap_or_default();
        let mut families: Vec<usize> = hits.iter().map(|&(f, _)| f).collect();
        families.dedup();
        match (families.as_slice(), hits.first()) {
            ([_], Some(&(fi, xi))) => {
                let family = self.families.get(fi).ok_or_else(|| Unavailable::NoPostScript(name.into()))?;
                let file = family.files.get(xi).ok_or_else(|| Unavailable::NoPostScript(name.into()))?;
                Ok(Face { family, file })
            }
            ([], _) | (_, None) => Err(Unavailable::NoPostScript(name.into())),
            (many, _) => Err(Unavailable::Ambiguous {
                name: name.into(),
                families: many.iter().filter_map(|&i| self.families.get(i)).map(|f| f.family.clone()).collect(),
            }),
        }
    }

    /// The URLs of a path in google/fonts (a [`FontFile::path`] or a licence path): jsDelivr,
    /// then raw GitHub.
    pub fn urls(&self, path: &str) -> Vec<String> {
        let encoded = encode_path(path);
        std::iter::once(&self.base_url).chain(&self.fallback_base_url).map(|base| format!("{base}{encoded}")).collect()
    }
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, IndexError> {
    Err(IndexError::Invalid(msg.into()))
}

fn check_version(version: &str) -> Result<(), IndexError> {
    let major = version.split_once('.').and_then(|(major, minor)| {
        let major: u32 = major.parse().ok()?;
        minor.parse::<u32>().ok()?;
        Some(major)
    });
    if major == Some(MAJOR_VERSION) { Ok(()) } else { Err(IndexError::Version(version.chars().take(40).collect())) }
}

fn check_str(what: &str, s: &str) -> Result<(), IndexError> {
    if s.len() > MAX_STR || s.chars().any(char::is_control) {
        return invalid(format!("{what} is too long or has control characters"));
    }
    Ok(())
}

fn check_base_url(u: &str) -> Result<(), IndexError> {
    let ok = u.len() <= MAX_STR
        && u.len() > "https://".len()
        && u.starts_with("https://")
        && u.ends_with('/')
        && !u.chars().any(|c| c.is_control() || c.is_whitespace() || c == '\\');
    if ok { Ok(()) } else { invalid(format!("base URL `{}` is not an https folder URL", u.chars().take(80).collect::<String>())) }
}

/// A file name the store can write on every platform.
pub(crate) fn safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_FILE_NAME
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-[],.+".contains(&b))
}

/// A family folder name: google/fonts' `[a-z0-9_-]+`.
pub(crate) fn safe_dir(name: &str) -> bool {
    !name.is_empty() && name.len() <= MAX_DIR && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Splits `<licence dir>/<family dir>/<file>`, checking each part.
fn split_path(path: &str) -> Result<(&str, &str), IndexError> {
    let mut parts = path.split('/');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(lic), Some(dir), Some(file), None) if ["ofl", "apache", "ufl"].contains(&lic) && safe_dir(dir) && safe_file_name(file) => {
            Ok((dir, file))
        }
        _ => invalid(format!("`{}` is not <ofl|apache|ufl>/<folder>/<file>", path.chars().take(120).collect::<String>())),
    }
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn check_family(fam: &mut Family, languages: usize) -> Result<(), IndexError> {
    check_str("family name", &fam.family)?;
    if fam.family.trim().is_empty() {
        return invalid("a family has no name");
    }
    let name = fam.family.clone();
    check_str("designer", &fam.designer)?;
    check_str("license", &fam.license)?;
    if fam.files.is_empty() {
        return invalid(format!("`{name}` has no files"));
    }
    let lists = [fam.categories.len(), fam.subsets.len(), fam.languages.len(), fam.scripts.len(), fam.files.len()];
    if lists.iter().any(|&n| n > MAX_LIST) {
        return invalid(format!("`{name}` has too many entries"));
    }
    for s in fam.categories.iter().chain(&fam.subsets).chain(&fam.scripts) {
        check_str("category, subset or script", s)?;
    }
    if fam.languages.iter().any(|&i| i >= languages) {
        return invalid(format!("`{name}` names a language the index doesn't have"));
    }
    let mut dir: Option<String> = None;
    let mut names = HashSet::new();
    for f in &fam.files {
        let (d, file) = split_path(&f.path)?;
        let lower = file.to_ascii_lowercase();
        if ![".ttf", ".otf", ".ttc", ".otc"].iter().any(|ext| lower.ends_with(ext)) {
            return invalid(format!("`{}` is not a font file", f.path));
        }
        if !names.insert(lower) {
            return invalid(format!("`{}` is listed twice", f.path));
        }
        match &dir {
            None => dir = Some(d.to_string()),
            Some(x) if x == d => {}
            Some(_) => return invalid(format!("the files of `{name}` are in different folders")),
        }
        if !is_sha256(&f.sha256) {
            return invalid(format!("`{}` has a bad sha256", f.path));
        }
        if f.size == 0 || f.size > MAX_FILE_BYTES {
            return invalid(format!("`{}` has size {}", f.path, f.size));
        }
        if !(1..=1000).contains(&f.weight) {
            return invalid(format!("`{}` has weight {}", f.path, f.weight));
        }
        check_str("style", &f.style)?;
        if f.axes.len() > 64 || f.postscript.len() > MAX_LIST {
            return invalid(format!("`{}` has too many axes or PostScript names", f.path));
        }
        for a in &f.axes {
            if a.tag.len() != 4 || !a.tag.is_ascii() || !a.min.is_finite() || !a.max.is_finite() || a.min > a.max {
                return invalid(format!("`{}` has a bad axis", f.path));
            }
        }
        for ps in &f.postscript {
            check_str("PostScript name", ps)?;
            if ps.is_empty() {
                return invalid(format!("`{}` has an empty PostScript name", f.path));
            }
        }
    }
    let dir = dir.unwrap_or_default();
    if let Some(lp) = &fam.license_path {
        let (d, file) = split_path(lp)?;
        if d != dir || names.contains(&file.to_ascii_lowercase()) {
            return invalid(format!("the licence of `{name}` is not beside its fonts"));
        }
    }
    fam.dir = dir;
    Ok(())
}

/// Percent-encodes a path for a URL, keeping `/` and the unreserved characters.
fn encode_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len() + 8);
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
