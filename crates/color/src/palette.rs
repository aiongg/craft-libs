//! Swatch files (F04.14): Photoshop's `.aco` swatches and `.act` colour tables, swatch exchange
//! `.ase` files, GIMP `.gpl` palettes and CSS custom properties, read into [`SwatchGroup`]s of
//! [`SwatchEntry`]s and written back.
//!
//! Each colour stays in its own model ([`SwatchColor`]): a CMYK swatch keeps its inks and a Lab
//! swatch its values through a read and a write. What a format cannot hold is reported, not
//! dropped silently ([`SwatchReport`]).
//!
//! ```no_run
//! use craft_color::palette::{SwatchFormat, read_swatches, write_swatches};
//! # let bytes: Vec<u8> = Vec::new();
//! let (groups, report) = read_swatches(&bytes, "Brand.ase")?;
//! let (aco, report) = write_swatches(&groups, SwatchFormat::Aco)?;
//! # Ok::<(), craft_color::palette::SwatchError>(())
//! ```

use craft_cms::convert::CmykConverter;
use craft_theme::{Hsb, Lab, Srgba};

/// A swatch file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum SwatchFormat {
    /// Photoshop swatches (`.aco`, binary, versions 1 and 2): RGB, HSB, CMYK, Lab and grey
    /// colours with names, no groups. Colour-book colours are kept as they are.
    Aco,
    /// Photoshop colour table (`.act`, 768 bytes of RGB plus an optional count and transparent
    /// index): 8-bit RGB, no names. Recognised by its file name only: the bytes have no
    /// signature.
    Act,
    /// Adobe swatch exchange (`.ase`, binary): RGB, CMYK, Lab and grey colours, global, spot or
    /// process, in groups.
    Ase,
    /// GIMP palette (`.gpl`, text): 8-bit RGB with names; groups as `# Group:` comment headers
    /// (VectorCraft's convention).
    Gpl,
    /// CSS custom properties on `:root` (`--brand-blue: #3e63dd;`), one comment per group.
    /// Written only.
    Css,
}

impl SwatchFormat {
    /// Every format, in the Export menu's order.
    pub const ALL: [SwatchFormat; 5] = [SwatchFormat::Aco, SwatchFormat::Act, SwatchFormat::Ase, SwatchFormat::Gpl, SwatchFormat::Css];

    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            SwatchFormat::Aco => "aco",
            SwatchFormat::Act => "act",
            SwatchFormat::Ase => "ase",
            SwatchFormat::Gpl => "gpl",
            SwatchFormat::Css => "css",
        }
    }

    /// The format a file name's extension names (case-insensitive), if any.
    // pc: crates/engine/src/cmd/swatch_cmds.rs:377 `Format::parse`, :394 `ext_format`.
    pub fn from_name(file_name: &str) -> Option<SwatchFormat> {
        let extension = std::path::Path::new(file_name).extension()?.to_str()?.to_ascii_lowercase();
        SwatchFormat::ALL.into_iter().find(|f| f.extension() == extension)
    }

    /// Whether [`read_swatches`] reads the format (every one but CSS).
    pub fn can_read(self) -> bool {
        self != SwatchFormat::Css
    }

    /// The format of `bytes`: the `ASEF` signature, `GIMP Palette` on the first line, an `.aco`
    /// version header that parses; `.act` only by `file_name` (768 or 772 bytes).
    // inv-vectorcraft: crates/color/src/palette_io.rs:346 `sniff`, :365 `sniff_bytes`; pc:
    // crates/engine/src/cmd/swatch_cmds.rs:394 `ext_format` for the name fallback.
    pub fn sniff(bytes: &[u8], file_name: &str) -> Option<SwatchFormat> {
        let by_name = SwatchFormat::from_name(file_name);
        let act_length = bytes.len() == ACT_PALETTE_LEN || bytes.len() == ACT_PALETTE_LEN + ACT_TRAILER_LEN;
        if act_length && by_name == Some(SwatchFormat::Act) {
            return Some(SwatchFormat::Act);
        }
        if bytes.starts_with(ASE_SIGNATURE) {
            return Some(SwatchFormat::Ase);
        }
        if is_gpl(bytes) {
            return Some(SwatchFormat::Gpl);
        }
        // An `.act` whose first colour is `0, 1, x` starts like an `.aco`: at `.act` lengths the
        // header counts only when its sections fill the file exactly.
        if matches!(bytes.get(..2), Some([0, 1] | [0, 2]))
            && let Ok((_, len)) = parse_aco(bytes)
            && (!act_length || len == bytes.len())
        {
            return Some(SwatchFormat::Aco);
        }
        by_name
    }
}

/// A swatch colour in its own model.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "model", content = "values", rename_all = "camelCase"))]
pub enum SwatchColor {
    /// Encoded RGB, 0–1 each, in the RGB working space.
    Rgb([f32; 3]),
    /// Hue 0–360°, saturation and brightness 0–1 (a view of an RGB colour).
    Hsb([f32; 3]),
    /// Cyan, magenta, yellow and black ink, 0–1 each, in the CMYK working space.
    Cmyk([f32; 4]),
    /// L* 0–100, a*, b* (D50).
    Lab([f32; 3]),
    /// Black ink 0–1 (0 white), as Photoshop's K.
    Gray(f32),
    /// A colour space this crate does not model (colour-book colours in an `.aco` file), kept
    /// as stored so it survives a round trip. Shows as black.
    Book { space: u16, values: [u16; 4] },
}

impl SwatchColor {
    /// The colour as encoded sRGB for drawing: CMYK through `cmyk` (the document's converter),
    /// Lab through D50, HSB and grey directly; a [`Book`](SwatchColor::Book) colour is black.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:40 (`conv.rgb(&s.color)` per swatch).
    pub fn to_srgb(&self, cmyk: &CmykConverter) -> Srgba {
        match *self {
            SwatchColor::Rgb(v) => Srgba::from_rgb_array(v.map(unit)),
            SwatchColor::Hsb(v) => Srgba::from_rgb_array(hsb_to_rgb(v)),
            SwatchColor::Cmyk(v) => Srgba::from_rgb_array(cmyk.to_srgb(v.map(unit)).map(unit)),
            SwatchColor::Lab([l, a, b]) => {
                let (l, a, b) = (finite(l), finite(a), finite(b));
                // ICC v4 Lab encoding (the Lab D50 profile's device values).
                let encoded = [unit(l / 100.0), unit((a + 128.0) / 255.0), unit((b + 128.0) / 255.0)];
                match profile_to_srgb(&working_transforms().lab, &encoded) {
                    Some(rgb) => Srgba::from_rgb_array(rgb),
                    None => Srgba::from_lab(Lab { l, a, b }),
                }
            }
            SwatchColor::Gray(k) => {
                let level = 1.0 - unit(k);
                Srgba::from_rgb_array(profile_to_srgb(&working_transforms().gray, &[level]).unwrap_or([level; 3]))
            }
            SwatchColor::Book { .. } => Srgba::rgb(0.0, 0.0, 0.0),
        }
    }
}

/// How an `.ase` file marks a colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum SwatchKind {
    #[default]
    Process,
    Global,
    Spot,
}

/// One named swatch.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwatchEntry {
    pub name: String,
    pub color: SwatchColor,
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: SwatchKind,
}

impl SwatchEntry {
    /// A process swatch.
    pub fn new(name: impl Into<String>, color: SwatchColor) -> SwatchEntry {
        SwatchEntry { name: name.into(), color, kind: SwatchKind::Process }
    }
}

/// A named group of swatches, as the Swatches panel shows them. A file without groups reads as
/// one group named after the file.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwatchGroup {
    pub name: String,
    pub entries: Vec<SwatchEntry>,
}

/// What a read or a write could not keep. Each item is a sentence the app can show; the
/// variants let it word them itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SwatchReport {
    pub notes: Vec<SwatchNote>,
}

/// One thing a read or a write could not keep.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SwatchNote {
    /// Colour-book colours (`.aco` colours in a space this crate does not model) read as black,
    /// or left out of a format that cannot hold them, with their count.
    BookColors(usize),
    /// `.ase` colours in a colour model this crate does not read were skipped, with their count.
    Skipped(usize),
    /// The format has no groups: the swatches were written as one list.
    GroupsFlattened,
    /// The format has only 8-bit RGB: CMYK, Lab and grey colours were converted, with their count.
    ConvertedToRgb(usize),
    /// The format has no spot or global colours: they were written as process colours, with their
    /// count.
    KindsDropped(usize),
    /// `.act` holds at most 256 colours: the rest were left out, with their count.
    Truncated(usize),
    /// Names were cut to what the format holds (`.ase`: 1024 UTF-16 units).
    NamesCut(usize),
}

/// Why a swatch file could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SwatchError {
    /// Neither the content nor the name says what the file is.
    UnknownFormat,
    /// The format is write-only (CSS).
    CannotRead(SwatchFormat),
    /// The file is cut short or malformed, with the parser's message.
    Malformed(SwatchFormat, String),
    /// Nothing to write (no swatches in any group).
    Empty,
}

impl std::fmt::Display for SwatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwatchError::UnknownFormat => f.write_str("not a swatch file"),
            SwatchError::CannotRead(format) => write!(f, ".{} files can be written, not read", format.extension()),
            SwatchError::Malformed(format, message) => write!(f, "not a readable .{} file: {message}", format.extension()),
            SwatchError::Empty => f.write_str("there are no swatches to write"),
        }
    }
}

impl std::error::Error for SwatchError {}

/// Reads a swatch file: the format from the content ([`SwatchFormat::sniff`]), `.act` from
/// `file_name`. Ungrouped colours go into a group named after the file (its stem); a swatch
/// without a name is "Swatch N". Names are cut to one line.
// pc: crates/engine/src/cmd/swatch_cmds.rs:491 `read_file` (`.aco`, `.act`, `.ase` with
// :406 `from_aco`, :435 `from_ase`), crates/psd/src/aco.rs:150 `parse`, crates/psd/src/ase.rs:180
// `parse`, crates/psd/src/act.rs:46 `parse`; inv-vectorcraft: crates/color/src/palette_io.rs:376
// `read_gpl`. The binary parsers move here (craft-color has no psd crate).
pub fn read_swatches(bytes: &[u8], file_name: &str) -> Result<(Vec<SwatchGroup>, SwatchReport), SwatchError> {
    let format = SwatchFormat::sniff(bytes, file_name).ok_or(SwatchError::UnknownFormat)?;
    let stem = stem_of(file_name);
    let mut report = SwatchReport::default();
    let mut groups: Vec<SwatchGroup> = Vec::new();
    match format {
        SwatchFormat::Css => return Err(SwatchError::CannotRead(format)),
        SwatchFormat::Aco => {
            let (swatches, _) = parse_aco(bytes).map_err(|e| malformed(format, e))?;
            let books = swatches.iter().filter(|(_, c)| matches!(c, AcoColor::Other { .. })).count();
            push_note(&mut report, books, SwatchNote::BookColors);
            let entries =
                swatches.into_iter().enumerate().map(|(i, (name, color))| SwatchEntry::new(entry_name(i, &name), from_aco(color))).collect();
            groups.push(SwatchGroup { name: stem, entries });
        }
        SwatchFormat::Act => {
            let used = act_used(bytes).map_err(|e| malformed(format, e))?;
            let entries = used
                .iter()
                .enumerate()
                .map(|(i, c)| SwatchEntry::new(format!("Swatch {}", i + 1), SwatchColor::Rgb(c.map(|v| f32::from(v) / 255.0))))
                .collect();
            groups.push(SwatchGroup { name: stem, entries });
        }
        SwatchFormat::Ase => {
            let items = parse_ase(bytes).map_err(|e| malformed(format, e))?;
            let (mut n, mut skipped) = (0usize, 0usize);
            let mut convert = |s: &AseSwatch| {
                n += 1;
                let entry = from_ase(&s.color).map(|color| SwatchEntry { name: entry_name(n - 1, &s.name), color, kind: s.kind });
                skipped += usize::from(entry.is_none());
                entry
            };
            let mut loose = Vec::new();
            for item in &items {
                match item {
                    AseItem::Swatch(s) => loose.extend(convert(s)),
                    AseItem::Group(name, swatches) => {
                        let entries = swatches.iter().filter_map(&mut convert).collect();
                        let name = clip_name(name);
                        groups.push(SwatchGroup { name: if name.is_empty() { stem.clone() } else { name }, entries });
                    }
                }
            }
            if !loose.is_empty() || groups.is_empty() {
                groups.insert(0, SwatchGroup { name: stem, entries: loose });
            }
            push_note(&mut report, skipped, SwatchNote::Skipped);
        }
        SwatchFormat::Gpl => groups = read_gpl(bytes, file_name),
    }
    if count_swatches(&groups) == 0 {
        return Err(malformed(format, "the file holds no swatches"));
    }
    Ok((groups, report))
}

/// Writes `groups` as `format`, with what the format could not hold.
// pc: crates/engine/src/cmd/swatch_cmds.rs:566 `write_file` (`.aco` with :418 `to_aco`, `.ase`
// with :447 `to_ase`, `.act`), crates/psd/src/aco.rs:201 `write` (version 1 then 2),
// crates/psd/src/act.rs:63 `write`; inv-vectorcraft: crates/color/src/palette_io.rs:128
// `write_gpl`, :182 `write_css` with :148 `css_property`.
pub fn write_swatches(groups: &[SwatchGroup], format: SwatchFormat) -> Result<(Vec<u8>, SwatchReport), SwatchError> {
    if count_swatches(groups) == 0 {
        return Err(SwatchError::Empty);
    }
    let mut report = SwatchReport::default();
    let all = || groups.iter().flat_map(|g| g.entries.iter());
    let kinds = all().filter(|e| e.kind != SwatchKind::Process).count();
    let flattened = groups.len() > 1;
    match format {
        SwatchFormat::Aco => {
            if count_swatches(groups) > ACO_MAX_SWATCHES {
                return Err(malformed(format, "an .aco file holds at most 65535 swatches"));
            }
            let entries: Vec<(String, AcoColor)> = all().map(|e| (e.name.clone(), to_aco(&e.color))).collect();
            if flattened {
                report.notes.push(SwatchNote::GroupsFlattened);
            }
            push_note(&mut report, kinds, SwatchNote::KindsDropped);
            push_note(&mut report, names_cut(groups, false), SwatchNote::NamesCut);
            Ok((write_aco(&entries), report))
        }
        SwatchFormat::Act => {
            let (colors, books, converted) = rgb_colors(groups);
            let mut triplets: Vec<[u8; 3]> = colors.iter().flatten().map(|(_, c, _)| *c).collect();
            if triplets.is_empty() {
                return Err(SwatchError::Empty);
            }
            let left_out = triplets.len().saturating_sub(ACT_COLORS);
            triplets.truncate(ACT_COLORS);
            if flattened {
                report.notes.push(SwatchNote::GroupsFlattened);
            }
            push_note(&mut report, books, SwatchNote::BookColors);
            push_note(&mut report, converted, SwatchNote::ConvertedToRgb);
            push_note(&mut report, kinds, SwatchNote::KindsDropped);
            push_note(&mut report, left_out, SwatchNote::Truncated);
            let mut out: Vec<u8> = triplets.iter().flatten().copied().collect();
            out.resize(ACT_PALETTE_LEN, 0);
            out.extend((triplets.len() as u16).to_be_bytes());
            out.extend(0xFFFFu16.to_be_bytes());
            Ok((out, report))
        }
        SwatchFormat::Ase => {
            if count_swatches(groups) > ASE_MAX_SWATCHES || groups.len() > ASE_MAX_GROUPS {
                return Err(malformed(format, "too many swatches or groups for an .ase file"));
            }
            let mut body = Vec::new();
            let (mut blocks, mut books) = (0u32, 0usize);
            for g in groups {
                let mut name = Vec::new();
                put_ase_name(&mut name, &g.name);
                put_ase_block(&mut body, ASE_GROUP_START, &name);
                blocks += 2;
                for e in &g.entries {
                    match to_ase(&e.color) {
                        Some(color) => {
                            put_ase_color(&mut body, &e.name, &color, e.kind);
                            blocks += 1;
                        }
                        None => books += 1,
                    }
                }
                put_ase_block(&mut body, ASE_GROUP_END, &[]);
            }
            push_note(&mut report, books, SwatchNote::BookColors);
            push_note(&mut report, names_cut(groups, true), SwatchNote::NamesCut);
            let mut out = Vec::with_capacity(12 + body.len());
            out.extend(ASE_SIGNATURE);
            out.extend(1u16.to_be_bytes());
            out.extend(0u16.to_be_bytes());
            out.extend(blocks.to_be_bytes());
            out.extend(body);
            Ok((out, report))
        }
        SwatchFormat::Gpl | SwatchFormat::Css => {
            let (colors, books, converted) = rgb_colors(groups);
            push_note(&mut report, books, SwatchNote::BookColors);
            push_note(&mut report, converted, SwatchNote::ConvertedToRgb);
            let text = if format == SwatchFormat::Gpl {
                push_note(&mut report, kinds, SwatchNote::KindsDropped);
                let plain: Vec<Vec<(String, [u8; 3])>> = colors.iter().map(|l| l.iter().map(|(n, c, _)| (n.clone(), *c)).collect()).collect();
                write_gpl(groups, &plain)
            } else {
                write_css(groups, &colors)
            };
            Ok((text.into_bytes(), report))
        }
    }
}

/// PhotoCraft's default swatches (46 RGB colours in five groups: Grays, the exact RGB primaries
/// and secondaries, Pastel, Pure and Dark hues), what Reset Swatches restores.
// pc: crates/engine/src/presets/swatches.rs:294 `builtin`.
pub fn default_swatches() -> Vec<SwatchGroup> {
    let rgb = |name: &str, c: [u8; 3]| SwatchEntry::new(name, SwatchColor::Rgb(c.map(|v| f32::from(v) / 255.0)));
    let grays = [
        ("Black", 0),
        ("90% Gray", 26),
        ("80% Gray", 51),
        ("70% Gray", 77),
        ("60% Gray", 102),
        ("50% Gray", 128),
        ("40% Gray", 153),
        ("30% Gray", 179),
        ("20% Gray", 204),
        ("White", 255),
    ];
    let hues = ["Red", "Orange", "Yellow", "Yellow Green", "Green", "Cyan Green", "Cyan", "Blue", "Violet", "Magenta"];
    let pastel: [[u8; 3]; 10] = [
        [236, 128, 128],
        [244, 176, 132],
        [250, 224, 128],
        [214, 240, 128],
        [150, 232, 150],
        [128, 232, 200],
        [128, 220, 240],
        [128, 176, 244],
        [168, 144, 244],
        [232, 144, 232],
    ];
    let pure: [[u8; 3]; 10] = [
        [230, 40, 40],
        [245, 120, 30],
        [250, 210, 30],
        [160, 220, 40],
        [40, 200, 80],
        [30, 200, 170],
        [30, 170, 230],
        [40, 100, 230],
        [120, 70, 220],
        [210, 50, 180],
    ];
    let dark: [[u8; 3]; 10] = [
        [120, 20, 20],
        [130, 60, 10],
        [130, 110, 10],
        [80, 120, 20],
        [20, 100, 40],
        [10, 100, 90],
        [10, 80, 120],
        [20, 50, 120],
        [60, 30, 110],
        [110, 20, 90],
    ];
    let row = |prefix: &str, colors: [[u8; 3]; 10]| -> Vec<SwatchEntry> {
        hues.iter().zip(colors).map(|(hue, c)| rgb(&if prefix.is_empty() { (*hue).to_string() } else { format!("{prefix} {hue}") }, c)).collect()
    };
    // Photoshop's order: red, yellow, green, cyan, blue, magenta.
    let exact: [(&str, [u8; 3]); 6] = [
        ("RGB Red", [255, 0, 0]),
        ("RGB Yellow", [255, 255, 0]),
        ("RGB Green", [0, 255, 0]),
        ("RGB Cyan", [0, 255, 255]),
        ("RGB Blue", [0, 0, 255]),
        ("RGB Magenta", [255, 0, 255]),
    ];
    let group = |name: &str, entries: Vec<SwatchEntry>| SwatchGroup { name: name.into(), entries };
    vec![
        group("Grays", grays.iter().map(|(n, v)| rgb(n, [*v; 3])).collect()),
        group("RGB", exact.iter().map(|(n, c)| rgb(n, *c)).collect()),
        group("Pastel", row("Pastel", pastel)),
        group("Pure", row("", pure)),
        group("Dark", row("Dark", dark)),
    ]
}

// ---------------------------------------------------------------- file helpers

/// Longest name in a binary swatch file, in UTF-16 code units.
const MAX_NAME_UNITS: usize = 1024;
/// Longest swatch or group name kept from a file, in characters.
const MAX_NAME_CHARS: usize = 255;
const GPL_HEADER: &str = "GIMP Palette";
const GPL_GROUP: &str = "# Group:";
const ASE_SIGNATURE: &[u8; 4] = b"ASEF";
const ASE_GROUP_START: u16 = 0xC001;
const ASE_GROUP_END: u16 = 0xC002;
const ASE_COLOR: u16 = 0x0001;
const ASE_MAX_SWATCHES: usize = 100_000;
const ASE_MAX_GROUPS: usize = 10_000;
const ACO_MAX_SWATCHES: usize = u16::MAX as usize;
const ACT_COLORS: usize = 256;
const ACT_PALETTE_LEN: usize = ACT_COLORS * 3;
const ACT_TRAILER_LEN: usize = 4;

/// Transforms from the Lab D50 and sGray profiles to sRGB (relative colorimetric, black point
/// compensation); `None` when one does not link.
struct WorkingTransforms {
    lab: Option<std::sync::Arc<craft_cms::Transform>>,
    gray: Option<std::sync::Arc<craft_cms::Transform>>,
}

fn working_transforms() -> &'static WorkingTransforms {
    static TRANSFORMS: std::sync::OnceLock<WorkingTransforms> = std::sync::OnceLock::new();
    TRANSFORMS.get_or_init(|| {
        let opts = craft_cms::TransformOptions { intent: craft_cms::Intent::RelativeColorimetric, bpc: true, ..Default::default() };
        let srgb = craft_cms::Builtin::Srgb.profile();
        let link = |source: &craft_cms::Profile| craft_cms::cached(source, srgb, opts).ok();
        WorkingTransforms { lab: link(craft_cms::Builtin::LabD50.profile()), gray: link(craft_cms::Builtin::SGray.profile()) }
    })
}

fn profile_to_srgb(transform: &Option<std::sync::Arc<craft_cms::Transform>>, input: &[f32]) -> Option<[f32; 3]> {
    let mut out = [0.0f32; 3];
    transform.as_ref()?.eval(input, &mut out).ok()?;
    Some(out.map(unit))
}

fn malformed(format: SwatchFormat, message: impl Into<String>) -> SwatchError {
    SwatchError::Malformed(format, message.into())
}

fn unit(v: f32) -> f32 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }
}

fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// One line of text, cut to the length a name may have.
fn clip_name(name: &str) -> String {
    let line = name.split(['\n', '\r', '\t']).collect::<Vec<_>>().join(" ");
    line.trim().chars().take(MAX_NAME_CHARS).collect()
}

fn entry_name(index: usize, name: &str) -> String {
    let name = clip_name(name);
    if name.is_empty() { format!("Swatch {}", index + 1) } else { name }
}

/// The group name for ungrouped colours: the file's stem.
fn stem_of(file_name: &str) -> String {
    let stem = std::path::Path::new(file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let stem = clip_name(stem);
    if stem.is_empty() { "Imported Swatches".into() } else { stem }
}

fn count_swatches(groups: &[SwatchGroup]) -> usize {
    groups.iter().map(|g| g.entries.len()).sum()
}

/// Whole UTF-16 scalar values of `name` that fit in `max` units.
fn utf16_units(name: &str, max: usize) -> Vec<u16> {
    let mut units = Vec::new();
    for c in name.chars() {
        if units.len() + c.len_utf16() > max {
            break;
        }
        let mut buf = [0u16; 2];
        units.extend_from_slice(c.encode_utf16(&mut buf));
    }
    units
}

fn names_cut(groups: &[SwatchGroup], include_groups: bool) -> usize {
    let cut = |name: &str| name.encode_utf16().count() > MAX_NAME_UNITS;
    let swatches = groups.iter().flat_map(|g| g.entries.iter()).filter(|e| cut(&e.name)).count();
    swatches + if include_groups { groups.iter().filter(|g| cut(&g.name)).count() } else { 0 }
}

/// A bounds-checked big-endian cursor.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.remaining() {
            return Err(format!("the file is cut short at byte {}", self.pos));
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(finite(f32::from_bits(self.u32()?)))
    }
    /// Fails when `count` items of at least `min_size` bytes cannot fit in what is left.
    fn check_count(&self, count: u64, min_size: u64) -> Result<(), String> {
        if count.saturating_mul(min_size) > self.remaining() as u64 { Err(format!("the file is cut short at byte {}", self.pos)) } else { Ok(()) }
    }
    /// A name of `n` UTF-16 units that include the trailing null; units past the limit are dropped.
    fn units(&mut self, n: usize) -> Result<String, String> {
        self.check_count(n as u64, 2)?;
        let mut units = Vec::with_capacity(n.min(MAX_NAME_UNITS));
        for _ in 0..n {
            let u = self.u16()?;
            if units.len() < MAX_NAME_UNITS {
                units.push(u);
            }
        }
        while units.last() == Some(&0) {
            units.pop();
        }
        Ok(String::from_utf16_lossy(&units))
    }
}

// ---------------------------------------------------------------- .aco

/// A colour record as stored, at 16 bits (CMYK as ink amounts).
#[derive(Clone, Copy)]
enum AcoColor {
    Rgb([u16; 3]),
    Hsb([u16; 3]),
    Cmyk([u16; 4]),
    Lab { l: u16, a: i16, b: i16 },
    Gray(u16),
    Other { space: u16, values: [u16; 4] },
}

const ACO_RGB: u16 = 0;
const ACO_HSB: u16 = 1;
const ACO_CMYK: u16 = 2;
const ACO_LAB: u16 = 7;
const ACO_GRAY: u16 = 8;

impl AcoColor {
    fn from_record(space: u16, v: [u16; 4]) -> AcoColor {
        match space {
            ACO_RGB => AcoColor::Rgb([v[0], v[1], v[2]]),
            ACO_HSB => AcoColor::Hsb([v[0], v[1], v[2]]),
            ACO_CMYK => AcoColor::Cmyk(v.map(|x| u16::MAX - x)),
            ACO_LAB => AcoColor::Lab { l: v[0], a: v[1] as i16, b: v[2] as i16 },
            ACO_GRAY => AcoColor::Gray(v[0]),
            _ => AcoColor::Other { space, values: v },
        }
    }

    fn to_record(self) -> (u16, [u16; 4]) {
        match self {
            AcoColor::Rgb([r, g, b]) => (ACO_RGB, [r, g, b, 0]),
            AcoColor::Hsb([h, s, b]) => (ACO_HSB, [h, s, b, 0]),
            AcoColor::Cmyk(c) => (ACO_CMYK, c.map(|x| u16::MAX - x)),
            AcoColor::Lab { l, a, b } => (ACO_LAB, [l, a as u16, b as u16, 0]),
            AcoColor::Gray(g) => (ACO_GRAY, [g, 0, 0, 0]),
            AcoColor::Other { space, values } => (space, values),
        }
    }
}

fn q16(v: f32) -> u16 {
    (unit(v) * 65535.0).round() as u16
}

fn from_aco(c: AcoColor) -> SwatchColor {
    let f = |v: u16| f32::from(v) / 65535.0;
    match c {
        AcoColor::Rgb(v) => SwatchColor::Rgb(v.map(f)),
        AcoColor::Hsb([h, sa, b]) => SwatchColor::Hsb([f(h) * 360.0, f(sa), f(b)]),
        AcoColor::Cmyk(v) => SwatchColor::Cmyk(v.map(f)),
        AcoColor::Lab { l, a, b } => SwatchColor::Lab([f32::from(l.min(10000)) / 100.0, f32::from(a) / 100.0, f32::from(b) / 100.0]),
        AcoColor::Gray(g) => SwatchColor::Gray(1.0 - f32::from(g.min(10000)) / 10000.0),
        AcoColor::Other { space, values } => SwatchColor::Book { space, values },
    }
}

fn to_aco(c: &SwatchColor) -> AcoColor {
    match *c {
        SwatchColor::Rgb(v) => AcoColor::Rgb(v.map(q16)),
        SwatchColor::Hsb([h, sa, b]) => {
            let h = finite(h).rem_euclid(360.0);
            AcoColor::Hsb([((h / 360.0) * 65535.0).round().min(65535.0) as u16, q16(sa), q16(b)])
        }
        SwatchColor::Cmyk(v) => AcoColor::Cmyk(v.map(q16)),
        SwatchColor::Lab([l, a, b]) => {
            let ab = |v: f32| (finite(v) * 100.0).round().clamp(-12800.0, 12700.0) as i16;
            AcoColor::Lab { l: (finite(l) * 100.0).round().clamp(0.0, 10000.0) as u16, a: ab(a), b: ab(b) }
        }
        SwatchColor::Gray(k) => AcoColor::Gray(((1.0 - unit(k)) * 10000.0).round() as u16),
        SwatchColor::Book { space, values } => AcoColor::Other { space, values },
    }
}

fn aco_record(r: &mut Reader<'_>) -> Result<AcoColor, String> {
    let space = r.u16()?;
    let v = [r.u16()?, r.u16()?, r.u16()?, r.u16()?];
    Ok(AcoColor::from_record(space, v))
}

fn aco_section(r: &mut Reader<'_>, named: bool) -> Result<Vec<(String, AcoColor)>, String> {
    let count = usize::from(r.u16()?);
    r.check_count(count as u64, if named { 14 } else { 10 })?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let color = aco_record(r)?;
        let name = if named {
            let n = r.u32()?;
            r.units(usize::try_from(n).map_err(|_| "a name is too long".to_string())?)?
        } else {
            String::new()
        };
        out.push((name, color));
    }
    Ok(out)
}

/// The swatches of an `.aco` file (version 2 names when the file has them) and how many bytes its
/// sections span.
fn parse_aco(data: &[u8]) -> Result<(Vec<(String, AcoColor)>, usize), String> {
    let mut r = Reader::new(data);
    match r.u16()? {
        1 => {
            let v1 = aco_section(&mut r, false)?;
            // Files from Photoshop 6 on follow with a version 2 section; other trailing bytes are
            // padding.
            if r.remaining() >= 4 && data[r.pos..].get(..2) == Some(&[0, 2]) {
                r.u16()?;
                let v2 = aco_section(&mut r, true)?;
                return Ok((v2, r.pos));
            }
            Ok((v1, r.pos))
        }
        2 => {
            let v2 = aco_section(&mut r, true)?;
            Ok((v2, r.pos))
        }
        v => Err(format!("unsupported version {v}")),
    }
}

fn write_aco(entries: &[(String, AcoColor)]) -> Vec<u8> {
    let count = entries.len() as u16;
    let mut out = Vec::with_capacity(8 + entries.len() * 40);
    for version in [1u16, 2] {
        out.extend(version.to_be_bytes());
        out.extend(count.to_be_bytes());
        for (name, color) in entries {
            let (space, v) = color.to_record();
            out.extend(space.to_be_bytes());
            for x in v {
                out.extend(x.to_be_bytes());
            }
            if version == 2 {
                let units = utf16_units(name, MAX_NAME_UNITS);
                // The length counts the terminating null.
                out.extend((units.len() as u32 + 1).to_be_bytes());
                for u in units {
                    out.extend(u.to_be_bytes());
                }
                out.extend(0u16.to_be_bytes());
            }
        }
    }
    out
}

// ---------------------------------------------------------------- .act

fn act_used(data: &[u8]) -> Result<Vec<[u8; 3]>, String> {
    if data.len() != ACT_PALETTE_LEN && data.len() != ACT_PALETTE_LEN + ACT_TRAILER_LEN {
        return Err("an .act file is 768 bytes, or 772 with the count trailer".into());
    }
    let mut colors: Vec<[u8; 3]> = data[..ACT_PALETTE_LEN].as_chunks::<3>().0.to_vec();
    if data.len() > ACT_PALETTE_LEN {
        let count = usize::from(u16::from_be_bytes([data[ACT_PALETTE_LEN], data[ACT_PALETTE_LEN + 1]]));
        colors.truncate(count.min(ACT_COLORS));
    }
    Ok(colors)
}

// ---------------------------------------------------------------- .ase

fn from_ase(c: &AseColor) -> Option<SwatchColor> {
    Some(match *c {
        AseColor::Rgb(v) => SwatchColor::Rgb(v),
        AseColor::Cmyk(v) => SwatchColor::Cmyk(v),
        // L is stored as a fraction (L*/100); a few writers store L* itself, which can only be told
        // apart above 1.
        AseColor::Lab([l, a, b]) => SwatchColor::Lab([if l > 1.0 { l } else { l * 100.0 }, a, b]),
        AseColor::Gray(g) => SwatchColor::Gray(1.0 - g),
        AseColor::Other => return None,
    })
}

fn to_ase(c: &SwatchColor) -> Option<AseColor> {
    Some(match *c {
        SwatchColor::Rgb(v) => AseColor::Rgb(v),
        SwatchColor::Hsb(v) => AseColor::Rgb(hsb_to_rgb(v)),
        SwatchColor::Cmyk(v) => AseColor::Cmyk(v),
        SwatchColor::Lab([l, a, b]) => AseColor::Lab([l / 100.0, a, b]),
        SwatchColor::Gray(k) => AseColor::Gray(1.0 - k),
        SwatchColor::Book { .. } => return None,
    })
}

fn hsb_to_rgb([h, s, b]: [f32; 3]) -> [f32; 3] {
    Srgba::from_hsb(Hsb { h: finite(h), s: unit(s), b: unit(b) }).to_rgb_array().map(unit)
}

enum AseColor {
    Rgb([f32; 3]),
    Cmyk([f32; 4]),
    Lab([f32; 3]),
    Gray(f32),
    /// A colour model this crate does not read.
    Other,
}

struct AseSwatch {
    name: String,
    color: AseColor,
    kind: SwatchKind,
}

enum AseItem {
    Swatch(AseSwatch),
    Group(String, Vec<AseSwatch>),
}

fn ase_name(r: &mut Reader<'_>) -> Result<String, String> {
    if r.remaining() == 0 {
        return Ok(String::new());
    }
    let n = usize::from(r.u16()?);
    r.units(n)
}

fn ase_color(body: &mut Reader<'_>) -> Result<AseSwatch, String> {
    let name = ase_name(body)?;
    let model: [u8; 4] = body.array()?;
    let color = match &model {
        b"RGB " => AseColor::Rgb([body.f32()?, body.f32()?, body.f32()?]),
        b"CMYK" => AseColor::Cmyk([body.f32()?, body.f32()?, body.f32()?, body.f32()?]),
        b"LAB " => AseColor::Lab([body.f32()?, body.f32()?, body.f32()?]),
        b"Gray" => AseColor::Gray(body.f32()?),
        _ => {
            // Everything but the trailing colour type belongs to the unknown model.
            let n = body.remaining().saturating_sub(2);
            body.bytes(n)?;
            AseColor::Other
        }
    };
    // Some writers omit the colour type; it defaults to process.
    let kind = if body.remaining() >= 2 {
        match body.u16()? {
            0 => SwatchKind::Global,
            1 => SwatchKind::Spot,
            _ => SwatchKind::Process,
        }
    } else {
        SwatchKind::Process
    };
    Ok(AseSwatch { name, color, kind })
}

fn parse_ase(data: &[u8]) -> Result<Vec<AseItem>, String> {
    let mut r = Reader::new(data);
    let sig: [u8; 4] = r.array()?;
    if &sig != ASE_SIGNATURE {
        return Err("the ASEF signature is missing".into());
    }
    let major = r.u16()?;
    let _minor = r.u16()?;
    if major != 1 {
        return Err(format!("unsupported version {major}"));
    }
    let count = u64::from(r.u32()?);
    // Every block has at least a type and a length.
    r.check_count(count, 6)?;
    let mut items = Vec::new();
    let mut open: Option<(String, Vec<AseSwatch>)> = None;
    let (mut total, mut groups) = (0usize, 0usize);
    for _ in 0..count {
        let kind = r.u16()?;
        let len = usize::try_from(r.u32()?).map_err(|_| "a block is too long".to_string())?;
        let mut body = Reader::new(r.bytes(len)?);
        match kind {
            ASE_GROUP_START => {
                groups += 1;
                if groups > ASE_MAX_GROUPS {
                    return Err("too many groups".into());
                }
                if let Some((name, swatches)) = open.take() {
                    items.push(AseItem::Group(name, swatches));
                }
                open = Some((ase_name(&mut body)?, Vec::new()));
            }
            ASE_GROUP_END => {
                if let Some((name, swatches)) = open.take() {
                    items.push(AseItem::Group(name, swatches));
                }
            }
            ASE_COLOR => {
                total += 1;
                if total > ASE_MAX_SWATCHES {
                    return Err("too many swatches".into());
                }
                let s = ase_color(&mut body)?;
                match open.as_mut() {
                    Some((_, v)) => v.push(s),
                    None => items.push(AseItem::Swatch(s)),
                }
            }
            _ => {}
        }
    }
    if let Some((name, swatches)) = open.take() {
        items.push(AseItem::Group(name, swatches));
    }
    Ok(items)
}

fn put_ase_name(out: &mut Vec<u8>, name: &str) {
    let units = utf16_units(name, MAX_NAME_UNITS);
    out.extend((units.len() as u16 + 1).to_be_bytes());
    for u in units {
        out.extend(u.to_be_bytes());
    }
    out.extend(0u16.to_be_bytes());
}

fn put_ase_block(out: &mut Vec<u8>, kind: u16, body: &[u8]) {
    out.extend(kind.to_be_bytes());
    out.extend((body.len() as u32).to_be_bytes());
    out.extend(body);
}

fn put_ase_color(out: &mut Vec<u8>, name: &str, color: &AseColor, kind: SwatchKind) {
    let mut b = Vec::with_capacity(64);
    put_ase_name(&mut b, name);
    let floats = |b: &mut Vec<u8>, v: &[f32]| {
        for x in v {
            b.extend(x.to_bits().to_be_bytes());
        }
    };
    match color {
        AseColor::Rgb(c) => {
            b.extend(b"RGB ");
            floats(&mut b, c);
        }
        AseColor::Cmyk(c) => {
            b.extend(b"CMYK");
            floats(&mut b, c);
        }
        AseColor::Lab(c) => {
            b.extend(b"LAB ");
            floats(&mut b, c);
        }
        AseColor::Gray(g) => {
            b.extend(b"Gray");
            floats(&mut b, &[*g]);
        }
        AseColor::Other => {}
    }
    let code: u16 = match kind {
        SwatchKind::Global => 0,
        SwatchKind::Spot => 1,
        SwatchKind::Process => 2,
    };
    b.extend(code.to_be_bytes());
    put_ase_block(out, ASE_COLOR, &b);
}

// ---------------------------------------------------------------- .gpl and .css

fn one_line(s: &str) -> String {
    s.split(['\n', '\r', '\t']).collect::<Vec<_>>().join(" ").trim().to_string()
}

fn rgb8(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| (unit(v) * 255.0).round() as u8)
}

fn read_gpl(bytes: &[u8], file_name: &str) -> Vec<SwatchGroup> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}');
    let mut groups: Vec<SwatchGroup> = Vec::new();
    let mut seen = 0usize;
    let mut grouped = false;
    for line in text.lines().skip(1) {
        let line = line.trim();
        if let Some(g) = line.strip_prefix(GPL_GROUP) {
            let name = clip_name(g);
            groups.push(SwatchGroup { name: if name.is_empty() { "Color Group".into() } else { name }, entries: Vec::new() });
            grouped = true;
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let rgb: Vec<u8> = words.iter().take(3).map_while(|v| v.parse().ok()).collect();
        let [r, g, b] = rgb[..] else { continue };
        let entry = SwatchEntry::new(entry_name(seen, &words[3..].join(" ")), SwatchColor::Rgb([r, g, b].map(|v| f32::from(v) / 255.0)));
        seen += 1;
        if !grouped && groups.is_empty() {
            groups.push(SwatchGroup { name: stem_of(file_name), entries: Vec::new() });
        }
        if let Some(group) = groups.last_mut() {
            group.entries.push(entry);
        }
    }
    groups
}

fn write_gpl(groups: &[SwatchGroup], colors: &[Vec<(String, [u8; 3])>]) -> String {
    let title = groups.first().map_or_else(String::new, |g| one_line(&g.name));
    let mut out = format!("{GPL_HEADER}\nName: {title}\nColumns: 0\n#\n");
    for (group, list) in groups.iter().zip(colors) {
        out.push_str(&format!("{GPL_GROUP} {}\n", one_line(&group.name)));
        for (name, [r, g, b]) in list {
            out.push_str(&format!("{r:3} {g:3} {b:3}\t{}\n", one_line(name)));
        }
    }
    out
}

/// `name` as a CSS custom property (`--` and an escaped identifier): lower case, spaces as
/// hyphens, other characters an identifier can't hold escaped with a backslash.
fn css_property(name: &str) -> String {
    let mut out = String::from("--");
    for word in name.split_whitespace() {
        if out.len() > 2 {
            out.push('-');
        }
        for ch in word.chars().flat_map(char::to_lowercase) {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || !ch.is_ascii() {
                out.push(ch);
            } else if ch.is_ascii_graphic() {
                out.push('\\');
                out.push(ch);
            } else {
                out.push_str(&format!("\\{:x} ", ch as u32));
            }
        }
    }
    if out.len() == 2 {
        out.push_str("swatch");
    }
    out
}

/// A comment's text: `*/` can't end it early.
fn css_comment(s: &str) -> String {
    one_line(s).replace("*/", "* /")
}

fn write_css(groups: &[SwatchGroup], colors: &[Vec<RgbSwatch>]) -> String {
    let mut out = String::from(":root {\n");
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    // The next number to try for each base, so repeated names don't search the ones used before.
    let mut next: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (group, list) in groups.iter().zip(colors) {
        out.push_str(&format!("  /* {} */\n", css_comment(&group.name)));
        for (name, [r, g, b], spot) in list {
            // Names differing only in case or punctuation map to one property: number the others.
            let base = css_property(name);
            let property = if taken.insert(base.clone()) {
                base
            } else {
                let n = next.entry(base.clone()).or_insert(1);
                loop {
                    *n += 1;
                    let candidate = format!("{base}-{n}");
                    if taken.insert(candidate.clone()) {
                        break candidate;
                    }
                }
            };
            let note = if *spot { " /* spot */" } else { "" };
            out.push_str(&format!("  {property}: #{r:02x}{g:02x}{b:02x};{note}\n"));
        }
    }
    out.push_str("}\n");
    out
}

/// Whether the head of `bytes` reads as a GIMP palette.
fn is_gpl(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(bytes.get(..1024).unwrap_or(bytes));
    head.trim_start_matches('\u{feff}').trim_start().starts_with(GPL_HEADER)
}

/// Name, 8-bit colour and whether it is a spot colour.
type RgbSwatch = (String, [u8; 3], bool);

/// The colours of a format that holds only 8-bit RGB: other models converted, colour-book colours
/// left out. Returns the colours per group with the `BookColors` and `ConvertedToRgb` counts.
fn rgb_colors(groups: &[SwatchGroup]) -> (Vec<Vec<RgbSwatch>>, usize, usize) {
    let (mut books, mut converted) = (0, 0);
    let out = groups
        .iter()
        .map(|g| {
            g.entries
                .iter()
                .filter_map(|e| {
                    match e.color {
                        SwatchColor::Book { .. } => {
                            books += 1;
                            return None;
                        }
                        SwatchColor::Cmyk(_) | SwatchColor::Lab(_) | SwatchColor::Gray(_) => converted += 1,
                        SwatchColor::Rgb(_) | SwatchColor::Hsb(_) => {}
                    }
                    Some((e.name.clone(), rgb8(e.color.to_srgb(CmykConverter::coated()).to_rgb_array()), e.kind == SwatchKind::Spot))
                })
                .collect()
        })
        .collect();
    (out, books, converted)
}

fn push_note(report: &mut SwatchReport, count: usize, note: impl FnOnce(usize) -> SwatchNote) {
    if count > 0 {
        report.notes.push(note(count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(c: &SwatchColor) -> Vec<f32> {
        match *c {
            SwatchColor::Rgb(v) | SwatchColor::Hsb(v) | SwatchColor::Lab(v) => v.to_vec(),
            SwatchColor::Cmyk(v) => v.to_vec(),
            SwatchColor::Gray(k) => vec![k],
            SwatchColor::Book { .. } => Vec::new(),
        }
    }

    fn assert_same(a: &SwatchColor, b: &SwatchColor) {
        assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b), "{a:?} vs {b:?}");
        if let (SwatchColor::Book { .. }, SwatchColor::Book { .. }) = (a, b) {
            assert_eq!(a, b);
        }
        for (x, y) in values(a).into_iter().zip(values(b)) {
            assert!((x - y).abs() < 1e-3, "{a:?} vs {b:?}");
        }
    }

    fn group(name: &str, entries: Vec<SwatchEntry>) -> SwatchGroup {
        SwatchGroup { name: name.into(), entries }
    }

    /// An `.aco` file with RGB, CMYK and Lab swatches round-trips with each colour in its model.
    #[test]
    fn aco_round_trips_colours_in_their_own_model() {
        let colors = [
            ("Red", SwatchColor::Rgb([1.0, 0.0, 0.25])),
            ("Hue", SwatchColor::Hsb([120.0, 0.5, 0.75])),
            ("Cyan ink", SwatchColor::Cmyk([1.0, 0.0, 0.25, 0.1])),
            ("Lab", SwatchColor::Lab([53.0, -80.0, 67.5])),
            ("Gray", SwatchColor::Gray(0.4)),
            ("Book 日本", SwatchColor::Book { space: 3, values: [1, 2, 3, 4] }),
        ];
        let entries = colors.iter().map(|(n, c)| SwatchEntry::new(*n, *c)).collect();
        let (bytes, report) = write_swatches(&[group("All", entries)], SwatchFormat::Aco).unwrap();
        assert!(report.notes.is_empty(), "{report:?}");
        let (groups, report) = read_swatches(&bytes, "Brand.aco").unwrap();
        assert_eq!(report.notes, [SwatchNote::BookColors(1)]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Brand");
        for ((name, color), entry) in colors.iter().zip(&groups[0].entries) {
            assert_eq!(entry.name, *name);
            assert_same(color, &entry.color);
        }
        // A version 1 section comes first, then version 2 with the names.
        assert_eq!(&bytes[..4], &[0, 1, 0, 6]);
        assert!(bytes.windows(4).any(|w| w == [0, 2, 0, 6]));
    }

    /// `.ase` keeps groups and spot colours; `.aco` flattens them and says so.
    #[test]
    fn ase_keeps_groups_and_aco_reports_flattening() {
        let mut spot = SwatchEntry::new("Spot Orange", SwatchColor::Cmyk([0.0, 0.5, 1.0, 0.0]));
        spot.kind = SwatchKind::Spot;
        let groups = vec![
            group("Brand", vec![SwatchEntry::new("Blue", SwatchColor::Rgb([0.0, 0.0, 1.0])), spot]),
            group("Greys", vec![SwatchEntry::new("Mid", SwatchColor::Gray(0.5))]),
        ];
        let (ase, report) = write_swatches(&groups, SwatchFormat::Ase).unwrap();
        assert!(report.notes.is_empty());
        let (back, _) = read_swatches(&ase, "x.ase").unwrap();
        assert_eq!(back.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Brand", "Greys"]);
        assert_eq!(back[0].entries[1].kind, SwatchKind::Spot);
        assert_same(&back[0].entries[1].color, &groups[0].entries[1].color);
        assert_same(&back[1].entries[0].color, &SwatchColor::Gray(0.5));

        // A colour in a model this crate does not read is skipped and counted.
        let mut unknown = Vec::new();
        put_ase_name(&mut unknown, "Odd");
        unknown.extend(b"XYZ ");
        unknown.extend([0u8; 12]);
        unknown.extend(2u16.to_be_bytes());
        let mut file = ase.clone();
        put_ase_block(&mut file, ASE_COLOR, &unknown);
        file[8..12].copy_from_slice(&(u32::from_be_bytes(ase[8..12].try_into().unwrap()) + 1).to_be_bytes());
        let (skipped, report) = read_swatches(&file, "x.ase").unwrap();
        assert_eq!(report.notes, [SwatchNote::Skipped(1)]);
        assert_eq!(count_swatches(&skipped), 3);

        let (aco, report) = write_swatches(&groups, SwatchFormat::Aco).unwrap();
        assert_eq!(report.notes, [SwatchNote::GroupsFlattened, SwatchNote::KindsDropped(1)]);
        let (flat, _) = read_swatches(&aco, "x.aco").unwrap();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].entries.len(), 3);
    }

    /// A `.gpl` palette with `# Group:` headers reads as groups; a nameless line is "Swatch N".
    #[test]
    fn gpl_reads_group_headers_and_fills_missing_names() {
        let text = "GIMP Palette\nName: Test\nColumns: 0\n#\n255   0   0\tRed\n# Group: Cool\n  0   0 255\n  0 255 255\tAqua\n";
        let (groups, _) = read_swatches(text.as_bytes(), "Test.gpl").unwrap();
        assert_eq!(groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Test", "Cool"]);
        assert_eq!(groups[0].entries[0].name, "Red");
        assert_eq!(groups[1].entries[0].name, "Swatch 2");
        assert_eq!(groups[1].entries[1].name, "Aqua");
        assert_same(&groups[1].entries[0].color, &SwatchColor::Rgb([0.0, 0.0, 1.0]));

        let (written, report) = write_swatches(&groups, SwatchFormat::Gpl).unwrap();
        assert!(report.notes.is_empty());
        let (again, _) = read_swatches(&written, "Test.gpl").unwrap();
        assert_eq!(again, groups);
    }

    /// The format comes from the content, whatever the extension; `.act` only from the name.
    #[test]
    fn the_format_is_sniffed_from_content_and_act_from_the_name() {
        let entries = vec![SwatchEntry::new("A", SwatchColor::Rgb([1.0, 0.0, 0.0]))];
        let groups = [group("G", entries)];
        for format in [SwatchFormat::Aco, SwatchFormat::Ase, SwatchFormat::Gpl] {
            let (bytes, _) = write_swatches(&groups, format).unwrap();
            assert_eq!(SwatchFormat::sniff(&bytes, "mystery.bin"), Some(format));
            assert_eq!(SwatchFormat::sniff(&bytes, "wrong.css"), Some(format));
        }
        // 768 bytes starting `0, 1, x`: only the name makes it an `.act`.
        let mut act = vec![0u8; ACT_PALETTE_LEN];
        act[1] = 1;
        assert_eq!(SwatchFormat::sniff(&act, "table.ACT"), Some(SwatchFormat::Act));
        assert_eq!(SwatchFormat::sniff(&act, "table.bin"), None);
        let (groups, _) = read_swatches(&act, "table.act").unwrap();
        assert_eq!(groups[0].entries.len(), 256);
        let (written, _) = write_swatches(&groups, SwatchFormat::Act).unwrap();
        assert_eq!(written.len(), 772);
        assert_eq!(SwatchFormat::from_name("a/b.Gpl"), Some(SwatchFormat::Gpl));
        assert_eq!(SwatchFormat::from_name("noext"), None);
        assert_eq!(read_swatches(b"hello", "x.txt"), Err(SwatchError::UnknownFormat));
    }

    /// CSS is written with escaped property names and cannot be read.
    #[test]
    fn css_is_written_with_escaped_names_and_not_read() {
        let mut spot = SwatchEntry::new("Brand Blue #2", SwatchColor::Rgb([0.0, 0.0, 1.0]));
        spot.kind = SwatchKind::Spot;
        let groups = [group("Brand", vec![spot, SwatchEntry::new("brand blue #2", SwatchColor::Rgb([1.0, 0.0, 0.0]))])];
        let (bytes, report) = write_swatches(&groups, SwatchFormat::Css).unwrap();
        assert!(report.notes.is_empty());
        let css = String::from_utf8(bytes.clone()).unwrap();
        assert_eq!(css, ":root {\n  /* Brand */\n  --brand-blue-\\#2: #0000ff; /* spot */\n  --brand-blue-\\#2-2: #ff0000;\n}\n");
        assert_eq!(read_swatches(&bytes, "x.css"), Err(SwatchError::CannotRead(SwatchFormat::Css)));
        assert!(!SwatchFormat::Css.can_read());
        assert_eq!(write_swatches(&[], SwatchFormat::Css), Err(SwatchError::Empty));
    }

    /// Each model draws as sRGB; CMYK goes through the converter.
    #[test]
    fn swatch_colours_draw_in_srgb() {
        let cmyk = CmykConverter::coated();
        let near = |c: Srgba, rgb: [f32; 3], tol: f32| (c.r - rgb[0]).abs() <= tol && (c.g - rgb[1]).abs() <= tol && (c.b - rgb[2]).abs() <= tol;
        assert!(near(SwatchColor::Hsb([120.0, 1.0, 1.0]).to_srgb(cmyk), [0.0, 1.0, 0.0], 1e-6));
        assert!(near(SwatchColor::Lab([100.0, 0.0, 0.0]).to_srgb(cmyk), [1.0; 3], 0.01));
        assert!(near(SwatchColor::Gray(1.0).to_srgb(cmyk), [0.0; 3], 1e-6));
        assert!(near(SwatchColor::Book { space: 3, values: [0; 4] }.to_srgb(cmyk), [0.0; 3], 0.0));
        assert!(near(SwatchColor::Cmyk([0.0; 4]).to_srgb(cmyk), [1.0; 3], 0.03));
        let cyan = SwatchColor::Cmyk([1.0, 0.0, 0.0, 0.0]).to_srgb(cmyk);
        assert!(cyan.r < 0.3 && cyan.g > cyan.r && cyan.b > cyan.r, "{cyan:?}");
        assert_eq!(default_swatches().iter().map(|g| g.entries.len()).sum::<usize>(), 46);
    }
}
