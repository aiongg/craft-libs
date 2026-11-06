//! Harmony controls (F04.16): [`HarmonyWheel`], the colours of a harmony as markers on a hue and
//! saturation wheel, and [`HarmonyPanel`], VectorCraft's Color Guide (rule, harmony strip,
//! variation grid). The math is [`crate::harmony`].

use craft_theme::{ControlSize, Hsb, Srgba};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, Context, Id, IdSalt, Rect, Stroke, StrokeKind, Ui, vec2};

use super::ColorDisc;
use super::picker::{Inner, RowRule, row_line, texts as picker_texts};
use crate::harmony::{Guide, GuideOptions, Harmony, Variation};
use craft_ui::controls::{NumberField, Select, Swatch};
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::edit::{EditEvent, EditPolicy};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, Word, Words};
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};

craft_ui::words! {
    /// The harmony controls' words, English by default. The rules' names are [`HARMONY_WORDS`].
    #[non_exhaustive]
    pub struct HarmonyLabels {
        rule: "harmony.rule" = "Rule:";
        custom: "harmony.custom" = "Custom";
        brightness: "harmony.brightness" = "Brightness";
        set_base: "harmony.setBase" = "Set base color to the current color";
        show_tints_shades: "harmony.showTintsShades" = "Show Tints/Shades";
        show_warm_cool: "harmony.showWarmCool" = "Show Warm/Cool";
        show_vivid_muted: "harmony.showVividMuted" = "Show Vivid/Muted";
        save_as_swatches: "harmony.saveAsSwatches" = "Save Colors as Swatches";
        options: "harmony.options" = "Color Guide Options…";
        shades: "harmony.shades" = "Shades";
        tints: "harmony.tints" = "Tints";
        cool: "harmony.cool" = "Cool";
        warm: "harmony.warm" = "Warm";
        muted: "harmony.muted" = "Muted";
        vivid: "harmony.vivid" = "Vivid";
    }
}

/// The rules' names as words, keyed `harmony.rule.<id>` with [`Harmony::id`] (21 entries,
/// [`Harmony::ALL`]'s order).
// inv-vectorcraft: crates/color/src/harmony.rs:101 `label`, :128 `id`.
pub const HARMONY_WORDS: &[craft_ui::localized::Word] = &[
    craft_ui::localized::Word::keyed("harmony.rule.complementary", "Complementary"),
    craft_ui::localized::Word::keyed("harmony.rule.complementary2", "Complementary 2"),
    craft_ui::localized::Word::keyed("harmony.rule.splitComplementary", "Split Complementary"),
    craft_ui::localized::Word::keyed("harmony.rule.leftComplement", "Left Complement"),
    craft_ui::localized::Word::keyed("harmony.rule.rightComplement", "Right Complement"),
    craft_ui::localized::Word::keyed("harmony.rule.analogous", "Analogous"),
    craft_ui::localized::Word::keyed("harmony.rule.analogous2", "Analogous 2"),
    craft_ui::localized::Word::keyed("harmony.rule.monochromatic", "Monochromatic"),
    craft_ui::localized::Word::keyed("harmony.rule.shades", "Shades"),
    craft_ui::localized::Word::keyed("harmony.rule.triad", "Triad"),
    craft_ui::localized::Word::keyed("harmony.rule.triad2", "Triad 2"),
    craft_ui::localized::Word::keyed("harmony.rule.triad3", "Triad 3"),
    craft_ui::localized::Word::keyed("harmony.rule.tetrad", "Tetrad"),
    craft_ui::localized::Word::keyed("harmony.rule.tetrad2", "Tetrad 2"),
    craft_ui::localized::Word::keyed("harmony.rule.tetrad3", "Tetrad 3"),
    craft_ui::localized::Word::keyed("harmony.rule.compound", "Compound"),
    craft_ui::localized::Word::keyed("harmony.rule.compound2", "Compound 2"),
    craft_ui::localized::Word::keyed("harmony.rule.highContrast", "High Contrast"),
    craft_ui::localized::Word::keyed("harmony.rule.highContrast2", "High Contrast 2"),
    craft_ui::localized::Word::keyed("harmony.rule.highContrast3", "High Contrast 3"),
    craft_ui::localized::Word::keyed("harmony.rule.pentagram", "Pentagram"),
];

/// What [`HarmonyWheel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct HarmonyWheelResponse {
    /// The colour whose marker was pressed; it becomes the selected one.
    pub pressed: Option<usize>,
    /// The edits of the colours this pass, each with the colour's index: a marker dragged (new hue
    /// and saturation) or, for the selected colour, the Brightness slider (new brightness); the
    /// value is `[h°, s, b]` with `None` where the edit kept the value. The host applies each value
    /// with [`move_on_wheel`](crate::harmony::move_on_wheel), linked while a rule is chosen. A pass
    /// reports the marker's edit and the slider's, in that order, when both happened. An edit with
    /// no value (a cancel) is for the selected colour.
    pub edits: Vec<(usize, EditEvent<[Option<f32>; 3]>)>,
}

/// A hue and saturation wheel at the selected colour's brightness (hue around, saturation
/// outwards), a marker per colour joined to the centre by a line (the selected one larger), and
/// a Brightness slider for the selected colour under it.
///
/// - Press a marker to select it; drag it to move that colour. Arrow keys move the selected
///   marker (←/→ hue by 1°, ↑/↓ saturation by 1 %), Tab moves between markers.
/// - AccessKit: a `Group` named by `label`; each marker a `Slider`-like node named by its hex.
pub struct HarmonyWheel<'a> {
    id_salt: IdSalt,
    label: String,
    colors: &'a [Srgba],
    selected: Option<usize>,
    size: f32,
}

impl<'a> HarmonyWheel<'a> {
    /// A wheel over `colors`; its side is five `size.thumb.lg` unless set.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, colors: &'a [Srgba]) -> HarmonyWheel<'a> {
        HarmonyWheel { id_salt: IdSalt::new(id_salt), label: label.into(), colors, selected: None, size: 0.0 }
    }

    /// Marks the colour at this index as selected; none by default.
    pub fn selected(mut self, selected: Option<usize>) -> HarmonyWheel<'a> {
        self.selected = selected;
        self
    }

    /// The wheel's side in points.
    pub fn size(mut self, size: f32) -> HarmonyWheel<'a> {
        self.size = size;
        self
    }

    // inv-vectorcraft: crates/ui-egui/src/widgets/color.rs:485 `harmony_wheel`; built on
    // `ColorWheel`'s ring geometry where it fits (`WheelGeometry`).
    /// Draws the wheel and returns its response.
    pub fn show(self, ui: &mut Ui) -> HarmonyWheelResponse {
        let Self { id_salt, label, colors, selected, size } = self;
        let labels = HarmonyLabels::of(ui);
        let tokens = Tokens::of(ui);
        let side = if size > 0.0 { size } else { tokens.size.thumb.lg * 5.0 };
        let selected = selected.filter(|&i| i < colors.len());
        let hsb = selected.map(|i| colors[i].to_hsb());
        let names: Vec<String> = colors.iter().map(|c| c.to_hex()).collect();
        let mut response = HarmonyWheelResponse::default();
        let disc = ColorDisc::new((id_salt, "disc"), label, colors)
            .selected(selected)
            .brightness(hsb.map_or(1.0, |c| c.b))
            .side(side)
            .names(&names)
            .show(ui);
        response.pressed = disc.pressed;
        if let Some(event) = disc.edit.event {
            let marker = event.value().map(|m| m.marker).or(selected);
            if let Some(marker) = marker {
                response.edits.push((marker, event.map(|m| [Some(m.hue), Some(m.saturation), None])));
            }
        }
        let b = hsb.map_or(1.0, |c| c.b);
        let track = hsb.map(|c| [Srgba::from_hsb(Hsb { b: 0.0, ..c }), Srgba::from_hsb(Hsb { b: 1.0, ..c })]);
        let numbers = LocaleNumbers::of(ui);
        let unit = Unit::new(&Words::of(ui).get(Word::plain(picker_texts::PERCENT)));
        let rule = RowRule { range: 0.0..=100.0, step: 1.0, decimals: 0 };
        let width = NumberField::fit_width(ui, ControlSize::Md, &numbers, rule.range.clone(), 0, &unit);
        let inner = Inner { policy: EditPolicy::default(), enabled: selected.is_some(), width, part: false };
        let value = (f64::from(b) * 100.0).round();
        let (field, slider) =
            row_line(ui, (id_salt, "brightness"), &labels.brightness, value, &rule, unit, &numbers, track.map(|t| t.to_vec()), false, inner);
        if let Some(i) = selected {
            for event in [field.event, slider.event].into_iter().flatten() {
                response.edits.push((i, event.map(|v| [None, None, Some((v / 100.0) as f32)])));
            }
        }
        response
    }
}

/// The Color Guide panel's settings, kept across runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HarmonyPanelState {
    /// The chosen harmony rule.
    pub rule: Harmony,
    /// The variation grid's options.
    pub options: GuideOptions,
}

/// A Color Guide panel menu entry the app carries out.
#[derive(Clone, Debug, PartialEq)]
pub enum HarmonyMenuAction {
    /// Save Colors as Swatches: the selected cells' colours, in grid order.
    SaveAsSwatches(Vec<Srgba>),
    /// Color Guide Options…: the app opens its options dialog (steps 1–20 and the Less–More
    /// amount of [`GuideOptions`]).
    Options,
}

/// What [`HarmonyPanel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct HarmonyPanelResponse {
    /// A colour of the strip or the grid was clicked: the app applies it to the selection or the
    /// current colour.
    pub chosen: Option<Srgba>,
    /// The base swatch was clicked: the app sets the base to its current colour.
    pub set_base: bool,
}

/// VectorCraft's Color Guide: the base swatch (26 points; a click sets the base to the current
/// colour) and the rule [`Select`](craft_ui::controls::Select); the harmony strip (20 points
/// high, one cell per colour, tooltip the hex); the variation grid, one row per harmony colour
/// from shades to tints (or cool to warm, muted to vivid), the harmony colour itself in the centre
/// column outlined; the grid's end labels. A click on any cell chooses its colour; Shift or Cmd
/// selects several for Save Colors as Swatches.
pub struct HarmonyPanel<'a> {
    id_salt: IdSalt,
    state: &'a mut HarmonyPanelState,
    base: Srgba,
    limit: Option<&'a [Srgba]>,
}

impl<'a> HarmonyPanel<'a> {
    /// The guide for `base`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut HarmonyPanelState, base: Srgba) -> HarmonyPanel<'a> {
        HarmonyPanel { id_salt: IdSalt::new(id_salt), state, base, limit: None }
    }

    /// Limit to Library: every colour snaps to the nearest of `palette`
    /// ([`Guide::limited`](crate::harmony::Guide::limited)).
    pub fn limit(mut self, palette: &'a [Srgba]) -> HarmonyPanel<'a> {
        self.limit = Some(palette);
        self
    }

    /// Draws the panel, editing the state in place, and returns its response.
    pub fn show(self, ui: &mut Ui) -> HarmonyPanelResponse {
        let Self { id_salt, state, base, limit } = self;
        let labels = HarmonyLabels::of(ui);
        let words = Words::of(ui);
        let tokens = Tokens::of(ui);
        let base_side = tokens.size.control.md - 2.0 * tokens.space.gap.tight;
        let ctx = ui.ctx().clone();
        let salt = IdSalt::new(id_salt);
        let mut response = HarmonyPanelResponse::default();

        ui.horizontal(|ui| {
            let chip = Swatch::new((salt, "base"), labels.set_base.as_str(), base.to_rgba()).dimensions(base_side, base_side);
            let chip = ui.add(chip).on_hover_text(labels.set_base.as_str());
            response.set_base = chip.clicked();
            let mut rule = state.rule;
            let mut select = Select::new((salt, "rule"), labels.rule.as_str(), &mut rule);
            for rule in Harmony::ALL {
                select = select.option(rule, words.get(rule_word(rule)).into_owned());
            }
            let _ = select.show(ui);
            state.rule = rule;
        });

        let key = GuideKey { base, rule: state.rule, options: state.options, limit: limit.map(<[Srgba]>::to_vec) };
        let guide = guide_of(&ctx, Id::unique(("craft_color::harmony_guide", salt)), &key);
        let picked_id = picked_key(salt);
        let mut picked =
            ctx.data(|d| d.get_temp::<Picked>(picked_id)).filter(|p| p.key == key).unwrap_or_else(|| Picked { key: key.clone(), cells: Vec::new() });

        ui.add_space(ui.spacing().item_spacing.y);
        let swatch = Swatch::style(&tokens, craft_theme::ControlSize::Md);
        // The harmony strip: one cell per colour.
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), tokens.size.control.md), egui::Sense::hover());
        let width = strip.width() / guide.colors.len().max(1) as f32;
        for (i, color) in guide.colors.iter().enumerate() {
            let rect = Rect::from_min_size(strip.min + vec2(i as f32 * width, 0.0), vec2(width, strip.height()));
            let hex = color.to_hex();
            let cell = Interactive::new((salt, "strip", i), Role::Button, hex.as_str()).interact_at(ui, rect).finish(ui);
            ui.painter().rect_filled(rect, 0.0, Color32::from(*color));
            if cell.on_hover_text(hex).clicked() {
                response.chosen = Some(*color);
            }
        }
        ui.add_space(ui.spacing().item_spacing.y);

        // The variation grid: a row per harmony colour, the colour itself in the centre column.
        let columns = guide.grid.first().map_or(1, Vec::len);
        let centre = guide.centre();
        let cell_width = ui.available_width() / columns as f32;
        let gap = if cell_width >= 8.0 { 1.0 } else { 0.0 };
        let edge = |i: usize| (i as f32 * cell_width).round();
        let multi = ui.input(|i| i.modifiers.shift || i.modifiers.command);
        for (r, row) in guide.grid.iter().enumerate() {
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), tokens.size.control.md), egui::Sense::hover());
            for (c, color) in row.iter().enumerate() {
                let rect = Rect::from_min_size(line.min + vec2(edge(c), 0.0), vec2(edge(c + 1) - edge(c) - gap, tokens.size.control.sm));
                let hex = color.to_hex();
                let on = picked.cells.contains(&(r, c));
                let cell = Interactive::new((salt, "cell", r, c), Role::Button, hex.as_str()).selected(on).interact_at(ui, rect).finish(ui);
                ui.painter().rect_filled(rect, 0.0, Color32::from(*color));
                if on {
                    ui.painter().rect_stroke(rect, 0.0, Stroke::new(swatch.selected.width, Color32::WHITE), StrokeKind::Inside);
                    ui.painter().rect_stroke(rect, 0.0, swatch.selected, StrokeKind::Outside);
                } else if c == centre {
                    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, tokens.color.text.normal.to_egui()), StrokeKind::Inside);
                }
                if cell.on_hover_text(hex).clicked() {
                    if !multi {
                        picked.cells.clear();
                        response.chosen = Some(*color);
                    }
                    match picked.cells.iter().position(|&p| p == (r, c)) {
                        Some(at) => {
                            picked.cells.remove(at);
                        }
                        None => picked.cells.push((r, c)),
                    }
                }
            }
        }
        let (left, right) = match state.options.variation {
            Variation::TintsShades => (&labels.shades, &labels.tints),
            Variation::WarmCool => (&labels.cool, &labels.warm),
            Variation::VividMuted => (&labels.muted, &labels.vivid),
        };
        ui.horizontal(|ui| {
            ui.label(left.as_str());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(right.as_str()));
        });

        picked.cells.sort_unstable();
        picked.cells.dedup();
        ctx.data_mut(|d| {
            d.insert_temp(picked_id, picked.clone());
            d.insert_temp(colors_key(salt), picked.cells.iter().filter_map(|&(r, c)| guide.grid.get(r)?.get(c).copied()).collect::<Vec<_>>());
        });
        response
    }

    /// The panel menu's rows for the dock's `panel_menu_entries`: Show Tints/Shades, Show
    /// Warm/Cool, Show Vivid/Muted (the current one a checked radio), a separator, Save Colors as
    /// Swatches (enabled while the panel `id_salt` has cells selected) and Color Guide Options….
    /// The commands are [`HarmonyPanel::MENU_COMMANDS`]; run the chosen one with
    /// [`run_command`](Self::run_command).
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, state: &HarmonyPanelState) -> Vec<MenuEntry<PanelCommand>> {
        let labels = HarmonyLabels::of_ctx(ctx);
        let has_cells = ctx.data(|d| d.get_temp::<Vec<Srgba>>(colors_key(IdSalt::new(id_salt)))).is_some_and(|c| !c.is_empty());
        let variation = |text: &str, command: &str, variation: Variation| {
            MenuItem::new(text, command.to_owned()).radio(state.options.variation == variation).into()
        };
        vec![
            variation(&labels.show_tints_shades, Self::MENU_COMMANDS[0], Variation::TintsShades),
            variation(&labels.show_warm_cool, Self::MENU_COMMANDS[1], Variation::WarmCool),
            variation(&labels.show_vivid_muted, Self::MENU_COMMANDS[2], Variation::VividMuted),
            MenuEntry::Separator,
            MenuItem::new(labels.save_as_swatches.as_str(), Self::MENU_COMMANDS[3].to_owned()).enabled(has_cells).into(),
            MenuItem::new(labels.options.as_str(), Self::MENU_COMMANDS[4].to_owned()).into(),
        ]
    }

    /// The command ids of [`panel_menu`](Self::panel_menu), in row order.
    pub const MENU_COMMANDS: [&'static str; 5] =
        ["harmony.showTintsShades", "harmony.showWarmCool", "harmony.showVividMuted", "harmony.saveAsSwatches", "harmony.options"];

    /// Runs the panel-menu `command` the dock reported: a variation is set in `state`; the others
    /// come back for the app. An id that is not one of [`MENU_COMMANDS`](Self::MENU_COMMANDS) does
    /// nothing.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, state: &mut HarmonyPanelState, command: &PanelCommand) -> Option<HarmonyMenuAction> {
        match Self::MENU_COMMANDS.iter().position(|c| *c == command.as_str())? {
            at @ 0..=2 => {
                state.options.variation = Variation::ALL[at];
                None
            }
            3 => {
                let colors = ctx.data(|d| d.get_temp::<Vec<Srgba>>(colors_key(IdSalt::new(id_salt))))?;
                (!colors.is_empty()).then_some(HarmonyMenuAction::SaveAsSwatches(colors))
            }
            _ => Some(HarmonyMenuAction::Options),
        }
    }
}

/// The word of `rule`'s name: the [`HARMONY_WORDS`] entry keyed with its id.
fn rule_word(rule: Harmony) -> Word {
    let id = format!("harmony.rule.{}", rule.id());
    HARMONY_WORDS.iter().copied().find(|w| w.id == id).unwrap_or(Word::plain(rule.id()))
}

/// What the guide is made from; selected cells belong to the guide they were picked from.
#[derive(Clone, PartialEq)]
struct GuideKey {
    base: Srgba,
    rule: Harmony,
    options: GuideOptions,
    limit: Option<Vec<Srgba>>,
}

#[derive(Clone)]
struct Picked {
    key: GuideKey,
    cells: Vec<(usize, usize)>,
}

fn picked_key(salt: IdSalt) -> Id {
    Id::unique(("craft_color::harmony_picked", salt))
}

fn colors_key(salt: IdSalt) -> Id {
    Id::unique(("craft_color::harmony_colors", salt))
}

/// The guide for `key`, kept until it changes: snapping every cell to a library is slow to redo
/// each frame.
fn guide_of(ctx: &Context, id: Id, key: &GuideKey) -> Guide {
    if let Some((k, guide)) = ctx.data(|d| d.get_temp::<(GuideKey, Guide)>(id))
        && k == *key
    {
        return guide;
    }
    let guide = Guide::new(key.base, key.rule, &key.options);
    let guide = match &key.limit {
        Some(palette) => guide.limited(palette),
        None => guide,
    };
    ctx.data_mut(|d| d.insert_temp(id, (key.clone(), guide.clone())));
    guide
}
