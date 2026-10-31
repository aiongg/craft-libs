//! The Preferences appearance page over the suite selection: [`AppearancePage`] and its
//! [`ThemeChip`]s.

use craft_theme::{Appearance, Contrast, ContrastChoice, Mode, ModeChoice, MotionChoice, Size, TextSize, ThemeDef, ThemeId, TypeRole, Variants};
use craft_ui::composites::{Form, FormGrid, LabelAlign};
use craft_ui::controls::Select;
use craft_ui::direction;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::Labels as _;
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem};
use craft_ui::primitives::{Content, Interactive, Text};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Color32, CornerRadius, Id, Key, Modifiers, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::{SelectionChange, SuiteClient};

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;

/// A chip's size.
const CHIP_SIZE: Vec2 = vec2(30.0, 26.0);
/// From the outline to the fill.
const CHIP_INSET: f32 = 3.0;
/// The chrome strip along the top of the fill.
const CHIP_STRIP: f32 = 5.0;
/// The accent dot's radius and its distance from the fill's bottom-end corner, on both axes.
const CHIP_DOT_RADIUS: f32 = 2.5;
const CHIP_DOT_OFFSET: f32 = 6.0;
/// The outline's and the fill's corner radii.
const CHIP_OUTLINE_RADIUS: u8 = 3;
const CHIP_FILL_RADIUS: f32 = 2.0;
/// The star badge's radius, and how far its centre sits inside the chip's top end corner.
const BADGE_RADIUS: f32 = 7.0;
const BADGE_INSET: Vec2 = vec2(3.0, 3.0);
/// Between two chips: room for the badge that overhangs the chip's corner.
const CHIP_GAP: f32 = 6.0;

craft_ui::words! {
    /// The appearance page's words, English by default.
    #[non_exhaustive]
    pub struct AppearanceLabels {
        appearance: "appearance.appearance" = "Appearance";
        mode: "appearance.mode" = "Appearance Mode";
        mode_system: "appearance.modeSystem" = "Sync with system";
        mode_dark: "appearance.modeDark" = "Dark";
        mode_light: "appearance.modeLight" = "Light";
        light_theme: "appearance.lightTheme" = "Light Theme";
        dark_theme: "appearance.darkTheme" = "Dark Theme";
        active: "appearance.active" = "Active";
        favorite: "appearance.favorite" = "Favorite";
        add_favorite: "appearance.addFavorite" = "Add to Favorites";
        remove_favorite: "appearance.removeFavorite" = "Remove from Favorites";
        size: "appearance.size" = "Size";
        compact: "appearance.compact" = "Compact";
        comfortable: "appearance.comfortable" = "Comfortable";
        text_size: "appearance.textSize" = "UI Font Size";
        text_small: "appearance.textSmall" = "Small";
        text_default: "appearance.textDefault" = "Default";
        text_large: "appearance.textLarge" = "Large";
        text_larger: "appearance.textLarger" = "Larger";
        contrast: "appearance.contrast" = "Contrast";
        contrast_system: "appearance.contrastSystem" = "Sync with system";
        contrast_standard: "appearance.contrastStandard" = "Standard";
        contrast_high: "appearance.contrastHigh" = "High";
        motion: "appearance.motion" = "Motion";
        motion_system: "appearance.motionSystem" = "Sync with system";
        motion_full: "appearance.motionFull" = "Full";
        motion_reduced: "appearance.motionReduced" = "Reduced";
    }
}

/// What the page reported this pass.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AppearanceResponse {
    /// The draft changed this pass (the window's Apply becomes enabled).
    pub changed: bool,
    /// The favourite star of this theme was toggled; the app updates its favourites list.
    pub favorite_toggled: Option<ThemeId>,
}

/// The Appearance group of Preferences › Interface over the suite's shared selection, after
/// PhotoCraft's page (gap 64).
///
/// Rows, top to bottom, labels left-aligned (31 Oct decision): Appearance Mode (Sync with system,
/// Dark, Light), Light Theme and Dark Theme (a row of [`ThemeChip`]s each: the built-in themes,
/// then the library's, each in the rows for the modes it has; "Active" after the chips of the row
/// for the mode the app shows now), Size (Compact, Comfortable), UI Font Size (the [`TextSize`]
/// steps), Contrast and Motion (Sync with system, Standard or Full, High or Reduced).
///
/// The page edits `draft`, a [`SelectionChange`] the Preferences model keeps; the chips and lists
/// show [`SuiteClient::shown`] with the draft applied. Nothing reaches the suite until the window
/// applies: the model's `apply` calls `client.change(draft)`, its `cancel` clears the draft
/// (PhotoCraft applies on Apply and OK too). Each chip row is one radio group: one Tab stop,
/// arrows move and pick, Space picks; a chip's context menu (and Shift+F10) has Add to Favorites
/// / Remove from Favorites, the keyboard path to the star.
///
/// ```ignore
/// impl CustomSettings<Prefs> for MyCustom {
///     fn row(&mut self, ui: &mut Ui, key: &str, _draft: &mut SettingsDraft) {
///         if key == "interface.appearance" {
///             AppearancePage::new(&self.client, &mut self.appearance).favorites(&self.favorites).show(ui);
///         }
///     }
///     fn apply(&mut self, _: &Prefs) -> Result<(), String> { self.client.change(std::mem::take(&mut self.appearance)); Ok(()) }
///     fn cancel(&mut self) { self.appearance = SelectionChange::default(); }
///     fn differs(&self) -> bool { !self.appearance.is_empty() }
/// }
/// ```
///
/// [`TextSize`]: craft_theme::TextSize
pub struct AppearancePage<'a> {
    client: &'a SuiteClient,
    draft: &'a mut SelectionChange,
    favorites: &'a [ThemeId],
    text_size: bool,
}

impl<'a> AppearancePage<'a> {
    pub fn new(client: &'a SuiteClient, draft: &'a mut SelectionChange) -> Self {
        Self { client, draft, favorites: &[], text_size: true }
    }

    /// The themes marked with a filled star.
    pub fn favorites(mut self, favorites: &'a [ThemeId]) -> Self {
        self.favorites = favorites;
        self
    }

    /// Shows the UI Font Size row (default on); an app with its own text-size control turns it
    /// off.
    pub fn text_size(mut self, show: bool) -> Self {
        self.text_size = show;
        self
    }

    // pc: crates/ui-egui/src/prefs/prefs_ui/fields.rs:315 interface_groups (appearance group);
    // pc: crates/ui-egui/src/prefs/prefs_ui/appearance.rs:311 (mode list), :268 theme_row
    pub fn show(self, ui: &mut Ui) -> AppearanceResponse {
        let Self { client, draft, favorites, text_size } = self;
        let labels = AppearanceLabels::of(ui);
        let shown = draft.applied_to(&client.shown());
        let mode = shown.mode.unwrap_or(ModeChoice::System);
        let light_active = match mode {
            ModeChoice::Light => true,
            ModeChoice::Dark => false,
            ModeChoice::System => ui.ctx().system_theme() == Some(egui::Theme::Light),
        };
        let themes = themes(client);
        let mut out = AppearanceResponse::default();
        FormGrid::new("craft_theme_store::appearance").label_align(LabelAlign::Start).show(ui, |form| {
            let modes = [
                (ModeChoice::System, labels.mode_system.as_str()),
                (ModeChoice::Dark, labels.mode_dark.as_str()),
                (ModeChoice::Light, labels.mode_light.as_str()),
            ];
            if let Some(mode) = select_row(form, "mode", &labels.mode, mode, &modes) {
                draft.mode = Some(mode);
                out.changed = true;
            }
            for dark in [false, true] {
                let (title, chosen) = if dark { (&labels.dark_theme, &shown.theme_dark) } else { (&labels.light_theme, &shown.theme_light) };
                let row = ChipRow { title, dark, active: dark != light_active, chosen: chosen.as_ref(), themes: &themes, favorites, labels: &labels };
                let (picked, toggled) = row.show(form);
                if let Some(id) = picked {
                    *(if dark { &mut draft.theme_dark } else { &mut draft.theme_light }) = Some(id);
                    out.changed = true;
                }
                out.favorite_toggled = out.favorite_toggled.take().or(toggled);
            }
            let sizes = [(Size::Compact, labels.compact.as_str()), (Size::Comfortable, labels.comfortable.as_str())];
            if let Some(size) = select_row(form, "size", &labels.size, shown.size.unwrap_or_default(), &sizes) {
                draft.size = Some(size);
                out.changed = true;
            }
            if text_size {
                let steps = [
                    (TextSize::Small, labels.text_small.as_str()),
                    (TextSize::Default, labels.text_default.as_str()),
                    (TextSize::Large, labels.text_large.as_str()),
                    (TextSize::Larger, labels.text_larger.as_str()),
                ];
                if let Some(step) = select_row(form, "text_size", &labels.text_size, shown.text_size.unwrap_or_default(), &steps) {
                    draft.text_size = Some(step);
                    out.changed = true;
                }
            }
            let contrasts = [
                (ContrastChoice::System, labels.contrast_system.as_str()),
                (ContrastChoice::Standard, labels.contrast_standard.as_str()),
                (ContrastChoice::High, labels.contrast_high.as_str()),
            ];
            if let Some(contrast) = select_row(form, "contrast", &labels.contrast, shown.contrast.unwrap_or_default(), &contrasts) {
                draft.contrast = Some(contrast);
                out.changed = true;
            }
            let motions = [
                (MotionChoice::System, labels.motion_system.as_str()),
                (MotionChoice::Full, labels.motion_full.as_str()),
                (MotionChoice::Reduced, labels.motion_reduced.as_str()),
            ];
            if let Some(motion) = select_row(form, "motion", &labels.motion, shown.motion.unwrap_or_default(), &motions) {
                draft.motion = Some(motion);
                out.changed = true;
            }
        });
        out
    }
}

/// The built-in themes, then the library's.
fn themes(client: &SuiteClient) -> Vec<ThemeDef> {
    let library = client.snapshot().index.iter().map(|entry| &entry.theme);
    craft_theme::builtins().iter().chain(library).cloned().collect()
}

/// A row of `label` and a select over `options`; the option picked this pass.
fn select_row<T: Clone + PartialEq + Send + Sync + 'static>(
    form: &mut Form<'_>,
    id: &str,
    label: &str,
    current: T,
    options: &[(T, &str)],
) -> Option<T> {
    let mut value = current.clone();
    form.row(label, |ui, _| {
        let mut select = Select::new(id, label, &mut value);
        for (option, text) in options {
            select = select.option(option.clone(), *text);
        }
        select.show(ui)
    });
    (value != current).then_some(value)
}

/// One Light Theme or Dark Theme row.
struct ChipRow<'a> {
    title: &'a str,
    dark: bool,
    /// The row's mode is the one the app shows now.
    active: bool,
    chosen: Option<&'a ThemeId>,
    themes: &'a [ThemeDef],
    favorites: &'a [ThemeId],
    labels: &'a AppearanceLabels,
}

impl ChipRow<'_> {
    /// The theme picked and the theme whose favourite star was toggled this pass.
    fn show(self, form: &mut Form<'_>) -> (Option<ThemeId>, Option<ThemeId>) {
        let mut picked = None;
        let mut toggled = None;
        form.row(self.title, |ui, _| {
            let themes: Vec<&ThemeDef> = self
                .themes
                .iter()
                .filter(|t| matches!((t.variants, self.dark), (Variants::Both, _) | (Variants::DarkOnly, true) | (Variants::LightOnly, false)))
                .collect();
            let builder = UiBuilder::new()
                .id_salt(("chips", self.dark))
                .layout(direction::row(ui))
                .accessibility_role(Role::RadioGroup)
                .accessibility_label(self.title);
            let group = ui.scope_builder(builder, |group| {
                group.spacing_mut().item_spacing.x = CHIP_GAP;
                let ids: Vec<Id> = themes.iter().map(|t| group.scope_id().with(t.id.as_str())).collect();
                let mut selected = themes.iter().position(|t| Some(&t.id) == self.chosen);
                if let Some(to) = arrow_target(group, &ids) {
                    group.memory_mut(|m| m.request_focus(ids[to]));
                    if selected != Some(to) {
                        picked = Some(themes[to].id.clone());
                        selected = Some(to);
                    }
                }
                let tab_stop = selected.unwrap_or(0);
                group.ctx().accesskit_node_builder(group.unique_id(), |node| node.set_size_of_set(themes.len()));
                for (i, theme) in themes.iter().enumerate() {
                    let favorite = self.favorites.contains(&theme.id);
                    let chip = ThemeChip {
                        theme,
                        dark: self.dark,
                        selected: selected == Some(i),
                        favorite,
                        slot: Some(Slot { id: ids[i], position: Some(i), tab_stop: i == tab_stop }),
                    };
                    let shown = chip.show_in(group, self.labels);
                    if shown.favorite_clicked {
                        toggled = Some(theme.id.clone());
                    } else if shown.response.clicked() && selected != Some(i) {
                        picked = Some(theme.id.clone());
                    }
                }
            });
            let active = self.active.then(|| ui.add(Text::new(self.labels.active.as_str()).role(TypeRole::Caption).content(Content::Accent)));
            (group.response, active)
        });
        (picked, toggled)
    }
}

/// The chip the arrow keys move to from the focused chip of a row, consuming the key. The ends
/// leave the key to the focus movement between controls.
fn arrow_target(ui: &Ui, ids: &[Id]) -> Option<usize> {
    let focused = ids.iter().position(|id| ui.memory(|m| m.has_focus(*id)))?;
    let moves = [
        (Key::ArrowLeft, direction::step(ui, Key::ArrowLeft)),
        (Key::ArrowRight, direction::step(ui, Key::ArrowRight)),
        (Key::ArrowUp, Some(direction::Step::Previous)),
        (Key::ArrowDown, Some(direction::Step::Next)),
    ];
    moves.into_iter().find_map(|(key, step)| {
        let to = match step? {
            direction::Step::Previous => focused.checked_sub(1),
            direction::Step::Next => Some(focused + 1).filter(|to| *to < ids.len()),
        }?;
        ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)).then_some(to)
    })
}

/// A 30 × 26 miniature of a theme, after PhotoCraft's theme swatch (gap 64).
///
/// The outline (corner radius 3) holds, inset 3 px, a fill of the theme's panel colour (radius
/// 2) with a 5 px strip of its chrome colour along the top and an accent dot (radius 2.5) 6 px in
/// from the bottom-right corner, all resolved for the row's mode (PhotoCraft samples `dock`,
/// `chrome` and `accent`). The outline is 2 px accent when selected, 1 px dim text on hover, else
/// 1 px card border. A star badge (radius 7, at the top-right corner offset by (−3, +3)) shows
/// filled in the accent for a favourite, and as an outline while the pointer is over the chip;
/// a click on the badge toggles the favourite, a click elsewhere picks the theme. Tooltip and
/// accessible name: the theme's name; AccessKit role `RadioButton` with its selected state.
pub struct ThemeChip<'a> {
    theme: &'a ThemeDef,
    dark: bool,
    selected: bool,
    favorite: bool,
    /// Set by the page: the chip's place in its row's radio group.
    slot: Option<Slot>,
}

/// A chip's place in a radio group.
#[derive(Clone, Copy)]
struct Slot {
    id: Id,
    /// Zero-based, for the set size the group reports.
    position: Option<usize>,
    /// Whether the chip is the group's Tab stop.
    tab_stop: bool,
}

/// What a chip reported.
#[derive(Debug)]
pub struct ThemeChipResponse {
    pub response: Response,
    /// The star badge was clicked, or the context menu's Add to Favorites / Remove from Favorites
    /// was chosen.
    pub favorite_clicked: bool,
}

impl<'a> ThemeChip<'a> {
    /// The chip of `theme` in its dark variant when `dark`, else its light one.
    pub fn new(theme: &'a ThemeDef, dark: bool) -> Self {
        Self { theme, dark, selected: false, favorite: false, slot: None }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn favorite(mut self, favorite: bool) -> Self {
        self.favorite = favorite;
        self
    }

    // pc: crates/ui-egui/src/prefs/prefs_ui/appearance.rs:218 theme_swatch (badge :255, star :264, toggle :276)
    pub fn show(self, ui: &mut Ui) -> ThemeChipResponse {
        let labels = AppearanceLabels::of(ui);
        self.show_in(ui, &labels)
    }

    fn show_in(self, ui: &mut Ui, labels: &AppearanceLabels) -> ThemeChipResponse {
        let tokens = Tokens::of(ui);
        let slot = self.slot.unwrap_or_else(|| Slot {
            id: ui.make_persistent_id(("theme_chip", self.theme.id.as_str(), self.dark)),
            position: None,
            tab_stop: true,
        });
        let name = self.theme.name.as_str();
        let colors = sample(ui.ctx(), self.theme, self.dark);
        let (_, rect) = ui.allocate_space(CHIP_SIZE);
        // Only the group's Tab stop takes keyboard focus; the arrow keys move it.
        let sense = if slot.tab_stop { Sense::click() } else { Sense::CLICK };
        let mut chip = Interactive::with_id(slot.id, Role::RadioButton, name)
            .sense(sense)
            .checked(self.selected)
            .state_layer(false)
            .corner_radius(CornerRadius::same(CHIP_OUTLINE_RADIUS));
        if let Some(position) = slot.position {
            chip = chip.position_in_set(position);
        }
        if self.favorite {
            chip = chip.value(labels.favorite.as_str());
        }
        let response = chip.interact_at(ui, rect).finish(ui).on_hover_text(name);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let inner = rect.shrink(CHIP_INSET);
            painter.rect_filled(inner, CHIP_FILL_RADIUS, colors.panel);
            painter.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width(), CHIP_STRIP)), 0.0, colors.chrome);
            let dot = pos2(direction::end_x(ui, inner) - direction::forward(ui) * CHIP_DOT_OFFSET, inner.bottom() - CHIP_DOT_OFFSET);
            painter.circle_filled(dot, CHIP_DOT_RADIUS, colors.accent);
            let stroke = if self.selected {
                Stroke::new(2.0, tokens.color.accent.solid.to_egui())
            } else if response.hovered() {
                Stroke::new(1.0, tokens.color.text.muted.to_egui())
            } else {
                Stroke::new(1.0, tokens.color.line.border.to_egui())
            };
            painter.rect_stroke(rect.shrink(1.0), CornerRadius::same(CHIP_OUTLINE_RADIUS), stroke, StrokeKind::Inside);
        }
        let action = if self.favorite { &labels.remove_favorite } else { &labels.add_favorite };
        let mut favorite_clicked = false;
        let center: Pos2 = pos2(direction::end_x(ui, rect) - direction::forward(ui) * BADGE_INSET.x, rect.top() + BADGE_INSET.y);
        let badge = Rect::from_center_size(center, Vec2::splat(BADGE_RADIUS * 2.0));
        if self.favorite || ui.rect_contains_pointer(rect.union(badge)) {
            ui.painter().circle(center, BADGE_RADIUS, tokens.color.surface.raised.to_egui(), Stroke::new(1.0, tokens.color.line.border.to_egui()));
            let (icon, tint) = if self.favorite { (Icon::StarFilled, tokens.color.accent.solid) } else { (Icon::Star, tokens.color.text.muted) };
            icons::paint(ui, badge, icon, IconSize::Sm, tint.to_egui());
            // Sensed without allocating: a widget placed at the badge would grow the row by the
            // part that overhangs the chip, and moving the chips under the pointer.
            let star = Interactive::with_id(slot.id.with("favorite"), Role::Button, format!("{action}: {name}"))
                .sense(Sense::CLICK)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, badge)
                .finish(ui);
            favorite_clicked = star.on_hover_text(action).clicked();
        }
        let entries: Vec<MenuEntry<()>> = vec![MenuItem::new(action.as_str(), ()).into()];
        favorite_clicked |= ContextMenu::new(&entries).show(ui, &response).chosen.is_some();
        ThemeChipResponse { response, favorite_clicked }
    }
}

/// The colours a chip draws for a theme in a mode.
struct Sample {
    panel: Color32,
    chrome: Color32,
    accent: Color32,
}

/// `theme` resolved in the mode, kept until the theme changes.
fn sample(ctx: &egui::Context, theme: &ThemeDef, dark: bool) -> Sample {
    let key = Id::unique("craft_theme_store::theme_chip").with((theme.id.as_str(), dark));
    if let Some((cached, [panel, chrome, accent])) = ctx.data(|d| d.get_temp::<(ThemeDef, [Color32; 3])>(key))
        && cached == *theme
    {
        return Sample { panel, chrome, accent };
    }
    let appearance = Appearance {
        mode: if dark { Mode::Dark } else { Mode::Light },
        contrast: Contrast::Standard,
        size: Size::Compact,
        text_size: TextSize::Default,
        reduced_motion: false,
    };
    let color = craft_theme::resolve(theme, appearance, None).tokens.color;
    let colors = [color.surface.panel.to_egui(), color.surface.chrome.to_egui(), color.accent.solid.to_egui()];
    ctx.data_mut(|d| d.insert_temp(key, (theme.clone(), colors)));
    Sample { panel: colors[0], chrome: colors[1], accent: colors[2] }
}
