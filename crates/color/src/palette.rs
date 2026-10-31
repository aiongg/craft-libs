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

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;

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
        todo!()
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
        todo!()
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
        todo!()
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
    /// Colour-book colours read as black (`.aco`, `.ase` colours in a space this crate does not
    /// model), with their count.
    BookColors(usize),
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
    todo!()
}

/// Writes `groups` as `format`, with what the format could not hold.
// pc: crates/engine/src/cmd/swatch_cmds.rs:566 `write_file` (`.aco` with :418 `to_aco`, `.ase`
// with :447 `to_ase`, `.act`), crates/psd/src/aco.rs:201 `write` (version 1 then 2),
// crates/psd/src/act.rs:63 `write`; inv-vectorcraft: crates/color/src/palette_io.rs:128
// `write_gpl`, :182 `write_css` with :148 `css_property`.
pub fn write_swatches(groups: &[SwatchGroup], format: SwatchFormat) -> Result<(Vec<u8>, SwatchReport), SwatchError> {
    todo!()
}

/// PhotoCraft's default swatches (Photoshop's default set: RGB, CMYK, grey, pastel and dark
/// groups), what Reset Swatches restores.
// pc: crates/engine/src/presets/swatches.rs:294 `builtin`.
pub fn default_swatches() -> Vec<SwatchGroup> {
    todo!()
}

#[cfg(test)]
mod tests {
    /// An `.aco` file with RGB, CMYK and Lab swatches round-trips with each colour in its model.
    #[test]
    #[ignore = "skeleton"]
    fn aco_round_trips_colours_in_their_own_model() {
        todo!()
    }

    /// `.ase` keeps groups and spot colours; `.aco` flattens them and says so.
    #[test]
    #[ignore = "skeleton"]
    fn ase_keeps_groups_and_aco_reports_flattening() {
        todo!()
    }

    /// A `.gpl` palette with `# Group:` headers reads as groups; a nameless line is "Swatch N".
    #[test]
    #[ignore = "skeleton"]
    fn gpl_reads_group_headers_and_fills_missing_names() {
        todo!()
    }

    /// The format comes from the content, whatever the extension; `.act` only from the name.
    #[test]
    #[ignore = "skeleton"]
    fn the_format_is_sniffed_from_content_and_act_from_the_name() {
        todo!()
    }

    /// CSS is written with escaped property names and cannot be read.
    #[test]
    #[ignore = "skeleton"]
    fn css_is_written_with_escaped_names_and_not_read() {
        todo!()
    }
}
