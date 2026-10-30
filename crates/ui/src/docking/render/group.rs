use std::{fmt::Debug, hash::Hash, sync::Arc};

use egui::{Color32, Galley, Key, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use super::{Renderer, clamp_float, stripless};
use crate::controls::{ControlSize, IconButton};
use crate::docking::{Action, DockContent, DropTarget, GroupDrag, GroupGesture, Permissions, Placement, Target, begin_drag};
use crate::icons::Icon;
use crate::tabs;
use crate::theme::TokensOf as _;

/// Where a group is drawn. It decides the drop targets the group offers and its gestures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::docking) enum GroupKind {
    /// A `Tabs` node of the split tree: side drops split it.
    Frame,
    /// A group in a dock column.
    Dock,
    /// A `Tabs` node of the floating window drawn at `window`: side drops split it.
    Floating { window: Rect },
}

/// The group a tab belongs to, for its menu and minimize gestures.
pub(super) struct TabGroup<'a, P> {
    pub(super) panels: &'a [P],
    pub(super) selected: &'a P,
    pub(super) minimized: bool,
    pub(super) kind: GroupKind,
    pub(super) rect: Rect,
}

/// The top band of a strip that drags the whole group.
const BORDER_BAND: f32 = 4.0;

/// Width of the "…" button that lists the panels a strip has no room for.
const OVERFLOW_WIDTH: f32 = 22.0;

/// Points of smooth (trackpad) scrolling that make one wheel notch.
const WHEEL_NOTCH: f32 = 40.0;

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Renderer<'_, P> {
    /// Draw one tab group at `rect`.
    ///
    /// An `active` index outside `panels` is a no-op. A lone protected panel is drawn stripless:
    /// its whole rect is the body and its only tab has `Rect::NOTHING`. Side splits are offered
    /// only for a `Frame` or `Floating` group whose selected panel accepts splits. Tab labels
    /// shrink widest-first to the minimum width, then the remaining tabs move to the overflow menu;
    /// hidden overflow tabs still appear in the drop target with `Rect::NOTHING`. The host's extra
    /// tabs follow the panel tabs and fit with them ([`fit_with_extra`]). `minimized` hides the body
    /// but keeps the strip. Tabs, strip, header and drag bar are mirrored in right-to-left layout.
    pub(super) fn group(&mut self, ui: &mut Ui, panels: &[P], active: usize, minimized: bool, rect: Rect, kind: GroupKind) {
        let Some(selected) = panels.get(active) else { return };
        self.record_float(panels, rect.size(), rect.size());
        let policy = self.permissions(selected);
        let splits = matches!(kind, GroupKind::Frame | GroupKind::Floating { .. }) && policy.accepts_splits;
        let rtl = self.config.right_to_left;
        if stripless(panels, &*self.content) {
            self.contents(ui, selected, rect);
            self.targets.push(DropTarget {
                anchor: selected.clone(),
                rect,
                tab_strip: None,
                tabs: vec![(selected.clone(), Rect::NOTHING)],
                allow_splits: splits,
                accepts_tabs: policy.accepts_tabs,
                right_to_left: rtl,
            });
            return;
        }
        let movable = panels.iter().all(|p| self.permissions(p).movable);
        let strip = Rect::from_min_max(rect.min, pos2(rect.right(), (rect.top() + self.look.tab_height).min(rect.bottom())));
        ui.painter().rect_filled(strip, 0.0, self.look.tab_strip);
        let floating = matches!(kind, GroupKind::Floating { .. });
        // Laid out left to right, then mirrored in a right-to-left layout: tabs from the right
        // edge, the header (the host's ≡) at the left.
        let mirror = |r: Rect| {
            if rtl {
                Rect::from_x_y_ranges((strip.left() + strip.right() - r.right())..=(strip.left() + strip.right() - r.left()), r.y_range())
            } else {
                r
            }
        };
        let requested = self.content.group_header_width(panels, selected, floating, strip.width());
        let header_width = if requested.is_finite() { requested.clamp(0.0, strip.width()) } else { 0.0 };
        // The panel-menu button is a square at the trailing end; the host's header sits before it
        // and the button's width comes out of the strip as the header's does.
        let menu_width = if self.config.panel_menu_button { strip.height().min(strip.width()) } else { 0.0 };
        let header_width = (header_width + menu_width).min(strip.width());
        let host_width = header_width - menu_width;
        if host_width > 0.0 {
            let header =
                mirror(Rect::from_min_max(pos2(strip.right() - header_width, strip.top()), pos2(strip.right() - menu_width, strip.bottom())));
            self.host_area(ui, "header", header, selected, |content, ui| content.group_header(ui, panels, selected, floating));
        }
        // The drag bar keeps its reserved width: tabs shrink, then overflow, before it narrows.
        let bar_width = self.config.drag_bar_width.clamp(0.0, (strip.width() - header_width).max(0.0));
        let tabs_width = (strip.width() - header_width - bar_width).max(0.0);
        let tab_style = crate::tab_strip::Tab::<P>::style(&crate::theme::Tokens::of(ui));
        let widths: Vec<f32> = panels
            .iter()
            .map(|p| {
                let close = if self.tab_closable(p) { tab_style.gap + tab_style.close_box } else { 0.0 };
                close
                    + self
                        .content
                        .label(ui, p, &self.label_of(p), &self.look.font, self.look.text)
                        .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, self.look.font.clone())
                        .size()
                        .x
                    + self.look.tab_padding
            })
            .collect();
        // A closable tab's narrowest width keeps the label the room a non-closable tab has.
        let floor_close = if panels.iter().any(|p| self.tab_closable(p)) { tab_style.gap + tab_style.close_box } else { 0.0 };
        let wanted = self.content.group_extra_tabs_width(panels, selected, floating);
        let wanted = if wanted.is_finite() { wanted.clamp(0.0, tabs_width) } else { 0.0 };
        // The widest tabs shrink first, to a common width no narrower than `min_tab_width`; only
        // when every tab at that width cannot fit do the rest go to the overflow menu. A tab
        // narrower than its name cuts it with "…" and names it in a tooltip.
        let (fit, extra_width) = fit_with_extra(&widths, active, tabs_width, wanted, self.config.tab_width_floor(self.look) + floor_close);
        let owner = TabGroup { panels, selected, minimized, kind, rect };
        if menu_width > 0.0 {
            self.panel_menu_button(ui, mirror(Rect::from_min_max(pos2(strip.right() - menu_width, strip.top()), strip.max)), &owner, policy);
        }
        let mut x = strip.left();
        let mut shown = Vec::new();
        for (index, width) in &fit.shown {
            let Some(panel) = panels.get(*index) else { continue };
            let tab = mirror(Rect::from_min_size(pos2(x, strip.top()), vec2(*width, strip.height())));
            self.tab(ui, panel, tab, *index == active, rect, Some(&owner));
            shown.push((panel.clone(), tab));
            x += width;
        }
        if !fit.overflow.is_empty() && fit.overflow_width > 0.0 {
            let overflow = mirror(Rect::from_min_size(pos2(x, strip.top()), vec2(fit.overflow_width, strip.height())));
            let response = ui.interact(overflow, self.id.with(("overflow", selected)), Sense::click());
            self.register_interaction(ui.ctx(), response.id);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &self.config.panels_label));
            ui.painter().with_clip_rect(overflow).text(overflow.center(), egui::Align2::CENTER_CENTER, "…", self.look.font.clone(), self.look.text);
            egui::Popup::menu(&response).show(|ui| {
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for index in &fit.overflow {
                        if let Some(panel) = panels.get(*index)
                            && self.menu_label(ui, panel).clicked()
                        {
                            self.output.actions.push(Action::Activate { panel: panel.clone() });
                            self.output.focused = Some(panel.clone());
                            ui.close();
                        }
                    }
                });
            });
            x += fit.overflow_width;
        }
        let mut extra = Rect::NOTHING;
        if extra_width > 0.0 {
            extra = mirror(Rect::from_min_size(pos2(x, strip.top()), vec2(extra_width, strip.height())));
            self.host_area(ui, "extra-tabs", extra, selected, |content, ui| content.group_extra_tabs(ui, panels, selected, floating));
            x += extra_width;
        }
        let bar =
            mirror(Rect::from_min_max(pos2(x.min(strip.right() - header_width), strip.top()), pos2(strip.right() - header_width, strip.bottom())));
        self.group_handle(ui, "drag-bar", bar, &owner, movable);
        // Registered last, so the band wins over the tabs below it.
        let border = mirror(Rect::from_min_max(strip.min, pos2(strip.right() - header_width, (strip.top() + BORDER_BAND).min(strip.bottom()))));
        self.group_handle(ui, "drag-border", border, &owner, movable);
        if self.config.wheel_selects_tabs {
            let tabs_area = mirror(Rect::from_min_max(strip.min, pos2(strip.right() - header_width, strip.bottom())));
            self.wheel_tabs(ui, tabs_area, extra, panels, active);
        }
        // Logical order, hidden tabs included without a hit area, for no-op detection.
        let tabs =
            panels.iter().map(|panel| (panel.clone(), shown.iter().find(|(p, _)| p == panel).map_or(Rect::NOTHING, |(_, rect)| *rect))).collect();
        self.targets.push(DropTarget {
            anchor: selected.clone(),
            rect,
            tab_strip: Some(strip),
            tabs,
            allow_splits: splits,
            accepts_tabs: policy.accepts_tabs,
            right_to_left: rtl,
        });
        if !minimized {
            self.contents(ui, selected, Rect::from_min_max(pos2(rect.left(), strip.bottom()), rect.max));
        }
        ui.painter().rect_stroke(rect, 0.0, self.look.border, StrokeKind::Inside);
    }

    /// A part of the strip the host draws (`header` or `extra-tabs`) at `rect`. A press in it
    /// focuses the shown panel, as a press on its tab does.
    fn host_area(&mut self, ui: &mut Ui, name: &'static str, rect: Rect, selected: &P, draw: impl FnOnce(&mut dyn DockContent<P>, &mut Ui)) {
        let mut child = ui.new_child(UiBuilder::new().scope_id(self.id.with((name, selected))).max_rect(rect));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        draw(&mut *self.content, &mut child);
        if child.rect_contains_pointer(rect) && child.input(|i| i.pointer.any_pressed()) {
            self.output.focused = Some(selected.clone());
        }
    }

    /// The panel-menu button at `rect`: opens the selected panel's menu, the one its tab opens.
    fn panel_menu_button(&mut self, ui: &mut Ui, rect: Rect, owner: &TabGroup<'_, P>, policy: Permissions) {
        let label = self.config.panel_menu_label.clone();
        let id = self.id.with(("panel-menu", owner.panels.first()));
        let response = IconButton::new(Icon::Menu, label).size(ControlSize::Sm).with_id(id).show_at(ui, rect);
        self.register_interaction(ui.ctx(), response.id);
        if response.clicked() {
            self.output.focused = Some(owner.selected.clone());
        }
        egui::Popup::menu(&response).show(|ui| self.panel_menu_items(ui, Some(owner), owner.selected, policy, owner.rect));
    }

    /// The drag bar (`drag-bar`: drag, double-click, and the group menu on a right click, Enter
    /// or Space) or the top border band (`drag-border`: drag only). Both move the whole group;
    /// a left click on the bar only focuses it.
    fn group_handle(&mut self, ui: &mut Ui, name: &'static str, hit: Rect, owner: &TabGroup<'_, P>, movable: bool) {
        if !crate::docking::targets::usable(hit) {
            return;
        }
        let bar = name == "drag-bar";
        let sense = if bar { Sense::click_and_drag() } else { Sense::drag() };
        let response = ui.interact(hit, self.id.with((name, owner.panels.first())), sense);
        let response = if movable { response.on_hover_cursor(egui::CursorIcon::Grab) } else { response };
        self.register_interaction(ui.ctx(), response.id);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &self.config.move_group_label));
        if movable && response.drag_started() {
            self.start_group_drag(ui, owner.panels, self.drag_source(owner));
        }
        if !bar {
            return;
        }
        if self.minimizes(owner) && response.double_clicked() {
            self.output.actions.push(Action::SetMinimized { panel: owner.selected.clone(), minimized: !owner.minimized });
        }
        if let Some(color) = self.bar_outline(&response) {
            ui.painter().with_clip_rect(hit).rect_stroke(hit.shrink(1.0), 0.0, Stroke::new(self.look.focus_width, color), StrokeKind::Inside);
        }
        let keyboard = response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        let command = if response.secondary_clicked() || keyboard {
            Some(egui::SetOpenCommand::Bool(true))
        } else if response.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        };
        egui::Popup::menu(&response).open_memory(command).show(|ui| self.group_menu(ui, owner, owner.selected));
    }

    /// The outline of a drag bar or column bar under the pointer (the handle's hover color) or
    /// with keyboard focus (the focus color, which wins), `None` otherwise.
    pub(super) fn bar_outline(&self, response: &egui::Response) -> Option<Color32> {
        if response.has_focus() {
            Some(self.look.focus)
        } else if response.hovered() {
            Some(self.look.resize.gamma_multiply(self.look.grip_alpha))
        } else {
            None
        }
    }

    /// The wheel over a group's strip: down selects the next tab, up the previous, one tab per
    /// notch, stopping at the ends. Trackpad scrolling adds up to a notch. The host's extra tabs
    /// (`extra`) keep the wheel for their own widgets.
    fn wheel_tabs(&mut self, ui: &Ui, strip: Rect, extra: Rect, panels: &[P], active: usize) {
        let key = self.id.with(("tab-wheel", panels.first()));
        if !ui.rect_contains_pointer(strip) || ui.rect_contains_pointer(extra) {
            ui.ctx().data_mut(|d| d.remove::<f32>(key));
            return;
        }
        let notches: f32 = ui.input(|i| {
            i.events
                .iter()
                .map(|event| match event {
                    egui::Event::MouseWheel { unit, delta, modifiers, .. } if !modifiers.command && !modifiers.ctrl => match unit {
                        egui::MouseWheelUnit::Point => delta.y / WHEEL_NOTCH,
                        egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => delta.y,
                    },
                    _ => 0.0,
                })
                .sum()
        });
        if notches == 0.0 || !notches.is_finite() {
            return;
        }
        let total = ui.ctx().data(|d| d.get_temp::<f32>(key)).unwrap_or(0.0) + notches;
        let steps = total.trunc();
        ui.ctx().data_mut(|d| d.insert_temp(key, total - steps));
        // A wheel turned down scrolls content up: a negative delta selects the next tab.
        let to = (active as i64).saturating_sub(steps as i64).clamp(0, panels.len().saturating_sub(1) as i64) as usize;
        if to != active
            && let Some(panel) = panels.get(to)
        {
            self.output.actions.push(Action::Activate { panel: panel.clone() });
            self.output.focused = Some(panel.clone());
        }
    }

    fn start_group_drag(&mut self, ui: &Ui, panels: &[P], rect: Rect) {
        let Some(first) = panels.first() else { return };
        let origin = ui.input(|i| i.pointer.press_origin()).or_else(|| ui.ctx().pointer_interact_pos()).unwrap_or(rect.center());
        let drag = GroupDrag {
            panel: first.clone(),
            original: [rect.left(), rect.top(), rect.width(), rect.height()],
            gesture: GroupGesture::Group,
            members: panels.to_vec(),
            origin,
            source: rect,
        };
        ui.ctx().data_mut(|d| d.insert_temp(self.id.with("group-drag"), drag));
    }

    /// The group items of the menu tail: Close Tab Group, Minimize or Expand, Float Tab Group
    /// and Move Group To. `panel` is the panel whose menu this is.
    fn group_menu(&mut self, ui: &mut Ui, owner: &TabGroup<'_, P>, panel: &P) {
        let all = |check: &dyn Fn(crate::docking::Permissions) -> bool| owner.panels.iter().all(|p| check(self.permissions(p)));
        let closable = self.config.close_menu_items && all(&|p| p.closable);
        let movable = all(&|p| p.movable);
        let floatable = movable && all(&|p| p.floatable);
        if closable && self.menu_action(ui, &self.config.close_group_label.clone()) {
            self.output.actions.push(Action::CloseGroup { panel: panel.clone() });
        }
        if self.minimizes(owner) {
            let label = if owner.minimized { self.config.expand_label.clone() } else { self.config.minimize_label.clone() };
            if self.menu_action(ui, &label) {
                self.output.actions.push(Action::SetMinimized { panel: panel.clone(), minimized: !owner.minimized });
            }
        }
        if owner.kind == GroupKind::Dock && self.menu_action(ui, &self.config.collapse_to_icons_label.clone()) {
            self.output.actions.push(Action::SetIconic { panel: panel.clone(), iconic: true });
        }
        let whole_window = matches!(owner.kind, GroupKind::Floating { .. }) && self.whole_window(panel);
        if floatable && !whole_window && self.menu_action(ui, &self.config.float_group_label.clone()) {
            let size = self.group_float_size(owner.panels, owner.rect.size());
            let rect = clamp_float(Rect::from_min_size(owner.rect.min + vec2(24.0, 24.0), size), self.float_bounds);
            self.output.actions.push(Action::MoveGroup {
                panel: panel.clone(),
                to: Target::Float { rect: Some([rect.left(), rect.top(), rect.width(), rect.height()]) },
            });
        }
        let targets = if movable { self.group_targets(owner.panels) } else { Vec::new() };
        if !targets.is_empty() {
            let logical = self.config.move_group_to_label.clone();
            let text = self.content.action_label(ui, &logical, &self.look.font, self.look.text);
            let response = ui.menu_button(text, |ui| {
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for target in &targets {
                        if self.menu_label(ui, target).clicked() {
                            self.output.actions.push(Action::MoveGroup {
                                panel: panel.clone(),
                                to: Target::At { anchor: target.clone(), placement: Placement::Tab { before: None } },
                            });
                            ui.close();
                        }
                    }
                });
            });
            response.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &logical));
        }
        self.frame_items(ui, owner, panel);
    }

    /// A stock menu button with shaped text and its logical accessibility label; true when
    /// clicked, after closing the menu.
    pub(super) fn menu_action(&mut self, ui: &mut Ui, logical: &str) -> bool {
        let text = self.content.action_label(ui, logical, &self.look.font, self.look.text);
        let response = ui.button(text);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), logical));
        if response.clicked() {
            ui.close();
        }
        response.clicked()
    }

    /// One anchor per other group that accepts a whole group as tabs, in layout order.
    fn group_targets(&self, members: &[P]) -> Vec<P> {
        let Some(first) = members.first() else { return Vec::new() };
        let mut places = Vec::new();
        let mut anchors = Vec::new();
        for anchor in self.layout.move_targets(first, |p| self.permissions(p)) {
            if members.contains(anchor) {
                continue;
            }
            let Some(place) = self.place(anchor) else { continue };
            if !places.contains(&place) {
                places.push(place);
                anchors.push(anchor.clone());
            }
        }
        anchors
    }

    /// A stock menu button for `panel` whose visible text comes from
    /// [`crate::docking::DockContent::label`] while its accessible name stays the logical `label_of`
    /// value. The function only returns the response; the caller performs the menu action.
    pub(super) fn menu_label(&mut self, ui: &mut Ui, panel: &P) -> egui::Response {
        let logical = self.label_of(panel);
        let text = self.content.label(ui, panel, &logical, &self.look.font, self.look.text);
        let response = ui.button(text);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &logical));
        response
    }

    /// A tab's label, cut at the end with '…' to fit `width`.
    pub(super) fn tab_label(&mut self, ui: &Ui, panel: &P, label: &str, color: Color32, width: f32) -> Arc<Galley> {
        let text = self.content.label(ui, panel, label, &self.look.font, color);
        crate::docking::chrome::truncated(ui, text, width, self.look.font.clone())
    }

    /// Draw one tab inside `rect` and return its response.
    ///
    /// Clicking activates and focuses the panel. In a minimized group the first click expands and
    /// records the time so that same click is not also interpreted as a double-click; a genuine
    /// double-click toggles minimized only when the owner minimizes. Middle-click closes only when
    /// the style enables it and the panel is closable. The context menu emits `Action`s, with float,
    /// move and close entries gated by the panel's permissions. `owner: None` removes minimized
    /// semantics and undock latching. An elided label exposes its full text as hover text.
    /// Whether a tab in a strip shows its ×.
    fn tab_closable(&self, panel: &P) -> bool {
        self.config.tab_close_buttons && self.permissions(panel).closable
    }

    pub(super) fn tab(&mut self, ui: &mut Ui, panel: &P, rect: Rect, active: bool, group: Rect, owner: Option<&TabGroup<'_, P>>) {
        let minimized = owner.filter(|owner| self.minimizes(owner)).map(|owner| owner.minimized);
        let policy = self.permissions(panel);
        let label = self.label_of(panel);
        let mut style = crate::tab_strip::Tab::<P>::style(&crate::theme::Tokens::of(ui));
        style.strip = self.look.tab_strip;
        style.active = self.look.tab_active;
        style.radius = self.look.tab_radius;
        style.text = self.look.text;
        style.inactive_text = self.look.inactive_text;
        style.pad_x = self.look.tab_padding * 0.5;
        let galley = self.tab_label(ui, panel, &label, style.label_color(active), f32::INFINITY);
        let tab_id = self.id.with(("tab", panel));
        let position = owner.and_then(|owner| owner.panels.iter().position(|p| p == panel)).unwrap_or(0);
        let text = crate::tab_strip::TabStripText::default();
        let view = crate::tab_strip::TabView::new(self.id, &text, position)
            .active(active)
            .focused(ui.memory(|m| m.has_focus(tab_id)))
            .sense(if policy.movable { Sense::click_and_drag() } else { Sense::click() });
        let closable = owner.is_some() && self.tab_closable(panel);
        let shown =
            crate::tab_strip::Tab::new(panel.clone(), &label).with_id(tab_id).shaped(galley).closable(closable).show_with(ui, rect, &view, &style);
        let response = shown.tab;
        if let Some(close) = shown.close {
            self.register_interaction(ui.ctx(), close.id);
            if close.clicked() {
                self.output.actions.push(Action::Close { panel: panel.clone() });
            }
        }
        self.register_interaction(ui.ctx(), response.id);
        // The click that expands a minimized group is not the first half of a double-click.
        let expand_key = self.id.with(("expanded-by-click", owner.map(|owner| owner.panels.first())));
        if response.clicked() {
            self.output.actions.push(Action::Activate { panel: panel.clone() });
            self.output.focused = Some(panel.clone());
            if minimized == Some(true) {
                // Activate already expands the group of its active tab.
                if !active {
                    self.output.actions.push(Action::SetMinimized { panel: panel.clone(), minimized: false });
                }
                let now = ui.input(|i| i.time);
                ui.ctx().data_mut(|d| d.insert_temp(expand_key, now));
            }
        }
        if let Some(minimized) = minimized
            && response.double_clicked()
        {
            let (now, window) = (ui.input(|i| i.time), ui.ctx().options(|o| o.input_options.max_double_click_delay));
            let expanded_by_first_click = ui.ctx().data_mut(|d| d.remove_temp::<f64>(expand_key)).is_some_and(|t| now - t <= window);
            if !expanded_by_first_click {
                self.output.actions.push(Action::SetMinimized { panel: panel.clone(), minimized: !minimized });
            }
        }
        if self.config.middle_click_closes_tabs && policy.closable && response.clicked_by(egui::PointerButton::Middle) {
            self.output.actions.push(Action::Close { panel: panel.clone() });
        }
        if policy.movable && response.drag_started() && begin_drag(ui.ctx(), self.id, panel.clone(), group) {
            self.latch_undock(ui.ctx(), owner.map(|owner| owner.kind));
        }
        response.context_menu(|ui| self.panel_menu_items(ui, owner, panel, policy, group));
    }

    /// The items of a panel's menu: the host's own, Float, Move Panel To, Close and, for a panel in
    /// a group, the group items. Shared by the tab's context menu and the strip's panel-menu
    /// button. `group` is the rect of the panel's group.
    fn panel_menu_items(&mut self, ui: &mut Ui, owner: Option<&TabGroup<'_, P>>, panel: &P, policy: crate::docking::Permissions, group: Rect) {
        self.content.panel_menu(ui, panel);
        if policy.movable && policy.floatable {
            let logical = self.config.float_label.clone();
            let text = self.content.action_label(ui, &logical, &self.look.font, self.look.text);
            let response = ui.button(text);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &logical));
            if response.clicked() {
                let size = self.float_size(panel, group.size());
                let rect = clamp_float(Rect::from_min_size(group.min + vec2(24.0, 24.0), size), self.float_bounds);
                self.output.actions.push(Action::Float { panel: panel.clone(), rect: [rect.left(), rect.top(), rect.width(), rect.height()] });
                ui.close();
            }
        }
        let targets: Vec<P> =
            if policy.movable { self.layout.move_targets(panel, |p| self.permissions(p)).into_iter().cloned().collect() } else { Vec::new() };
        if !targets.is_empty() {
            let text = self.content.action_label(ui, &self.config.move_panel_to_label, &self.look.font, self.look.text);
            let response = ui.menu_button(text, |ui| {
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for target in &targets {
                        if self.menu_label(ui, target).clicked() {
                            self.output.actions.push(Action::Move {
                                panel: panel.clone(),
                                anchor: target.clone(),
                                placement: Placement::Tab { before: None },
                            });
                            ui.close();
                        }
                    }
                });
            });
            response.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &self.config.move_panel_to_label));
        }
        if policy.closable && self.config.close_menu_items {
            let text = self.content.action_label(ui, &self.config.close_label, &self.look.font, self.look.text);
            let response = ui.button(text);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &self.config.close_label));
            if response.clicked() {
                self.output.actions.push(Action::Close { panel: panel.clone() });
                ui.close();
            }
        }
        if let Some(owner) = owner {
            ui.separator();
            self.group_menu(ui, owner, panel);
        }
    }
}

/// Fit the panel tabs and the host's extra tabs (`extra` wide), which fit as one more tab that
/// never overflows: in a row too narrow for every tab at its minimum width, the panel tabs
/// overflow into the room the extra tabs leave at theirs. Returns the extra tabs' width.
fn fit_with_extra(widths: &[f32], active: usize, available: f32, extra: f32, minimum: f32) -> (tabs::TabFit, f32) {
    if extra <= 0.0 {
        return (tabs::fit(widths, active, available, minimum, OVERFLOW_WIDTH), 0.0);
    }
    let all: Vec<f32> = widths.iter().copied().chain([extra]).collect();
    let mut fit = tabs::fit(&all, active, available, minimum, OVERFLOW_WIDTH);
    if fit.overflow.is_empty() {
        let width = fit.shown.pop().filter(|(index, _)| *index == widths.len()).map_or(0.0, |(_, width)| width);
        return (fit, width);
    }
    let fit = tabs::fit(widths, active, (available - extra.min(minimum)).max(0.0), minimum, OVERFLOW_WIDTH);
    let used = fit.shown.iter().map(|(_, width)| width).sum::<f32>() + fit.overflow_width;
    (fit, extra.min(available - used).max(0.0))
}
