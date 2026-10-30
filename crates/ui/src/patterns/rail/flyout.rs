//! `FlyoutButton`: a rail button that stands for a group of choices and opens them as a flyout.

use egui::accesskit::HasPopup;
use egui::{AsIdSalt, Color32, IdSalt, Key, Modifiers, PointerButton, Popup, Response, SetOpenCommand, Shape, Stroke, Ui, pos2};

use super::gesture::{drag_open, long_press, long_press_wait, reclick_closes};
use crate::controls::{IconButton, IconButtonStyle, Kbd};
use crate::direction;
use crate::icons::Icon;
use crate::menu::{Menu, MenuEntry, MenuItem};
use crate::story::{Level, Story, StoryArgs, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;
use craft_theme::ControlSize;

/// A flyout button's sizes and colours, from [`FlyoutButton::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FlyoutStyle {
    /// The button: side `rail.button`, glyph `rail.icon`, rest colour `rail.icon_color`, hover
    /// `rail.icon.hover` on `rail.hover`, corners `rail.radius`; toggled on, the icon button's
    /// selected look.
    pub icon_button: IconButtonStyle,
    /// Legs of the indicator triangle: `rail.indicator_size`.
    pub indicator: f32,
    /// Its distance from the button's corner: `rail.indicator_inset`.
    pub indicator_inset: f32,
    /// Its colour: `rail.indicator`.
    pub indicator_color: Color32,
    /// Between the button and the list: `rail.flyout_gap`.
    pub gap: f32,
}

/// One choice in a flyout.
#[derive(Clone, Debug)]
pub struct FlyoutItem<T> {
    command: T,
    icon: Icon,
    label: String,
    shortcut: Option<String>,
    enabled: bool,
}

impl<T> FlyoutItem<T> {
    /// A choice that reports `command` when chosen. `label` is its row's text; translate it in the
    /// app.
    pub fn new(command: T, icon: Icon, label: impl Into<String>) -> Self {
        Self { command, icon, label: label.into(), shortcut: None, enabled: true }
    }

    /// The key that selects it elsewhere, shown at the row's end ("B").
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        let shortcut = shortcut.into();
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }

    /// Whether the row can be chosen; default `true`.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// What [`FlyoutButton::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FlyoutResponse<T> {
    /// The button's response.
    pub response: Response,
    /// The shown item is selected: a click on the button, Enter or Space on it, or a long press
    /// (which also opens the flyout), unless [`hold_selects`](FlyoutButton::hold_selects) is off.
    pub clicked: bool,
    /// A double click on the button; the host decides what it means.
    pub double_clicked: bool,
    /// The command of the row chosen this pass. The flyout has closed.
    pub chosen: Option<T>,
    /// The flyout is open after this pass.
    pub open: bool,
}

/// One rail button that may stand for a group of tools.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::{FlyoutButton, FlyoutItem};
/// # egui::__run_test_ui(|ui| {
/// let items = [FlyoutItem::new("brush", Icon::Brush, "Brush").shortcut("B"), FlyoutItem::new("stamp", Icon::Stamp, "Stamp")];
/// let out = FlyoutButton::new("slot", Icon::Brush, "Brush").items(&items).current(&"brush").show(ui);
/// if let Some(tool) = out.chosen {
///     // Make it the slot's tool.
/// }
/// # });
/// ```
///
/// - **Pointer:** a click selects the shown item ([`clicked`](FlyoutResponse::clicked)). A button
///   with several items opens its flyout on a secondary click, on holding the primary button for
///   0.35 s (which also selects the shown item unless [`hold_selects`](Self::hold_selects) is
///   off, as for a command button), or on pressing and dragging away from the button.
///   A click on the button of the open flyout closes it, but not the release that ends the press
///   that opened it. Releasing over a row after such a press chooses it; so does a click on a row.
///   A press outside, or Esc, closes it. No tooltip shows while a flyout is open or a button is
///   held.
/// - **Look:** the button is an [`IconButton`] drawn from the `rail.*` tokens; a button with
///   several items has a `rail.indicator` triangle at the corner facing the canvas. The list is a
///   [`Menu`] of radio rows (the current item marked, its icon, its label, its shortcut at the
///   end) beside the button towards the canvas, `rail.flyout_gap` away, never on another side (when it would leave
///   the screen it moves up by the overflow), and mirrors in a
///   right-to-left interface.
/// - **Keyboard:** the Menu key and Shift+F10 open the flyout of the focused button, as a secondary
///   click does. Then the menu's keys apply (Up, Down, Home, End, Enter, Space); Esc closes it and
///   returns the focus to the button; Tab closes it and moves on. Enter and Space on the button
///   select the shown item.
/// - **AccessKit:** a `Button` with `has_popup` menu and `expanded` while the flyout is open; the
///   flyout is the menu's `Menu` node with `MenuItemRadio` rows, `toggled` on the current one.
#[must_use = "You should call .show"]
pub struct FlyoutButton<'a, T> {
    id_salt: IdSalt,
    icon: Icon,
    label: String,
    items: &'a [FlyoutItem<T>],
    current: Option<&'a T>,
    selected: bool,
    enabled: bool,
    hold_selects: bool,
    shortcut: Option<String>,
}

impl<'a, T: Clone + PartialEq> FlyoutButton<'a, T> {
    /// A button showing `icon` and named `label` (its tooltip and accessible name). It has no
    /// flyout until [`items`](Self::items) gives it several choices.
    pub fn new(id_salt: impl AsIdSalt, icon: Icon, label: impl Into<String>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            icon,
            label: label.into(),
            items: &[],
            current: None,
            selected: false,
            enabled: true,
            hold_selects: true,
            shortcut: None,
        }
    }

    /// The choices of the flyout. With fewer than two the button is a plain button.
    pub fn items(mut self, items: &'a [FlyoutItem<T>]) -> Self {
        self.items = items;
        self
    }

    /// The item the button shows, marked in the list.
    pub fn current(mut self, current: &'a T) -> Self {
        self.current = Some(current);
        self
    }

    /// Whether the slot holds the current tool: the toggled-on look.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Whether it takes input; default `true`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether a long press also selects the shown item; default `true`, which suits tool buttons.
    /// A command button (Screen Mode: a click cycles the mode) turns it off, so a hold only opens
    /// the flyout.
    pub fn hold_selects(mut self, hold_selects: bool) -> Self {
        self.hold_selects = hold_selects;
        self
    }

    /// The key that selects the shown item elsewhere ("B"): the tooltip reads "Label  (B)" and the
    /// accessible description is the key.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        let shortcut = shortcut.into();
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }

    pub fn style(tokens: &UiTokens) -> FlyoutStyle {
        let r = &tokens.component.rail;
        let base = IconButton::style(tokens, ControlSize::Md);
        let hover = r.hover.to_egui();
        let icon_hover = r.icon_hover.to_egui();
        FlyoutStyle {
            icon_button: IconButtonStyle {
                side: r.button,
                icon: r.icon,
                radius: egui::CornerRadius::same(r.radius.round().clamp(0.0, 255.0) as u8),
                fill: craft_theme::States::new(Color32::TRANSPARENT, hover, hover, Color32::TRANSPARENT),
                text: craft_theme::States::new(r.icon_color.to_egui(), icon_hover, icon_hover, base.text.disabled),
                ..base
            },
            indicator: r.indicator_size,
            indicator_inset: r.indicator_inset,
            indicator_color: r.indicator_color.to_egui(),
            gap: r.flyout_gap,
        }
    }

    /// Draws the button and, while it is open, its flyout.
    pub fn show(self, ui: &mut Ui) -> FlyoutResponse<T> {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let popup_id = id.with("popup");
        let multi = self.enabled && self.items.len() > 1;
        let open = multi && Popup::is_id_open(&ctx, popup_id);
        let opening_press = id.with("opening-press");
        if ui.input(|i| i.pointer.any_pressed()) {
            ctx.data_mut(|d| d.remove::<bool>(opening_press));
        }
        let from_button = ctx.data(|d| d.get_temp::<bool>(opening_press)).unwrap_or(false);
        let tips = !open && !ui.input(|i| i.pointer.primary_down());
        let mut button = IconButton::new(self.icon, self.label.as_str())
            .with_id(id)
            .with_style(style.icon_button)
            .selected(self.selected)
            .enabled(self.enabled)
            .show_tooltip(tips);
        if let Some(shortcut) = &self.shortcut {
            button = button.shortcut(shortcut.as_str());
        }
        let response = ui.add(button);
        if multi && ui.is_rect_visible(response.rect) {
            paint_indicator(ui, &response, &style);
        }

        let held_for = (response.is_pointer_button_down_on() && ui.input(|i| i.pointer.button_down(PointerButton::Primary)))
            .then(|| ui.input(|i| i.pointer.press_start_time().map(|start| i.time - start)))
            .flatten();
        if multi
            && !open
            && let Some(wait) = long_press_wait(held_for)
        {
            ctx.request_repaint_after(wait);
        }
        let long = multi && !open && long_press(held_for);
        let dragged =
            multi && !open && ui.input(|i| drag_open(response.rect, i.pointer.press_origin(), i.pointer.hover_pos(), i.pointer.primary_down()));
        let by_pointer = response.clicked_by(PointerButton::Primary);
        let by_other = response.clicked() && !by_pointer;
        let keys_are_menus = open && ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));

        let mut command = None;
        let mut keyboard = false;
        let tab = open && ctx.input(|i| i.key_pressed(Key::Tab));
        // On a touch screen, holding on after the long press opened the flyout is egui's long touch,
        // which reads as a secondary click: not a click on the open flyout's button.
        let secondary = response.secondary_clicked() && !(from_button && response.long_touched());
        let close = reclick_closes(open, secondary, by_pointer, from_button) || (open && by_other && !keys_are_menus) || tab;
        if close {
            command = Some(SetOpenCommand::Bool(false));
        } else if multi && !open && (response.secondary_clicked() || long || dragged) {
            command = Some(SetOpenCommand::Bool(true));
            ctx.data_mut(|d| d.insert_temp(opening_press, long || dragged));
        } else if multi
            && !open
            && response.has_focus()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10))
        {
            command = Some(SetOpenCommand::Bool(true));
            keyboard = true;
        }

        let clicked = self.enabled && ((long && self.hold_selects) || if open { by_pointer && !from_button } else { response.clicked() });
        let double_clicked = response.double_clicked();

        let mut chosen = None;
        let mut now_open = false;
        if multi {
            // egui drops the focus on Escape before the menu sees the key; a button that held it
            // while the flyout was open gets it back when Escape closes the flyout.
            let focus_held = id.with("focus-held");
            let escaped = open && ctx.input(|i| i.key_pressed(Key::Escape));
            let entries: Vec<MenuEntry<T>> = self
                .items
                .iter()
                .map(|item| {
                    let row = MenuItem::new(item.label.clone(), item.command.clone())
                        .icon(item.icon)
                        .radio(self.current == Some(&item.command))
                        .enabled(item.enabled);
                    match &item.shortcut {
                        Some(keys) => row.shortcut(Kbd::keys(&[keys])),
                        None => row,
                    }
                    .into()
                })
                .collect();
            let popup = Popup::menu(&response)
                .open_memory(command)
                .align(direction::beside(ui))
                .align_alternatives(&[])
                .gap(style.gap)
                .accessible_name(self.label.as_str());
            let shown = Menu::new(&entries).size(ControlSize::Md).pressing(from_button).opened_by_keyboard(keyboard).show(ui, popup);
            chosen = shown.chosen;
            now_open = shown.open;
            // A press that began outside the list and its button closes it, whether or not it ends
            // in a click (a drag on the canvas is none).
            if open && now_open {
                let list = shown.popup.as_ref().map(|r| r.rect);
                let outside = ui.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer.press_origin().is_some_and(|p| !response.rect.contains(p) && !list.is_some_and(|rect| rect.contains(p)))
                });
                if outside {
                    Popup::close_id(&ctx, popup_id);
                    now_open = false;
                }
            }
            if escaped && !shown.open && ctx.data(|d| d.get_temp::<bool>(focus_held)).unwrap_or(false) {
                response.request_focus();
            } else if shown.open {
                let focused = response.has_focus();
                ctx.data_mut(|d| d.insert_temp(focus_held, focused));
            }
            ctx.accesskit_node_builder(response.id, |node| {
                node.set_has_popup(HasPopup::Menu);
                node.set_expanded(shown.open);
            });
        }
        FlyoutResponse { response, clicked, double_clicked, chosen, open: now_open }
    }
}

/// The triangle at the bottom corner facing the canvas: the right corner, the left in a
/// right-to-left interface.
fn paint_indicator(ui: &Ui, response: &Response, style: &FlyoutStyle) {
    let rect = response.rect;
    let x = direction::end_x(ui, rect);
    let f = direction::forward(ui);
    let (near, far) = (style.indicator_inset, style.indicator_inset + style.indicator);
    let corner = pos2(x - f * near, rect.bottom() - near);
    let points = vec![corner, pos2(x - f * far, rect.bottom() - near), pos2(x - f * near, rect.bottom() - far)];
    ui.painter().add(Shape::convex_polygon(points, style.indicator_color, Stroke::NONE));
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["FlyoutButton"];

/// One button standing for three tools, and the response of the last pass that had one.
#[derive(StoryArgs, Debug, PartialEq)]
struct FlyoutArgs {
    #[arg(range = 1..=3)]
    items: usize,
    selected: bool,
}

fn show_story(ui: &mut Ui, args: &mut FlyoutArgs) {
    let count = args.items;
    let selected = args.selected;
    let all = [
        FlyoutItem::new("brush", Icon::Brush, "Brush").shortcut("B"),
        FlyoutItem::new("stamp", Icon::Stamp, "Stamp").shortcut("N"),
        FlyoutItem::new("eraser", Icon::Eraser, "Eraser").shortcut("E"),
    ];
    let items = &all[..count.clamp(1, all.len())];
    let state = ui.make_persistent_id("flyout-story-state");
    let tool = ui.data(|d| d.get_temp::<&'static str>(state)).unwrap_or("brush");
    let shown = items.iter().find(|item| item.command == tool).unwrap_or(&items[0]);
    let out = FlyoutButton::new("story", shown.icon, shown.label.as_str()).items(items).current(&tool).selected(selected).show(ui);
    let mut last = ui.data(|d| d.get_temp::<String>(state.with("last"))).unwrap_or_else(|| "Nothing yet".to_owned());
    if let Some(command) = out.chosen {
        ui.data_mut(|d| d.insert_temp(state, command));
        last = format!("chosen {command}");
    } else if out.double_clicked {
        last = "double clicked".to_owned();
    } else if out.clicked {
        last = "clicked".to_owned();
    }
    ui.data_mut(|d| d.insert_temp(state.with("last"), last.clone()));
    ui.label(format!("{last}; flyout {}", if out.open { "open" } else { "closed" }));
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/rail/flyout", Level::Patterns, "FlyoutButton", "Flyout button", FlyoutArgs { items: 3, selected: true }, show_story)
            .on(Surface::Chrome),
    ]
}
