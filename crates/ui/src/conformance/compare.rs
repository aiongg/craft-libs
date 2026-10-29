//! What a side of the comparison reads from a context, and the keyed difference of two sides.

use std::collections::HashMap;
use std::sync::Arc;

use craft_theme::{Mode, TypeRole, dump::tokens_text};
use egui::{Context, RawInput, Style, Theme};

use crate::direction::HasDirection;
use crate::testing::dump::{Dump, DumpValue};
use crate::testing::style_dump::style_dump_into;
use crate::theme::{UiTokens, tokens_in_mode};

pub(super) type Rows = Vec<(String, String)>;

const ROLES: [(&str, TypeRole); 7] = [
    ("caption", TypeRole::Caption),
    ("label", TypeRole::Label),
    ("body", TypeRole::Body),
    ("body_strong", TypeRole::BodyStrong),
    ("title", TypeRole::Title),
    ("heading", TypeRole::Heading),
    ("mono", TypeRole::Mono),
];

/// The three groups of keyed values for one mode.
pub(super) struct ModeRows {
    pub mode: Mode,
    /// The variant the theme shows in this mode.
    pub shown: Mode,
    pub tokens: Rows,
    pub ui: Rows,
    pub style: Rows,
}

/// Everything compared on one side: both modes, then the context's own settings.
pub(super) struct Side {
    pub modes: [ModeRows; 2],
    /// The mode egui shows.
    pub active: Mode,
    pub context: Rows,
}

impl Side {
    /// Runs `frames` frames on `ctx`, then reads it.
    pub fn read(ctx: &Context, frames: usize) -> Side {
        for _ in 0..frames {
            ctx.run_ui(RawInput::default(), |_| {}).drop_without_applying_deltas();
        }
        let modes = [Mode::Light, Mode::Dark].map(|mode| {
            let theme = if mode == Mode::Light { Theme::Light } else { Theme::Dark };
            mode_rows(mode, &tokens_in_mode(ctx, mode), &ctx.style_of(theme))
        });
        let mut context = Dump::new();
        context.line("theme_preference", format!("{:?}", ctx.options(|o| o.theme_preference)));
        context.line("direction", format!("{:?}", ctx.direction()));
        context.line("zoom_factor", ctx.zoom_factor());
        let active = if ctx.theme() == Theme::Dark { Mode::Dark } else { Mode::Light };
        Side { modes, active, context: rows(&context) }
    }
}

fn rows(dump: &Dump) -> Rows {
    dump.entries().map(|(k, v)| (k, v.to_owned())).collect()
}

fn mode_rows(mode: Mode, ui: &Arc<UiTokens>, style: &Style) -> ModeRows {
    let tokens = tokens_text(ui.tokens()).lines().filter_map(|l| l.split_once(" = ")).map(|(k, v)| (k.to_owned(), v.to_owned())).collect();

    let mut d = Dump::new();
    let a = &ui.appearance;
    d.group("appearance", |d| {
        d.line("mode", format!("{:?}", a.mode)).line("contrast", format!("{:?}", a.contrast)).line("size", format!("{:?}", a.size));
        d.line("text_size", format!("{:?}", a.text_size)).line("reduced_motion", a.reduced_motion);
    });
    d.line("mode", format!("{:?}", ui.mode));
    d.group("radius", |d| {
        let r = &ui.radius;
        d.line("control", r.control).line("popup", r.popup).line("window", r.window);
        d.line("panel", r.panel).line("button", r.button).line("pill", r.pill);
    });
    d.group("shadow", |d| {
        let s = &ui.shadow;
        d.line("popup", s.popup).line("dialog", s.dialog).line("floating", s.floating).line("knob", s.knob);
    });
    d.group("font", |d| {
        for (name, role) in ROLES {
            d.line(name, ui.font(role));
        }
    });
    d.group("families", |d| {
        let f = ui.families();
        d.line("ui", &f.ui).line("ui_semibold", &f.ui_semibold).line("mono", &f.mono).line("optical_size", f.optical_size);
    });

    let mut style_dump = Dump::new();
    style_dump_into(&mut style_dump, style);
    let mut style_rows = rows(&style_dump);
    style_rows.push(("override_font_id".to_owned(), format!("{:?}", style.override_font_id)));
    style_rows.push(("override_text_style".to_owned(), format!("{:?}", style.override_text_style)));
    style_rows.push(("interaction.tooltip_delay".to_owned(), style.interaction.tooltip_delay.dump_value()));
    style_rows.push(("right_to_left".to_owned(), style.right_to_left.to_string()));

    ModeRows { mode, shown: ui.mode, tokens, ui: rows(&d), style: style_rows }
}

/// One key that differs; `None` is a key the side does not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Diff {
    pub key: String,
    pub reference: Option<String>,
    pub app: Option<String>,
}

/// The keys on which `reference` and `app` differ: the reference's order, then the keys only the
/// app has.
pub(super) fn diff_rows(reference: &Rows, app: &Rows) -> Vec<Diff> {
    let in_reference: HashMap<&str, &str> = reference.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let in_app: HashMap<&str, &str> = app.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let mut out = Vec::new();
    for (key, value) in reference {
        let other = in_app.get(key.as_str());
        if other != Some(&value.as_str()) {
            out.push(Diff { key: key.clone(), reference: Some(value.clone()), app: other.map(|v| (*v).to_owned()) });
        }
    }
    for (key, value) in app {
        if !in_reference.contains_key(key.as_str()) {
            out.push(Diff { key: key.clone(), reference: None, app: Some(value.clone()) });
        }
    }
    out
}
