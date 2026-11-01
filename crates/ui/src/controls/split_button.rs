//! [`SplitButton`]: an action button with an arrow part that opens a menu of variants or presets.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Key, Modifiers, Popup, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use super::button::{Face, content_color};
use super::{Button, ButtonVariant, ControlSize, TooltipExt as _, icon_size};
use crate::direction;
use crate::icons::{self, Icon};
use crate::menu::{MenuEntry, MenuResponse, PressMenu};
use crate::primitives::{Corner, Interactive, StateLayer, Surface, SurfaceEdge};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// What the button reported this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct SplitButtonResponse<T> {
    /// The main part: `clicked()` runs the action.
    pub main: Response,
    /// The arrow part.
    pub arrow: Response,
    /// The menu, and the row chosen from it.
    pub menu: MenuResponse<T>,
}

/// A button whose end opens a menu, after VectorCraft's Image Trace button and Paragraph panel
/// buttons (Illustrator's Image Trace ▾).
///
/// ```ignore
/// let presets = trace_presets.iter().map(|p| MenuItem::new(&p.name, p.id).into()).collect::<Vec<_>>();
/// let out = SplitButton::new("trace", "Image Trace", &presets).arrow_label("Tracing Presets").show(ui);
/// if out.main.clicked() { trace(current) }
/// if let Some(id) = out.menu.chosen { trace(id) }
/// ```
///
/// - **Look.** A [`Button`](crate::controls::Button) of the variant and size whose end is an
///   arrow part `split_button.arrow_width` wide (VectorCraft's `SPLIT_ARROW`) with a chevron
///   down. One surface and one border cover the whole button; each part has its own hover and
///   pressed fill, clipped to its half (the outer corners are the button's, the inner ones
///   square), and a 1-point divider, inset `split_button.divider_inset` top and bottom in the
///   button's border colour, separates them. The chevron is in the muted text colour on a
///   secondary or ghost button (VectorCraft's `text_dim`) and in the variant's text colour on a
///   filled one, where muted text would not read.
/// - **Menu.** A press on the arrow opens a [`PressMenu`](crate::menu::PressMenu) of `entries`
///   under the button, aligned to its start edge, so press-drag-release chooses in one gesture;
///   a press on the main part runs the action and never opens the menu. While the menu is open,
///   a press outside it (the main part included) only closes it (lead call L21: the closing
///   press does nothing else); a press on the arrow closes it, as an anchor toggles its menu.
/// - **Keys.** One Tab stop: Space and Enter run the action; Alt+↓ (and ↓) open the menu with
///   its first row active. Esc closes the menu and leaves the focus on the button.
/// - **AccessKit.** The main part is a `Button` named by the label; the arrow a child `Button`
///   named by [`arrow_label`](Self::arrow_label) (the label alone without it) with `has_popup`
///   menu and `expanded` while open.
pub struct SplitButton<'a, T> {
    id_salt: IdSalt,
    label: String,
    entries: &'a [MenuEntry<T>],
    arrow_label: Option<String>,
    icon: Option<Icon>,
    variant: ButtonVariant,
    size: ControlSize,
    enabled: bool,
}

impl<'a, T: Clone + Send + Sync + 'static> SplitButton<'a, T> {
    /// A split button showing `label` (as shown, translated) whose arrow opens `entries`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, entries: &'a [MenuEntry<T>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            entries,
            arrow_label: None,
            icon: None,
            variant: ButtonVariant::Secondary,
            size: ControlSize::Md,
            enabled: true,
        }
    }

    /// The arrow's tooltip and accessible name ("Tracing Presets").
    pub fn arrow_label(mut self, label: impl Into<String>) -> Self {
        self.arrow_label = Some(label.into());
        self
    }

    /// A leading icon on the main part.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // vectorcraft: crates/ui-egui/src/widgets/buttons.rs:65 split_button_named
    pub fn show(self, ui: &mut Ui) -> SplitButtonResponse<T> {
        let tokens = Tokens::of(ui);
        let style = Button::style(&tokens, self.variant, self.size);
        let split_button = &tokens.component.split_button;
        let face = Face { label: &self.label, icon: self.icon, trailing: None, size: self.size, loading: false };
        let enabled = self.enabled && ui.is_enabled();
        let laid = face.layout(ui, &tokens, &style);
        let id = ui.make_persistent_id(self.id_salt);
        let (main_id, arrow_id) = (id.with("main"), id.with("arrow"));

        let (_, rect) = ui.allocate_space(vec2(laid.size.x + split_button.arrow_width, style.height));
        let rtl = direction::is_rtl(ui);
        let split = if rtl { rect.left() + split_button.arrow_width } else { rect.right() - split_button.arrow_width };
        let (main_rect, arrow_rect) = if rtl {
            (Rect::from_min_max(pos2(split, rect.top()), rect.right_bottom()), Rect::from_min_max(rect.left_top(), pos2(split, rect.bottom())))
        } else {
            (Rect::from_min_max(rect.left_top(), pos2(split, rect.bottom())), Rect::from_min_max(pos2(split, rect.top()), rect.right_bottom()))
        };
        let round = Corner::Button.radius(&tokens).nw;
        let left = CornerRadius { nw: round, sw: round, ne: 0, se: 0 };
        let right = CornerRadius { nw: 0, sw: 0, ne: round, se: round };
        let (main_radius, arrow_radius) = if rtl { (right, left) } else { (left, right) };

        // One surface under both parts: the parts add only their hover and pressed fills, and the
        // edge is drawn once over everything.
        let whole = Corner::Button.radius(&tokens);
        let surface = style.surface.map(|kind| Surface::style(&tokens, kind));
        let rest = if enabled { style.fill.rest } else { style.fill.disabled };
        if ui.is_rect_visible(rect) && rest != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, whole, rest);
        }
        let mut colors = style.colors();
        colors.fill.rest = Color32::TRANSPARENT;
        colors.fill.disabled = Color32::TRANSPARENT;
        let part = |id, label: &str, sense, radius| {
            Interactive::with_id(id, Role::Button, label).sense(sense).colors(colors).corner_radius(radius).enabled(enabled)
        };
        let arrow_label = self.arrow_label.as_deref().unwrap_or(&self.label);
        let main_focused = ui.memory(|m| m.has_focus(main_id));
        let main_sensed = part(main_id, &self.label, Sense::click(), main_radius).interact_at(ui, main_rect);
        let arrow_sensed = part(arrow_id, arrow_label, Sense::CLICK, arrow_radius).interact_at(ui, arrow_rect);
        let popup_id = Popup::default_response_id(arrow_sensed.response());
        let ctx = ui.ctx().clone();
        let was_open = Popup::is_id_open(&ctx, popup_id);
        let mut main = main_sensed.finish(ui);
        let mut arrow = arrow_sensed.expanded(was_open).finish(ui);
        // The press that closes the menu does nothing else: the click it ends with is not the action's.
        let closing_key = id.with("closing");
        let (pressed, released) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released()));
        if pressed && was_open {
            ui.data_mut(|d| d.insert_temp(closing_key, true));
        }
        let closing = was_open || ui.data(|d| d.get_temp::<bool>(closing_key)).unwrap_or(false);
        if closing {
            main.flags.remove(egui::response::Flags::CLICKED | egui::response::Flags::FAKE_PRIMARY_CLICKED);
        }
        if released {
            ui.data_mut(|d| d.remove::<bool>(closing_key));
        }
        let key_open = enabled
            && main_focused
            && !was_open
            && ui.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowDown) | i.consume_key(Modifiers::NONE, Key::ArrowDown));

        if ui.is_rect_visible(rect) {
            face.paint(ui, main_rect, &laid, &style, content_color(ui, &style.text, &main, true));
            let color = match self.variant {
                ButtonVariant::Secondary | ButtonVariant::Ghost => {
                    if enabled {
                        tokens.color.text.muted.to_egui()
                    } else {
                        tokens.color.text.disabled.to_egui()
                    }
                }
                _ => content_color(ui, &style.text, &arrow, true),
            };
            let chevron = Rect::from_center_size(arrow_rect.center(), vec2(style.icon, style.icon)).round_to_pixels(ui.pixels_per_point());
            icons::paint(ui, chevron, Icon::ChevronDown, icon_size(self.size), color);
            // The border follows the part under the pointer, else the main part.
            let lead = if arrow.hovered() || arrow.has_focus() { &arrow } else { &main };
            if let Some(mut edge) = surface {
                edge.radius = whole;
                edge.fill = Color32::TRANSPARENT;
                if let SurfaceEdge::Border(_) = edge.edge {
                    edge.edge = SurfaceEdge::Border(Stroke::new(style.border_width, StateLayer::color(ui, &style.border, lead)));
                }
                ui.painter().extend(edge.shapes(rect, ui.pixels_per_point()));
            }
            let divider = if enabled { style.border.rest } else { style.border.disabled };
            let x = if rtl { main_rect.left() } else { main_rect.right() };
            ui.painter().line_segment(
                [pos2(x, rect.top() + split_button.divider_inset), pos2(x, rect.bottom() - split_button.divider_inset)],
                Stroke::new(1.0, divider),
            );
        }

        let mut anchor = arrow.clone();
        anchor.rect = rect;
        anchor.interact_rect = rect;
        let menu = PressMenu::new(self.entries).size(self.size).keyboard_owner(main_id, key_open).show(ui, &anchor);
        let main = laid.tooltip(main, &self.label);
        if self.arrow_label.is_some() {
            arrow = arrow.tooltip(arrow_label);
        }
        SplitButtonResponse { main, arrow, menu }
    }
}
