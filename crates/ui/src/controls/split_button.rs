//! [`SplitButton`]: an action button with an arrow part that opens a menu of variants or presets.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, CornerRadius, IdSalt, Key, Modifiers, Popup, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use super::button::{Face, content_color};
use super::{Button, ButtonVariant, ControlSize, TooltipExt as _, icon_size};
use crate::direction;
use crate::icons::{self, Icon};
use crate::menu::{MenuEntry, MenuResponse, PressMenu};
use crate::primitives::{Corner, Interactive, SurfaceKind};
use crate::theme::{Tokens, TokensOf as _};

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
///   arrow part 18 points wide (VectorCraft's `SPLIT_ARROW`; `split_button.arrow_width`) with a
///   chevron down, split by a 1-point divider inset 5 points top and bottom in the button's
///   border colour. Each part has its own hover and pressed state layer; the outer corners are
///   the button's, the inner ones square.
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
        let face = Face { label: &self.label, icon: self.icon, trailing: None, size: self.size, loading: false };
        let enabled = self.enabled && ui.is_enabled();
        let laid = face.layout(ui, &tokens, &style);
        let id = ui.make_persistent_id(self.id_salt);
        let (main_id, arrow_id) = (id.with("main"), id.with("arrow"));
        let popup_id = arrow_id.with("popup");

        let (_, rect) = ui.allocate_space(vec2(laid.size.x + ARROW_WIDTH, style.height));
        let rtl = direction::is_rtl(ui);
        let split = if rtl { rect.left() + ARROW_WIDTH } else { rect.right() - ARROW_WIDTH };
        let (main_rect, arrow_rect) = if rtl {
            (Rect::from_min_max(pos2(split, rect.top()), rect.right_bottom()), Rect::from_min_max(rect.left_top(), pos2(split, rect.bottom())))
        } else {
            (Rect::from_min_max(rect.left_top(), pos2(split, rect.bottom())), Rect::from_min_max(pos2(split, rect.top()), rect.right_bottom()))
        };
        let round = Corner::Button.radius(&tokens).nw;
        let left = CornerRadius { nw: round, sw: round, ne: 0, se: 0 };
        let right = CornerRadius { nw: 0, sw: 0, ne: round, se: round };
        let (main_radius, arrow_radius) = if rtl { (right, left) } else { (left, right) };

        let ctx = ui.ctx().clone();
        let was_open = Popup::is_id_open(&ctx, popup_id);
        let part = |id, label: &str, sense, radius| {
            let mut part = Interactive::with_id(id, Role::Button, label)
                .sense(sense)
                .colors(style.colors())
                .border_width(style.border_width)
                .corner_radius(radius)
                .enabled(enabled);
            if style.surface.is_some() {
                part = part.surface(SurfaceKind::Raised);
            }
            part
        };
        let arrow_label = self.arrow_label.as_deref().unwrap_or(&self.label);
        let main_focused = ui.memory(|m| m.has_focus(main_id));
        let mut main = part(main_id, &self.label, Sense::click(), main_radius).interact_at(ui, main_rect).finish(ui);
        let mut arrow = part(arrow_id, arrow_label, Sense::CLICK, arrow_radius).expanded(was_open).interact_at(ui, arrow_rect).finish(ui);
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
            let color = content_color(ui, &style.text, &arrow, true);
            let chevron = Rect::from_center_size(arrow_rect.center(), vec2(style.icon, style.icon)).round_to_pixels(ui.pixels_per_point());
            icons::paint(ui, chevron, Icon::ChevronDown, icon_size(self.size), color);
            let divider = if enabled { style.border.rest } else { style.border.disabled };
            let x = if rtl { main_rect.left() } else { main_rect.right() };
            ui.painter().line_segment([pos2(x, rect.top() + DIVIDER_INSET), pos2(x, rect.bottom() - DIVIDER_INSET)], Stroke::new(1.0, divider));
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

/// The arrow part's width (`split_button.arrow_width` once the tokens land) and the divider's
/// inset from the top and bottom edges.
const ARROW_WIDTH: f32 = 18.0;
const DIVIDER_INSET: f32 = 5.0;
