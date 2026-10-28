//! The candidate interface faces and the `specimens/type/faces` story.
//!
//! The faces come from craft-fonts' `fonts/ui-candidates-manifest.txt`, embedded by `build.rs`
//! when `CRAFT_FONTS_DIR` is set. Every candidate has its own egui family (`face:<name>`, and
//! `face:<name>:semibold`), so the specimen shows all of them side by side. The appearance bar's
//! pickers put the chosen faces into the proportional, semibold and monospace slots.

use std::sync::Arc;

use craft_theme::{Size, TypeRole};
use craft_ui::fonts::{Family, I18nScripts, SEMIBOLD, SystemFonts, UiFonts};
use craft_ui::story::{Args, Level, Story};
use craft_ui::theme::{FontFamilies, ToEgui, Tokens, TokensOf, scope, style_for, style_format, tabular};
use egui::text::LayoutJob;
use egui::{Color32, FontData, FontFamily, RichText, Ui};

/// A font file from the candidate manifest.
pub struct Candidate {
    pub family: &'static str,
    pub style: &'static str,
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/ui_candidates.rs"));

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Faces"];

/// Candidate interface faces, in the order of the picker. Only Inter has `tnum`.
pub const UI_FACES: &[&str] = &["Inter", "IBM Plex Sans", "Source Sans 3"];
/// Candidate monospace faces.
pub const MONO_FACES: &[&str] = &["JetBrains Mono", "IBM Plex Mono", "Source Code Pro"];

/// Whether the candidate faces were embedded.
pub fn available() -> bool {
    !UI_CANDIDATES.is_empty()
}

fn has(family: &str) -> bool {
    UI_CANDIDATES.iter().any(|c| c.family == family)
}

/// The egui family of a candidate's regular face.
pub fn regular_family(face: &str) -> FontFamily {
    FontFamily::Name(format!("face:{face}").into())
}

/// The egui family of a candidate's semibold face.
pub fn semibold_family(face: &str) -> FontFamily {
    FontFamily::Name(format!("face:{face}:semibold").into())
}

/// Installs the candidates: each in its own families, the chosen UI face in the proportional and
/// semibold slots and the chosen mono face in the monospace slot. Does nothing without
/// candidates, so the gallery keeps egui's faces.
pub fn install(ctx: &egui::Context, ui_face: &str, mono_face: &str) {
    if !available() {
        return;
    }
    let mut fonts = UiFonts::new(&I18nScripts).system_fonts(SystemFonts::Off).family(Family::named(SEMIBOLD).weight(600));
    for c in UI_CANDIDATES {
        let semibold = c.style.eq_ignore_ascii_case("semibold");
        let mono = MONO_FACES.contains(&c.family);
        let own = if semibold { semibold_family(c.family) } else { regular_family(c.family) };
        let mut families = vec![own.clone()];
        if c.family == ui_face {
            families.push(if semibold { FontFamily::Name(SEMIBOLD.into()) } else { FontFamily::Proportional });
        }
        if c.family == mono_face && !semibold {
            families.push(FontFamily::Monospace);
        }
        let name = format!("{} {}", c.family, c.style);
        fonts = fonts.app_face(&name, FontData::from_static(c.bytes), &families);
        let mut family = Family::named(&format!("face:{}{}", c.family, if semibold { ":semibold" } else { "" }));
        if semibold {
            family = family.weight(600);
        }
        fonts = fonts.family(if mono { family.monospaced() } else { family });
    }
    fonts.install(ctx);
}

/// The stories this crate adds to the registry.
pub fn stories() -> Vec<Story> {
    vec![Story {
        id: "specimens/type/faces",
        level: Level::Specimens,
        component: "Faces",
        name: "Type faces",
        args: || Args::new().with_choice("group", GROUPS, 0).with_bool("tabular", false),
        show,
    }]
}

const GROUPS: &[&str] = &["UI faces", "Mono faces"];

const LATIN: &str = "Sphinx of black quartz, judge my vow. 0123456789";
const GREEK: &str = "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία";
const CYRILLIC: &str = "Съешь же ещё этих мягких французских булок";
const CODE: &str = "fn main() { let n = 0x1F + 1_000; } // 1111 0000";

fn show(ui: &mut Ui, args: &mut Args) {
    if !available() {
        ui.colored_label(
            Tokens::of(ui).color.status.warn.fg.to_egui(),
            "Built without the candidate faces: set CRAFT_FONTS_DIR to a craft-fonts checkout (branch design-system-fonts) and rebuild. The default faces are shown.",
        );
    }
    let mono = args.choice("group") == "Mono faces";
    let tnum = args.bool("tabular");
    let faces = if mono { MONO_FACES } else { UI_FACES };
    ui.columns(faces.len(), |columns| {
        for (ui, face) in columns.iter_mut().zip(faces) {
            ui.push_id(face, |ui| column(ui, face, mono, tnum));
        }
    });
}

/// The slots of one candidate: the UI face in the UI slots, or the mono face in the mono slot.
fn families_of(ui: &Ui, face: &str, mono: bool) -> FontFamilies {
    let installed = FontFamilies::installed(ui.ctx());
    match (has(face), mono) {
        (false, _) => installed,
        (true, false) => FontFamilies { ui: regular_family(face), ui_semibold: semibold_family(face), mono: installed.mono },
        (true, true) => FontFamilies { mono: regular_family(face), ..installed },
    }
}

fn column(ui: &mut Ui, face: &str, mono: bool, tnum: bool) {
    let tokens = Tokens::of(ui);
    let families = families_of(ui, face, mono);
    let text = tokens.color.text.normal.to_egui();
    ui.label(RichText::new(face).font(tokens.font(TypeRole::Title)));
    ui.add_space(4.0);
    let roles = [
        (TypeRole::Caption, "Caption"),
        (TypeRole::Label, "Label"),
        (TypeRole::Body, "Body"),
        (TypeRole::BodyStrong, "Body strong"),
        (TypeRole::Title, "Title"),
        (TypeRole::Heading, "Heading"),
        (TypeRole::Mono, "Mono"),
    ];
    for (role, name) in roles {
        ui.label(RichText::new(name).font(tokens.font(TypeRole::Caption)).color(tokens.color.text.muted.to_egui()));
        let samples: &[&str] = if mono || role == TypeRole::Mono { &[CODE, GREEK, CYRILLIC] } else { &[LATIN, GREEK, CYRILLIC] };
        for sample in samples {
            let mut format = style_format(&families, tokens.typography.get(role), text);
            if tnum {
                format = tabular(format);
            }
            sample_line(ui, sample, format);
        }
    }
    ui.add_space(8.0);
    digits(ui, &families, mono);
    for size in [Size::Compact, Size::Comfortable] {
        ui.add_space(8.0);
        scope(ui, |o| o.size = Some(size), |ui| inspector(ui, &families, size));
    }
}

fn sample_line(ui: &mut Ui, text: &str, format: egui::TextFormat) {
    let mut job = LayoutJob::single_section(text.to_owned(), format);
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}

/// Digits without and with the `tnum` feature, in the body role.
fn digits(ui: &mut Ui, families: &FontFamilies, mono: bool) {
    let tokens = Tokens::of(ui);
    let role = if mono { TypeRole::Mono } else { TypeRole::Body };
    let text = tokens.color.text.normal.to_egui();
    let format = style_format(families, tokens.typography.get(role), text);
    for (name, format) in [("default", format.clone()), ("tnum", tabular(format))] {
        ui.label(RichText::new(name).font(tokens.font(TypeRole::Caption)).color(tokens.color.text.muted.to_egui()));
        sample_line(ui, "1111  0000\n1234.56\n  777.00", format);
    }
}

/// A mock inspector panel of stock egui widgets under the style generated from the tokens.
fn inspector(ui: &mut Ui, families: &FontFamilies, size: Size) {
    let tokens = Tokens::of(ui);
    let style = Arc::new(style_for(&tokens, families));
    ui.label(RichText::new(format!("Inspector, {size:?}")).font(tokens.font(TypeRole::Caption)).color(tokens.color.text.muted.to_egui()));
    let frame = egui::Frame::new()
        .fill(tokens.color.surface.panel.to_egui())
        .stroke(egui::Stroke::new(1.0, tokens.color.line.border.to_egui()))
        .corner_radius(tokens.radius.panel)
        .inner_margin(egui::Margin::same(8));
    frame.show(ui, |ui| {
        ui.set_style(style);
        ui.heading("Transform");
        ui.separator();
        egui::Grid::new("inspector").num_columns(2).show(ui, |ui| {
            for (label, value) in [("X", 1024.5_f64), ("Y", 768.25), ("Width", 111.0), ("Height", 1000.0)] {
                ui.label(label);
                let mut v = value;
                ui.add(egui::DragValue::new(&mut v));
                ui.end_row();
            }
        });
        ui.separator();
        for (i, item) in ["Duplicate", "Group", "Arrange"].into_iter().enumerate() {
            let _ = ui.selectable_label(i == 1, item);
        }
        ui.separator();
        ui.horizontal(|ui| {
            let _ = ui.button("Cancel");
            ui.add(egui::Button::new("Apply").fill(Color32::TRANSPARENT).selected(true));
        });
    });
}

#[cfg(test)]
mod tests {
    use egui::{Context, RawInput};

    use craft_ui::story::ArgValue;

    use super::*;

    #[test]
    fn both_groups_render_with_and_without_tabular_figures() {
        let ctx = Context::default();
        install(&ctx, "Inter", "JetBrains Mono");
        let story = stories().remove(0);
        for group in 0..GROUPS.len() {
            for tnum in [false, true] {
                let mut args = story.defaults();
                for arg in args.iter_mut() {
                    match &mut arg.value {
                        ArgValue::Choice { index, .. } => *index = group,
                        ArgValue::Bool(b) => *b = tnum,
                        _ => {}
                    }
                }
                for _ in 0..2 {
                    ctx.run_ui(RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
                }
            }
        }
    }
}
