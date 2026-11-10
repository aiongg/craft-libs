//! egui's default fonts, stored deflated.

use std::sync::{Arc, LazyLock};

use egui::{FontData, FontDefinitions, FontFamily, FontTweak};

/// A font deflated by build.rs.
macro_rules! deflated {
    ($name:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/egui_fonts/", $name, ".ttf.deflate"))
    };
}

static HACK: LazyLock<Vec<u8>> = LazyLock::new(|| inflate(deflated!("Hack-Regular")));
static EGUI_ICONS: LazyLock<Vec<u8>> = LazyLock::new(|| inflate(deflated!("egui-icons")));
static UBUNTU_LIGHT: LazyLock<Vec<u8>> = LazyLock::new(|| inflate(deflated!("Ubuntu-Light")));

/// Inflated fonts are bounded (the largest is 0.42 MB), so corrupt data can't exhaust memory.
const MAX_INFLATED_BYTES: u64 = 16 << 20;

/// Inflates a font. Data that doesn't inflate gives empty bytes (the font is left out), never a panic.
fn inflate(deflated: &[u8]) -> Vec<u8> {
    use std::io::Read as _;
    let mut out = Vec::new();
    match flate2::read::DeflateDecoder::new(deflated).take(MAX_INFLATED_BYTES + 1).read_to_end(&mut out) {
        Ok(_) if out.len() as u64 <= MAX_INFLATED_BYTES => out,
        _ => Vec::new(),
    }
}

/// egui's default font definitions built from deflated copies of its faces, for builds without
/// `default_fonts` (the web size gate): the same faces (without egui's optional emoji fonts), tweaks and family order.
/// The deflated copies are made by this crate's build script from `epaint_default_fonts`.
pub fn deflated_default_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    let faces: [(&str, &'static [u8], f32); 3] = [
        ("Hack", HACK.as_slice(), 1.0),
        ("Ubuntu-Light", UBUNTU_LIGHT.as_slice(), 1.0),
        // The handful of icons in `egui::special_emojis`, which no platform font has.
        ("egui-icons", EGUI_ICONS.as_slice(), 0.90),
    ];
    for (name, bytes, scale) in faces {
        if bytes.is_empty() {
            log::error!("egui default font {name} is unavailable");
            continue;
        }
        let data = FontData::from_static(bytes).tweak(FontTweak { scale, ..Default::default() });
        fonts.font_data.insert(name.to_owned(), Arc::new(data));
    }
    let stack = |names: &[&str]| -> Vec<String> { names.iter().filter(|n| fonts.font_data.contains_key(**n)).map(|n| (*n).to_owned()).collect() };
    // egui 0.37's family order: egui-icons last, as a fallback.
    let monospace = stack(&["Hack", "Ubuntu-Light", "egui-icons"]);
    let proportional = stack(&["Ubuntu-Light", "egui-icons"]);
    fonts.families.insert(FontFamily::Monospace, monospace);
    fonts.families.insert(FontFamily::Proportional, proportional);
    fonts
}
