//! [`AddBesideHandles`]: the "+" buttons beside the four sides of a selected object, which add an
//! artboard (Photoshop) or a connected object (FigJam) there. One widget for the Artboard tool and
//! the connector tools (tasks SH-T8 and TK3).
//!
//! The Artboard tool runs on the tool host and gets no `Ui`: it places, hit-tests and paints the
//! buttons through [`AddBesideHandles::buttons`], [`at`](AddBesideHandles::at) and
//! [`paint`](AddBesideHandles::paint). An overlay drawn with a `Ui` calls
//! [`show`](AddBesideHandles::show), which also senses clicks and keys.

use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, CursorIcon, IdSalt, Key, Modifiers, Painter, Pos2, Sense, Stroke, Ui, UiBuilder, WidgetInfo, vec2};
use kurbo::Rect;

use craft_ui::canvas::{CanvasView, PartFocus};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::FocusRing;
use craft_ui::theme::{ToEgui, Tokens, TokensOf as _, UiTokens};

craft_ui::words! {
    /// The "+" buttons' names, English by default. The host may name the group for what it adds
    /// ("Add artboard beside").
    #[non_exhaustive]
    pub struct AddBesideLabels {
        #[allow(missing_docs)]
        group: "addBeside.group" = "Add beside";
        #[allow(missing_docs)]
        above: "addBeside.above" = "Add above";
        #[allow(missing_docs)]
        right: "addBeside.right" = "Add right";
        #[allow(missing_docs)]
        below: "addBeside.below" = "Add below";
        #[allow(missing_docs)]
        left: "addBeside.left" = "Add left";
        #[allow(missing_docs)]
        add_connected: "addBeside.addConnected" = "Add a connected shape";
    }
}

/// A side of the object, in document space (never mirrored right to left).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddSide {
    #[allow(missing_docs)]
    Top,
    #[allow(missing_docs)]
    Right,
    #[allow(missing_docs)]
    Bottom,
    #[allow(missing_docs)]
    Left,
}

impl AddSide {
    /// Clockwise from the top: the buttons' keyboard order.
    pub const ALL: [AddSide; 4] = [AddSide::Top, AddSide::Right, AddSide::Bottom, AddSide::Left];
}

/// The "+" buttons' look and place, from tokens. The same at every Size (they sit on the
/// document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AddBesideStyle {
    /// From the object's edge to a button's centre, screen points: `add_beside.offset` (18).
    pub offset: f32,
    /// A button's radius: `add_beside.radius` (8).
    pub radius: f32,
    /// The plus's arm as a share of the radius: `add_beside.arm` (0.5).
    pub arm: f32,
    /// The plus: `add_beside.line` (1.5) of `accent.solid`.
    pub plus: Stroke,
    /// The ring: `add_beside.ring` (1) of `accent.solid`.
    pub ring: Stroke,
    /// The disc: `surface.raised`.
    pub fill: Color32,
    /// A hovered or focused disc: `accent.solid`, the plus then in `text.on_strong`.
    pub hot_fill: Color32,
    /// The plus on a hovered or focused disc.
    pub hot_plus: Color32,
}

/// What [`AddBesideHandles::show`] reports.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AddBesideResponse {
    /// A button was clicked, or Enter or Space pressed on the focused group.
    pub clicked: Option<AddSide>,
    /// ⌥ was held with the click: add with contents (an artboard's copy).
    pub copy: bool,
    /// The button under the pointer (the host shows its ghost).
    pub hovered: Option<AddSide>,
    /// The group has the keyboard focus.
    pub focused: bool,
}

/// The ghost the host draws for a hovered side.
type Ghost<'a> = dyn Fn(&Painter, AddSide) + 'a;

/// The "+" buttons beside the four sides of `object` (document units).
///
/// - Place: each centred on its side, [`AddBesideStyle::offset`] screen points outside the edge,
///   through the view (a turned view turns the sides with the object).
/// - Hidden: all of them when the object on screen is smaller than two radii on a side, or
///   [`hidden`](Self::hidden) is set (a drag runs); one when its disc touches a
///   [`clear_of`](Self::clear_of) rectangle (a size label, a toolbar) or leaves the canvas.
/// - Hover: the pointing hand cursor, the hot look, and the host's [`ghost`](Self::ghost) of what
///   would be added.
/// - Keyboard ([`show`](Self::show) only): the group is one Tab stop named by
///   [`AddBesideLabels::group`] (or [`add_connected`](AddBesideLabels::add_connected) after
///   [`connecting`](Self::connecting)); arrows pick the button that lies in their screen direction, Enter or Space adds; each button is an
///   AccessKit `Button` ("Add above", …).
pub struct AddBesideHandles<'a> {
    id_salt: IdSalt,
    object: Rect,
    clear_of: &'a [egui::Rect],
    hidden: bool,
    connecting: bool,
    ghost: Option<Box<Ghost<'a>>>,
    labels: Option<AddBesideLabels>,
}

impl<'a> AddBesideHandles<'a> {
    /// The buttons around `object`, in document units.
    pub fn new(id_salt: impl AsIdSalt, object: Rect) -> Self {
        AddBesideHandles { id_salt: IdSalt::new(id_salt), object, clear_of: &[], hidden: false, connecting: false, ghost: None, labels: None }
    }

    /// Screen rectangles no button may cover.
    pub fn clear_of(mut self, rects: &'a [egui::Rect]) -> Self {
        self.clear_of = rects;
        self
    }

    /// Hides every button (while a drag runs).
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Names the group for adding a connected shape ([`AddBesideLabels::add_connected`]), as the
    /// connector tools do.
    pub fn connecting(mut self, connecting: bool) -> Self {
        self.connecting = connecting;
        self
    }

    /// Draws the host's ghost of what a hovered side would add (FigCraft: the next object and its
    /// connector, faint).
    pub fn ghost(mut self, ghost: impl Fn(&Painter, AddSide) + 'a) -> Self {
        self.ghost = Some(Box::new(ghost));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: AddBesideLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The buttons' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> AddBesideStyle {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:26-28 (`PLUS_OFFSET_PX` 18, `PLUS_RADIUS_PX`
        // 8), :274-279 (fill `card`, 1 pt accent ring, 1.5 pt plus, arm radius × 0.5).
        let (b, c) = (&tokens.component.add_beside, &tokens.color);
        let accent = c.accent.solid.to_egui();
        AddBesideStyle {
            offset: b.offset,
            radius: b.radius,
            arm: b.arm,
            plus: Stroke::new(b.line, accent),
            ring: Stroke::new(b.ring, accent),
            fill: c.surface.raised.to_egui(),
            hot_fill: accent,
            hot_plus: c.text.on_strong.to_egui(),
        }
    }

    /// The visible buttons' screen centres in [`AddSide::ALL`] order.
    pub fn buttons(&self, view: &CanvasView, style: &AddBesideStyle) -> Vec<(AddSide, Pos2)> {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:97-104 `plus_buttons` (offset over the zoom).
        // inv-figcraft: crates/ui-egui/src/canvas/board_connect.rs:45-52 `handle_at`, :57-60 (hidden
        // under two radii), :63-69 (clear of the canvas edge and the toolbar).
        let quad = view.quad(self.object.abs());
        let [top_left, top_right, bottom_right, bottom_left] = quad;
        let finite = quad.iter().all(|p| p.x.is_finite() && p.y.is_finite());
        // Side lengths, not the bounds: a turned view keeps the test the same.
        let small = (top_right - top_left).length() < 2.0 * style.radius || (bottom_left - top_left).length() < 2.0 * style.radius;
        if self.hidden || small || !finite {
            return Vec::new();
        }
        let centre = top_left.lerp(bottom_right, 0.5);
        let canvas = view.rect.shrink(style.radius);
        let middles =
            [top_left.lerp(top_right, 0.5), top_right.lerp(bottom_right, 0.5), bottom_right.lerp(bottom_left, 0.5), bottom_left.lerp(top_left, 0.5)];
        AddSide::ALL
            .into_iter()
            .zip(middles)
            .filter_map(|(side, middle)| {
                let out = (middle - centre).normalized();
                let at = middle + out * style.offset;
                let covered = self.clear_of.iter().any(|r| r.expand(style.radius).contains(at));
                (out.is_finite() && canvas.contains(at) && !covered).then_some((side, at))
            })
            .collect()
    }

    /// The button whose screen direction from `centre` is closest to `direction` (a unit vector),
    /// none when every button points away from it. A turned view turns the sides, so an arrow
    /// picks by where the button is on screen, not by its side.
    fn toward(buttons: &[(AddSide, Pos2)], centre: Pos2, direction: egui::Vec2) -> Option<AddSide> {
        buttons
            .iter()
            .map(|&(side, at)| (side, (at - centre).normalized().dot(direction)))
            .filter(|(_, alignment)| *alignment > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(side, _)| side)
    }

    /// The button at the screen point `at`, within its radius.
    pub fn at(&self, view: &CanvasView, style: &AddBesideStyle, at: Pos2) -> Option<AddSide> {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:107-112 `plus_at`.
        self.buttons(view, style).into_iter().find(|(_, centre)| centre.distance(at) <= style.radius).map(|(side, _)| side)
    }

    /// Draws the visible buttons, `hot` in the hot look with its ghost.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &AddBesideStyle, hot: Option<AddSide>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:268-281 (disc, ring, plus).
        // inv-figcraft: crates/ui-egui/src/canvas/board_connect.rs:75-82 (the hot look), :99-117 `preview`.
        for (side, centre) in self.buttons(view, style) {
            let is_hot = hot == Some(side);
            draw_button(painter, style, centre, is_hot);
            if is_hot && let Some(ghost) = &self.ghost {
                ghost(painter, side);
            }
        }
    }

    /// Draws the buttons, senses their clicks and the group's keys.
    pub fn show(self, ui: &mut Ui, view: &CanvasView) -> AddBesideResponse {
        // inv-figcraft: crates/ui-egui/src/canvas/board_connect.rs:54-92 `show`.
        // new: one focusable group (arrows pick, Enter or Space adds), `Sense::CLICK` per disc.
        let style = Self::style(&Tokens::of(ui));
        let labels = self.labels.clone().unwrap_or_else(|| AddBesideLabels::of(ui));
        let buttons = self.buttons(view, &style);
        let mut out = AddBesideResponse::default();
        let Some(&(_, first)) = buttons.first() else { return out };
        let group_id = ui.make_persistent_id(self.id_salt);
        let disc = |centre: Pos2| egui::Rect::from_center_size(centre, vec2(2.0 * style.radius, 2.0 * style.radius));
        let bounds = buttons.iter().fold(disc(first), |acc, &(_, c)| acc.union(disc(c)));
        let painter = ui.painter_at(view.rect);
        let group = ui.interact(bounds, group_id, Sense::FOCUSABLE);
        group.widget_info(|| WidgetInfo::labeled(Role::Group, ui.is_enabled(), if self.connecting { &labels.add_connected } else { &labels.group }));

        let picked_id = group_id.with("picked");
        let stored: Option<Option<AddSide>> = ui.data(|d| d.get_temp(picked_id));
        let mut picked = stored.flatten().filter(|side| buttons.iter().any(|(s, _)| s == side));
        out.focused = group.has_focus();
        if out.focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(group_id, egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() });
            });
            if picked.is_none() {
                picked = buttons.first().map(|b| b.0);
            }
            let centre = view.quad(self.object.abs()).iter().fold(Pos2::ZERO, |sum, p| sum + p.to_vec2() * 0.25);
            let arrows = [
                (Key::ArrowUp, vec2(0.0, -1.0)),
                (Key::ArrowRight, vec2(1.0, 0.0)),
                (Key::ArrowDown, vec2(0.0, 1.0)),
                (Key::ArrowLeft, vec2(-1.0, 0.0)),
            ];
            for (key, direction) in arrows {
                if let Some(side) = Self::toward(&buttons, centre, direction)
                    && ui.input_mut(|i| i.consume_key(Modifiers::NONE, key))
                {
                    picked = Some(side);
                }
            }
            for modifiers in [Modifiers::NONE, Modifiers::ALT] {
                let add = ui.input_mut(|i| i.consume_key(modifiers, Key::Enter) | i.consume_key(modifiers, Key::Space));
                if add && let Some(side) = picked {
                    out.clicked = Some(side);
                    out.copy = modifiers.alt;
                }
            }
        }
        ui.data_mut(|d| d.insert_temp(picked_id, picked));

        let ring = out.focused && FocusRing::visible(ui.ctx());
        let nodes = ui.new_child(UiBuilder::new().max_rect(bounds).accessibility_parent(group_id));
        for &(side, centre) in &buttons {
            let name = match side {
                AddSide::Top => &labels.above,
                AddSide::Right => &labels.right,
                AddSide::Bottom => &labels.below,
                AddSide::Left => &labels.left,
            };
            let response = nodes.interact(disc(centre), group_id.with(("button", side)), Sense::CLICK);
            response.widget_info(|| WidgetInfo::labeled(Role::Button, nodes.is_enabled(), name));
            let hovered = response.hovered();
            if hovered {
                out.hovered = Some(side);
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if response.clicked() {
                out.clicked = Some(side);
                out.copy = ui.input(|i| i.modifiers.alt);
            }
            let hot = hovered || (out.focused && picked == Some(side));
            draw_button(&painter, &style, centre, hot);
            if ring && picked == Some(side) {
                PartFocus::ring(&painter, disc(centre));
            }
            if hot && let Some(ghost) = &self.ghost {
                ghost(&painter, side);
            }
        }
        out
    }
}

/// One button: disc, ring and plus; `hot` swaps in the accent disc and the strong plus.
fn draw_button(painter: &Painter, style: &AddBesideStyle, centre: Pos2, hot: bool) {
    let (fill, plus) = if hot { (style.hot_fill, Stroke::new(style.plus.width, style.hot_plus)) } else { (style.fill, style.plus) };
    painter.circle(centre, style.radius, fill, style.ring);
    let arm = style.radius * style.arm;
    painter.line_segment([centre - vec2(arm, 0.0), centre + vec2(arm, 0.0)], plus);
    painter.line_segment([centre - vec2(0.0, arm), centre + vec2(0.0, arm)], plus);
}
