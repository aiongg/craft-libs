//! `ReorderableSections`: the sidebar widget over a `SectionLayout`.

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{Align, Align2, AsIdSalt, CursorIcon, IdSalt, Key, Label, Modifiers, PointerButton, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

use super::{SectionEvent, SectionKey, SectionLayout};
use crate::behavior::DragBadge;
use crate::composites::{SectionHeader, SectionStyle, live};
use crate::controls::IconButton;
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::list::drop_line;
use crate::localized::Labels as _;
use crate::menu::{ContextMenu, ItemKind, MenuEntry, MenuItem};
use crate::primitives::{Content, Interactive};
use crate::theme::{ToEgui, Tokens, TokensOf as _, role_format};

/// A section header's button at the trailing edge (LightCraft: Albums' "+" opens Create Album;
/// Keywords' makes a keyword tag). A press on it never folds or drags the section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionButton {
    /// The id reported in `SectionEvent::Button`.
    pub id: &'static str,
    /// The glyph.
    pub icon: Icon,
    /// The tooltip and accessible name, translated.
    pub label: String,
}

impl SectionButton {
    /// A header button.
    pub fn new(id: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:265 (`header_button`).
        Self { id, icon, label: label.into() }
    }
}

/// What one section shows this pass: the translated title and the rest the key doesn't carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionInfo<S> {
    /// The section.
    pub section: S,
    /// Its title.
    pub title: String,
    /// Its row count, shown at the trailing edge while folded when `show_counts` is on.
    pub count: Option<usize>,
    /// The hint an open section with no rows shows ("No albums yet").
    pub empty_hint: Option<String>,
    /// The header button.
    pub button: Option<SectionButton>,
    /// `false`: the build lacks it (LightCraft's Local on the web); not shown, keeps its place.
    pub available: bool,
}

impl<S> SectionInfo<S> {
    /// An available section with no count, hint or button.
    pub fn new(section: S, title: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/sidebar.rs:23 (`SidebarSection` with `title`, `empty_hint`, `available`).
        Self { section, title: title.into(), count: None, empty_hint: None, button: None, available: true }
    }

    /// The row count.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// The empty hint.
    #[must_use]
    pub fn empty_hint(mut self, hint: impl Into<String>) -> Self {
        self.empty_hint = Some(hint.into());
        self
    }

    /// The header button.
    #[must_use]
    pub fn button(mut self, button: SectionButton) -> Self {
        self.button = Some(button);
        self
    }

    /// Whether the build has the section.
    #[must_use]
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }
}

/// What [`ReorderableSections::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the sections' events"]
pub struct SectionsOutput<S> {
    /// The area.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<SectionEvent<S>>,
    /// Each header button's response this pass, to anchor the host's popup after a `Button`.
    pub buttons: Vec<(S, Response)>,
}

crate::words! {
    /// The sections' words, English by default.
    #[non_exhaustive]
    pub struct SectionsLabels {
        hide: "sections.hide" = "Hide This Section";
        move_up: "sections.moveUp" = "Move Up";
        move_down: "sections.moveDown" = "Move Down";
        reset: "sections.reset" = "Reset";
        /// The live-region announcement after a move: `{title}`, `{position}`, `{count}`.
        moved: "sections.moved" = "{title} moved to position {position} of {count}";
    }
}

/// Sidebar sections the user folds, hides and drags (N8.03), over the host's
/// [`SectionLayout`]. The widget reports [`SectionEvent`]s; it never changes the layout.
///
/// - **Headers.** `SectionHeader`'s look (row 24, chevron 11, hairline above), one per shown and
///   available section in layout order; the title; the folded row count in `Caption`
///   `text.faint` at the trailing edge (`SectionInfo::count`, `.show_counts(true)`); the header
///   button at the trailing edge. Click, Enter or Space folds or unfolds (`Folded`).
/// - **Bodies.** `body(ui, section)` for each open section; one that adds no widget shows its
///   `empty_hint` in `Caption` `text.faint` at the row indent.
/// - **Drag.** A header drags with `Sense::click_and_drag` past egui's threshold; the drop line
///   (`list::drop_line`, `list.drop` 2 pt with its halo) shows in the gap above the section it
///   would go before, or below the last; the dragged title follows the pointer as a `DragBadge`;
///   release over the area reports `Moved { section, before }` (none when `move_before` would be
///   a no-op); release outside or Esc leaves it. A press on the header button never drags or
///   folds.
/// - **Keys.** Cmd/Ctrl+] and [ on a focused header move it one shown place later or earlier
///   (`SectionLayout::move_by`), with Shift to the ends; reported as `Moved`. After any move the
///   new place is announced in a polite live region ([`SectionsLabels::moved`]).
/// - **Menu.** On a header (secondary click, Menu key, Shift+F10): Hide This Section, Move Up,
///   Move Down (disabled at the ends), a separator, a check item per available section (shown or
///   hidden), a separator, Reset (disabled when `is_default`). The host's fixed title block opens
///   the same menu through [`ReorderableSections::menu`].
/// - **Pinned header.** The header of the open section whose own header scrolled out of the
///   enclosing `ScrollArea`'s view stays at the top (`surface.panel` fill, `line.divider` hairline
///   under it); fold, drag, menu and keys work on it; the next header pushes it up. Folding from
///   the pinned copy scrolls the section's own header into view in the same pass.
/// - **Unavailable.** A section with `available: false` is not drawn and keeps its place.
/// - **AccessKit.** Each header a `DisclosureTriangle` named by its title with `expanded`; the
///   area a `Group` named by `.label(..)`.
/// - **RTL.** Headers, chevrons, counts, buttons and the drop line mirror.
pub struct ReorderableSections<'a, S> {
    id_salt: IdSalt,
    layout: &'a SectionLayout<S>,
    show_counts: bool,
    label: String,
    labels: Option<SectionsLabels>,
}

impl<'a, S: SectionKey> ReorderableSections<'a, S> {
    /// The sections of `layout`.
    pub fn new(id_salt: impl AsIdSalt, layout: &'a SectionLayout<S>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:189 (`section_header`).
        Self { id_salt: IdSalt::new(id_salt), layout, show_counts: false, label: String::new(), labels: None }
    }

    /// Show folded sections' row counts.
    #[must_use]
    pub fn show_counts(mut self, on: bool) -> Self {
        self.show_counts = on;
        self
    }

    /// The area's accessible name.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The words.
    #[must_use]
    pub fn labels(mut self, labels: SectionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the shown sections of `sections` (each section's info; a section of the layout
    /// without info is not drawn) with `body` for each open one.
    pub fn show(self, ui: &mut Ui, sections: &[SectionInfo<S>], mut body: impl FnMut(&mut Ui, S)) -> SectionsOutput<S> {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:5 (`section`), :62 (pinned header), :118 (drop), :164 (drag feedback).
        let mut builder = UiBuilder::new().id_salt(self.id_salt).accessibility_role(Role::Group);
        if !self.label.is_empty() {
            builder = builder.accessibility_label(self.label.as_str());
        }
        let mut events = Vec::new();
        let mut buttons: Vec<(S, Response)> = Vec::new();
        let root = ui.scope_builder(builder, |ui| {
            let pass = Pass::new(&self, ui, sections);
            let style = pass.style;
            let mut placed: Vec<Placed<S>> = Vec::with_capacity(pass.visible.len());
            for section in pass.visible.clone() {
                let Some(info) = sections.iter().find(|i| i.section == section) else { continue };
                let open = !self.layout.is_folded(section);
                let start = ui.cursor().top();
                ui.add_space(style.gap);
                let hairline = style.hairline.width;
                let (_, rect) = ui.allocate_space(vec2(ui.available_width(), hairline + style.row_height));
                let row = Rect::from_min_max(pos2(rect.left(), rect.top() + hairline), rect.max);
                ui.painter().hline(rect.x_range(), (rect.top() + hairline / 2.0).round_to_pixel_center(ui.pixels_per_point()), style.hairline);
                let out = pass.header(ui, info, row, open, false);
                events.extend(out.events);
                buttons.extend(out.button.map(|b| (section, b)));
                let mut bottom = row.bottom();
                if open {
                    ui.add_space(style.gap / 2.0);
                    let rows = ui.push_id(section.id(), |ui| body(ui, section)).response;
                    if rows.rect.height() <= 0.0 {
                        pass.empty_hint(ui, info);
                    }
                    bottom = ui.cursor().top();
                }
                placed.push(Placed { section, start, header: row, bottom, open });
            }
            if let Some(pinned) = placed.iter().find(|p| p.open && p.header.top() < pass.view.top() && p.bottom > pass.view.top())
                && let Some(info) = sections.iter().find(|i| i.section == pinned.section)
            {
                let top = pass.view.top().min(pinned.bottom - style.row_height);
                let row = Rect::from_x_y_ranges(pass.x.0..=pass.x.1, top..=top + style.row_height);
                let band = Rect::from_x_y_ranges(pass.view.x_range(), row.y_range());
                ui.painter().rect_filled(band, 0.0, pass.tokens.color.surface.panel.to_egui());
                ui.painter().hline(pass.view.x_range(), row.bottom(), style.hairline);
                let out = pass.header(ui, info, row, true, true);
                if out.events.iter().any(|e| matches!(e, SectionEvent::Folded { .. })) {
                    // Folded from here: the section's own header comes into view in the same pass.
                    let own = Rect::from_x_y_ranges(pinned.header.x_range(), pinned.start..=pinned.header.bottom());
                    ui.scroll_to_rect(own, Some(Align::TOP));
                }
                events.extend(out.events);
                if let Some(button) = out.button {
                    match buttons.iter_mut().find(|(s, _)| *s == pinned.section) {
                        Some(entry) => entry.1 = button,
                        None => buttons.push((pinned.section, button)),
                    }
                }
            }
            if let Some(event) = pass.drag(ui, &placed) {
                events.push(event);
            }
            pass.announce(ui, &events);
        });
        SectionsOutput { response: root.response, events, buttons }
    }

    /// The section menu's items inside a menu the host opened (its fixed title block), for
    /// `on` (a section, or `None` from the title block: no Hide, Move Up or Move Down). Returns
    /// the chosen item's event.
    pub fn menu(
        ui: &mut Ui,
        layout: &SectionLayout<S>,
        sections: &[SectionInfo<S>],
        on: Option<S>,
        labels: &SectionsLabels,
    ) -> Option<SectionEvent<S>> {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:315 (the header and My Photos menu).
        let visible = visible_sections(layout, sections);
        let mut chosen = None;
        for entry in menu_entries(layout, sections, &visible, on, labels) {
            let item = match entry {
                MenuEntry::Separator => {
                    ui.separator();
                    continue;
                }
                MenuEntry::Item(item) => item,
            };
            let response = match item.item_kind() {
                ItemKind::Check(checked) => ui.selectable_label(checked, item.label()),
                _ => ui.add_enabled(item.is_enabled(), egui::Button::new(item.label())),
            };
            if response.clicked() {
                chosen = item.command().copied();
            }
        }
        if chosen.is_some() {
            ui.close();
        }
        chosen
    }
}

/// The shown, available sections of `layout`, top to bottom.
fn visible_sections<S: SectionKey>(layout: &SectionLayout<S>, sections: &[SectionInfo<S>]) -> Vec<S> {
    layout.shown().filter(|s| sections.iter().any(|i| i.section == *s && i.available)).collect()
}

/// The `Moved` event that takes `section` one visible place up or down (`to_end`: to the first or
/// last), or `None` when that changes nothing the user sees. Unavailable sections are stepped
/// over like hidden ones.
fn step_event<S: SectionKey>(layout: &SectionLayout<S>, visible: &[S], section: S, up: bool, to_end: bool) -> Option<SectionEvent<S>> {
    let at = visible.iter().position(|s| *s == section)?;
    let before = if up {
        if at == 0 {
            return None;
        }
        Some(visible[if to_end { 0 } else { at - 1 }])
    } else {
        if at + 1 >= visible.len() {
            return None;
        }
        if to_end { None } else { visible.get(at + 2).copied() }
    };
    moved_event(layout, visible, section, before)
}

/// `Moved { section, before }` when it changes the order of the visible sections.
fn moved_event<S: SectionKey>(layout: &SectionLayout<S>, visible: &[S], section: S, before: Option<S>) -> Option<SectionEvent<S>> {
    let order = |l: &SectionLayout<S>| l.shown().filter(|s| visible.contains(s)).collect::<Vec<_>>();
    let mut next = layout.clone();
    next.move_before(section, before);
    (order(&next) != order(layout)).then_some(SectionEvent::Moved { section, before })
}

/// The items of the section menu; each carries the event it reports.
fn menu_entries<S: SectionKey>(
    layout: &SectionLayout<S>,
    sections: &[SectionInfo<S>],
    visible: &[S],
    on: Option<S>,
    labels: &SectionsLabels,
) -> Vec<MenuEntry<SectionEvent<S>>> {
    let mut entries = Vec::new();
    if let Some(section) = on {
        entries.push(MenuItem::new(labels.hide.as_str(), SectionEvent::Hidden(section)).into());
        for (up, label) in [(true, &labels.move_up), (false, &labels.move_down)] {
            let event = step_event(layout, visible, section, up, false);
            entries.push(MenuItem::new(label.as_str(), event.unwrap_or(SectionEvent::Reset)).enabled(event.is_some()).into());
        }
        entries.push(MenuEntry::Separator);
    }
    for state in layout.states() {
        let Some(info) = sections.iter().find(|i| i.section == state.section && i.available) else { continue };
        let event = if state.hidden { SectionEvent::Shown(state.section) } else { SectionEvent::Hidden(state.section) };
        entries.push(MenuItem::new(info.title.as_str(), event).check(!state.hidden).into());
    }
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(labels.reset.as_str(), SectionEvent::Reset).enabled(!layout.is_default()).into());
    entries
}

/// Where a section was laid out this pass (screen y).
struct Placed<S> {
    section: S,
    /// The top of the gap above its header.
    start: f32,
    header: Rect,
    /// The end of its rows (or of the header while folded).
    bottom: f32,
    open: bool,
}

/// What one header reported this pass.
struct HeaderOut<S> {
    events: Vec<SectionEvent<S>>,
    button: Option<Response>,
}

/// What every header of one `show` shares.
struct Pass<'a, S> {
    layout: &'a SectionLayout<S>,
    sections: &'a [SectionInfo<S>],
    labels: SectionsLabels,
    tokens: std::sync::Arc<crate::theme::UiTokens>,
    style: SectionStyle,
    show_counts: bool,
    /// The shown, available sections, top to bottom.
    visible: Vec<S>,
    /// The enclosing scroll area's visible part.
    view: Rect,
    /// The x range the sections are laid out in.
    x: (f32, f32),
    /// The widget's own id: the drag and the announcement hang off it.
    id: egui::Id,
}

impl<'a, S: SectionKey> Pass<'a, S> {
    fn new(widget: &ReorderableSections<'a, S>, ui: &Ui, sections: &'a [SectionInfo<S>]) -> Self {
        let tokens = Tokens::of(ui);
        let free = ui.available_rect_before_wrap();
        Self {
            layout: widget.layout,
            sections,
            labels: widget.labels.clone().unwrap_or_else(|| SectionsLabels::of(ui)),
            style: SectionHeader::style(&tokens, ui.pixels_per_point()),
            tokens,
            show_counts: widget.show_counts,
            visible: visible_sections(widget.layout, sections),
            view: ui.clip_rect(),
            x: (free.left(), free.right()),
            id: ui.scope_id(),
        }
    }

    fn dragging(&self, ctx: &egui::Context) -> Option<S> {
        ctx.data(|d| d.get_temp::<S>(self.id.with("drag")))
    }

    fn title_of(&self, section: S) -> &str {
        self.sections.iter().find(|i| i.section == section).map_or("", |i| i.title.as_str())
    }

    /// One header in `row`: the chevron, the title, the folded count and the button; folding,
    /// the drag's start, the keys and the menu. `pinned`: the copy kept at the top of the view.
    fn header(&self, ui: &mut Ui, info: &SectionInfo<S>, row: Rect, open: bool, pinned: bool) -> HeaderOut<S> {
        let section = info.section;
        let style = self.style;
        let rtl = direction::is_rtl(ui);
        let ctx = ui.ctx().clone();
        let mut events = Vec::new();

        let salt = IdSalt::new((if pinned { "pinned" } else { "header" }, section.id()));
        let sensed = Interactive::salted(salt, Role::DisclosureTriangle, info.title.as_str())
            .sense(Sense::click_and_drag())
            .expanded(open)
            .state_layer(false)
            .interact_at(ui, row);
        let clicked = sensed.response().clicked();
        let response = sensed.finish(ui).on_hover_cursor(CursorIcon::PointingHand);
        if clicked {
            events.push(SectionEvent::Folded { section, folded: open });
            ctx.accesskit_node_builder(response.id, |node| node.set_expanded(!open));
        }

        // A press on the header's button is the button's, also when it turns into a drag.
        let button_rect = info.button.as_ref().map(|_| {
            let side = style.row_height;
            let end = direction::end_x(ui, row);
            let start = end - direction::forward(ui) * side;
            Rect::from_x_y_ranges(start.min(end)..=start.max(end), row.y_range())
        });
        let on_button = button_rect.is_some_and(|r| ui.input(|i| i.pointer.press_origin()).is_some_and(|p| r.contains(p)));
        if response.drag_started_by(PointerButton::Primary) && !on_button {
            ctx.data_mut(|d| d.insert_temp(self.id.with("drag"), section));
        }

        if response.has_focus() {
            let visible = self.visible.clone();
            let layout = self.layout;
            let step = ui.input_mut(|i| {
                [(Key::CloseBracket, false), (Key::OpenBracket, true)].into_iter().find_map(|(key, up)| {
                    let curly = if up { Key::OpenCurlyBracket } else { Key::CloseCurlyBracket };
                    let to_end =
                        i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, curly) || i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, key);
                    (to_end || i.consume_key(Modifiers::COMMAND, key)).then_some((up, to_end))
                })
            });
            if let Some((up, to_end)) = step {
                events.extend(step_event(layout, &visible, section, up, to_end));
            }
        }

        let entries = menu_entries(self.layout, self.sections, &self.visible, Some(section), &self.labels);
        events.extend(ContextMenu::new(&entries).show(ui, &response).chosen);

        // Chevron, title and count.
        let x = |offset: f32| if rtl { row.right() - offset } else { row.left() + offset };
        let chevron = Rect::from_center_size(pos2(x(1.0 + style.chevron_box / 2.0), row.center().y), vec2(style.chevron_box, style.chevron_box));
        icons::paint_in(
            ui,
            chevron,
            if open { Icon::ChevronDown } else { Icon::ChevronRight },
            icons::active(&ctx),
            style.chevron,
            IconSize::Sm.stroke(),
            style.chevron_color,
        );
        let mut end = direction::end_x(ui, row);
        if let Some(r) = button_rect {
            end = direction::start_x(ui, r) - direction::forward(ui) * self.tokens.space.gap.inline;
        }
        if !open
            && self.show_counts
            && let Some(count) = info.count
        {
            let color = Content::Faint.color(&self.tokens).to_egui();
            let align = if rtl { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER };
            let at = ui.painter().text(pos2(end, row.center().y), align, count.to_string(), self.tokens.font(style.caption_role), color);
            end = direction::start_x(ui, at) - direction::forward(ui) * self.tokens.space.gap.inline;
        }
        let start = x(style.title_offset);
        if direction::forward(ui) * (end - start) >= 1.0 {
            let band = Rect::from_x_y_ranges(start.min(end)..=start.max(end), row.y_range());
            let format = role_format(&self.tokens, style.title_role, style.title);
            let text = egui::text::LayoutJob::single_section(info.title.clone(), format);
            ui.scope_builder(UiBuilder::new().max_rect(band).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(text).truncate().selectable(false));
            });
        }

        let button = info.button.as_ref().zip(button_rect).map(|(button, rect)| {
            let id = if pinned { "pinnedButton" } else { "button" };
            let response =
                IconButton::new(button.icon, button.label.as_str()).id_salt((id, section.id(), button.id)).side(style.row_height).show_at(ui, rect);
            if response.clicked() {
                events.push(SectionEvent::Button { section, id: button.id });
            }
            response
        });
        HeaderOut { events, button }
    }

    /// An open section with nothing in it says so at the title's indent.
    fn empty_hint(&self, ui: &mut Ui, info: &SectionInfo<S>) {
        let Some(hint) = &info.empty_hint else { return };
        let format = role_format(&self.tokens, TypeRole::Caption, Content::Faint.color(&self.tokens).to_egui());
        ui.horizontal(|ui| {
            ui.add_space(self.style.title_offset);
            ui.add(Label::new(egui::text::LayoutJob::single_section(hint.clone(), format)).selectable(false));
        });
    }

    /// While a header is dragged: the drop line and the title that follows the pointer; the
    /// release over the area reports the move, Esc or a release elsewhere leaves it.
    fn drag(&self, ui: &mut Ui, placed: &[Placed<S>]) -> Option<SectionEvent<S>> {
        let ctx = ui.ctx().clone();
        let drag_id = self.id.with("drag");
        let dragged = self.dragging(&ctx)?;
        let (down, released, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.primary_released(), i.pointer.latest_pos()));
        let escape = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if escape || (!down && !released) {
            ctx.data_mut(|d| d.remove::<S>(drag_id));
            return None;
        }
        let over = pos.filter(|p| self.view.contains(*p) && (self.x.0..=self.x.1).contains(&p.x));
        let before = over.and_then(|p| placed.iter().find(|s| p.y < (s.header.top() + s.bottom) / 2.0).map(|s| s.section));
        if released {
            ctx.data_mut(|d| d.remove::<S>(drag_id));
            return over.and_then(|_| moved_event(self.layout, &self.visible, dragged, before));
        }
        if over.is_some() {
            let gap = self.style.gap;
            let y = match before {
                Some(b) => placed.iter().find(|s| s.section == b).map(|s| s.start + gap / 2.0),
                None => placed.last().map(|s| s.bottom + gap / 2.0),
            };
            if let Some(y) = y {
                drop_line(ui, pos2(self.x.0, y), pos2(self.x.1, y));
            }
        }
        DragBadge::new((self.id, "drag"), self.title_of(dragged)).show(&ctx);
        None
    }

    /// Announces where a moved section is now. The host applies the move after this pass, so
    /// the position is read on the next one; the announcement then stays in the tree (a screen
    /// reader reads it when its text changes) until the next event.
    fn announce(&self, ui: &mut Ui, events: &[SectionEvent<S>]) {
        let ctx = ui.ctx().clone();
        let (pending, text) = (self.id.with("moved"), self.id.with("announcement"));
        if let Some(SectionEvent::Moved { section, .. }) = events.iter().rev().find(|e| matches!(e, SectionEvent::Moved { .. })) {
            ctx.data_mut(|d| d.insert_temp(pending, *section));
            ctx.request_repaint();
            return;
        }
        if let Some(section) = ctx.data(|d| d.get_temp::<S>(pending)) {
            ctx.data_mut(|d| d.remove::<S>(pending));
            if let Some(position) = self.visible.iter().position(|s| *s == section) {
                let (position, count) = ((position + 1).to_string(), self.visible.len().to_string());
                let said = craft_i18n::fmt(&self.labels.moved, &[("title", self.title_of(section)), ("position", &position), ("count", &count)]);
                ctx.data_mut(|d| d.insert_temp(text, said));
            }
        } else if !events.is_empty() {
            ctx.data_mut(|d| d.remove::<String>(text));
        }
        if let Some(said) = ctx.data(|d| d.get_temp::<String>(text)) {
            live::region(ui, self.id.with("live"), Role::Status, live::Politeness::Polite, |_, labels| labels.add(said));
        }
    }
}
