//! [`PreferencesBody`]: the contents of the Preferences window in any `Ui`.

use craft_theme::{ControlSize, Tone, TypeRole};
use egui::accesskit::Role;
use egui::{
    Align, AsIdSalt, Context, Event, Id, IdSalt, Key, KeyboardShortcut, Layout, Modifiers, Rect, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2,
};

use super::model::{Closed, Columns, PreferencesOutput, PreferencesTexts};
use super::model_trait::PreferencesModel;
use super::rows::{RowsOut, ShownRow, show_row, typed_value};
use super::search::{self, Listed};
use super::state::PreferencesState;
use crate::behavior::{RowInfo, Selection, SelectionMode};
use crate::composites::{EmptyState, GroupBox, Notice};
use crate::controls::{Button, SearchField};
use crate::dialog::{ButtonRole, EnterRule, FooterButton, FooterButtons, KeyInput, Platform, resolve_keys};
use crate::direction;
use crate::list::{List, ListEvent, ListRow};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::primitives::{Content, Divider, Text};
use crate::theme::{Tokens, TokensOf as _};

/// The sizes of the body, from [`PreferencesBody::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PreferencesStyle {
    /// Width of the page list column: `preferences.list_width`.
    pub list_width: f32,
    /// Between the page list and the page: `preferences.page_gap`.
    pub page_gap: f32,
    /// Between the search field, the panes, the notice and the buttons, and between rows:
    /// `gap.stack`.
    pub gap: f32,
    /// Width of a choice row's control: `preferences.choice_width`.
    pub choice_width: f32,
    /// Width of a text row's control: `preferences.text_width`.
    pub text_width: f32,
    /// Between the divider and the page's contents, and between them and the trailing edge:
    /// `preferences.page_inset`.
    pub page_inset: f32,
    /// Height of a page list row: `preferences.page_row_height`.
    pub page_row_height: f32,
    /// Width of a colour row's hex field: `preferences.hex_width`.
    pub hex_width: f32,
}

/// What a group of the shown page draws.
struct ShownGroup {
    title: Option<String>,
    columns: Columns,
    rows: Vec<ShownRow>,
}

/// The page to draw, copied from the host's model.
struct ShownPage {
    id: String,
    title: String,
    custom: bool,
    footnote: Option<String>,
    groups: Vec<ShownGroup>,
}

/// The Preferences window's contents: a search field over a page list and the current page, with
/// OK, Cancel and Apply under them. The window and the stories draw this; it fills the rect `ui` has left.
///
/// ```
/// # use craft_ui::preferences::{PreferencesBody, PreferencesModel};
/// # fn draw(ui: &mut egui::Ui, host: &mut impl PreferencesModel) {
/// let out = PreferencesBody::new("preferences").show(ui, host);
/// if out.closed.is_some() { /* close the window */ }
/// # }
/// ```
///
/// - **Pages and search.** The host builds the pages each pass ([`PreferencesModel::pages`]); the
///   body filters them with [`matches`](super::matches), lists the pages that match (the empty
///   ones dimmed) and shows the selected one, or the first listed when a search hides it.
/// - **Rows.** Each row is the library's control for its [`RowKind`](super::RowKind) in a
///   [`Field`](crate::composites::Field); an edit goes to [`PreferencesModel::set`]. Nothing is
///   saved before OK or Apply.
/// - **Buttons.** OK applies and closes, Apply applies and stays (enabled while the draft
///   differs), Cancel drops the draft. A failed apply shows its message over the buttons. Enter is OK,
///   Esc cancels, once a popup, the search text and a field's own gesture have had it. Cmd/Ctrl+F
///   focuses the search field.
/// - **Tab** visits the controls in layout order: the search field, the page list (one stop), the
///   rows, the buttons.
/// - **AccessKit:** the search field is a `SearchInput`, the page list a `ListBox` of the listed
///   pages, the page a `Group` named by its title, and the empty-search text a polite live region.
#[must_use = "show the body with `show`"]
pub struct PreferencesBody {
    id_salt: IdSalt,
    active: bool,
    cancel: bool,
    platform: Option<Platform>,
}

impl PreferencesBody {
    /// A body; `id_salt` keeps its page, search text and scroll, under the id of the `Ui` it is
    /// shown in.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), active: true, cancel: false, platform: None }
    }

    /// The id of the body of `id_salt` shown in `ui`, for [`state`](PreferencesBody::state).
    pub fn id(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// Whether the body takes Enter, Esc and Cmd/Ctrl+F this pass; on by default. A host turns it off
    /// while another window or dialog has the keys.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The window asked to close (its close button): the body cancels as if Cancel was pressed.
    pub fn cancel_requested(mut self, cancel: bool) -> Self {
        self.cancel = cancel;
        self
    }

    /// The button order to follow; the context's platform by default.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn style(tokens: &crate::theme::UiTokens) -> PreferencesStyle {
        let p = &tokens.component.preferences;
        PreferencesStyle {
            list_width: p.list_width,
            page_gap: p.page_gap,
            gap: tokens.space.gap.stack,
            choice_width: p.choice_width,
            text_width: p.text_width,
            page_inset: p.page_inset,
            page_row_height: p.page_row_height,
            hex_width: p.hex_width,
        }
    }

    /// What the body with `id` ([`PreferencesBody::id`]) remembers.
    pub fn state(ctx: &Context, id: Id) -> PreferencesState {
        PreferencesState::load(ctx, id)
    }

    /// Replaces what the body with `id` remembers.
    pub fn set_state(ctx: &Context, id: Id, state: PreferencesState) {
        state.store(ctx, id);
    }

    /// Shows the body in the rect `ui` has left, which it fills.
    pub fn show<H: PreferencesModel + ?Sized>(self, ui: &mut Ui, host: &mut H) -> PreferencesOutput {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(self.id_salt);
        let mut state = PreferencesState::load(&ctx, id);
        let texts = PreferencesTexts::of(ui);
        let numbers = LocaleNumbers::of(ui);
        let style = PreferencesBody::style(&tokens);
        let gap = style.gap;
        let keys = self.active && !host.nested_open();
        let mut out = PreferencesOutput::default();

        let find = keys && ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::F)));
        let search_before = state.search.clone();
        let search_focused = state.search_focused;

        // Justified, so that the field fills the width.
        let search = ui
            .allocate_ui_with_layout(vec2(ui.available_width(), 0.0), Layout::top_down_justified(Align::Min), |ui| {
                SearchField::new((id, "search"), texts.search_label.as_str(), &state.search, texts.clear_search.as_str())
                    .placeholder(texts.search_placeholder.as_str())
                    .show(ui)
            })
            .inner;
        if let Some(value) = typed_value(search.event) {
            state.search = value;
        }
        if find {
            search.response.request_focus();
        }
        state.search_focused = search.response.has_focus();
        ui.add_space(gap);

        // The page model, copied so that the host can be borrowed again to draw custom rows.
        let query = state.search.clone();
        let (page_list, shown_page) = {
            let pages = host.pages();
            let listed = search::list_pages(&pages, &query);
            let list: Vec<(String, String, bool)> =
                listed.iter().map(|l| (pages[l.index].id.to_owned(), pages[l.index].title.to_owned(), l.dimmed)).collect();
            let shown = search::shown_position(&listed, &pages, &state.selected).map(|at| {
                let (listed, page) = (listed[at], &pages[listed[at].index]);
                shown_page(&query, &listed, page)
            });
            (list, shown)
        };

        let rect = ui.available_rect_before_wrap();
        let rect = if rect.height().is_finite() {
            rect
        } else {
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), tokens.component.preferences.body_fallback_height))
        };
        let button_height = tokens.component.button.height.get(ControlSize::Md);
        let notice_height = if state.error.is_some() { state.notice_height + gap } else { 0.0 };
        let footer_height = button_height + gap + notice_height;
        let content = Rect::from_min_max(rect.min, pos2(rect.max.x, (rect.max.y - footer_height).max(rect.min.y)));
        let list_width = style.list_width.min(content.width() / 2.0);
        let page_gap = style.page_gap;
        let inset = page_gap / 2.0 + style.page_inset;
        let rtl = direction::is_rtl(ui);
        let (list_rect, page_rect, divider_x) = if rtl {
            (
                Rect::from_min_max(pos2(content.right() - list_width, content.top()), content.max),
                Rect::from_min_max(
                    pos2(content.left() + style.page_inset, content.top()),
                    pos2(content.right() - list_width - inset, content.bottom()),
                ),
                content.right() - list_width - page_gap / 2.0,
            )
        } else {
            (
                Rect::from_min_max(content.min, pos2(content.left() + list_width, content.bottom())),
                Rect::from_min_max(
                    pos2(content.left() + list_width + inset, content.top()),
                    pos2(content.right() - style.page_inset, content.bottom()),
                ),
                content.left() + list_width + page_gap / 2.0,
            )
        };
        Divider::paint(ui.painter(), pos2(divider_x, content.top()), pos2(divider_x, content.bottom()), &tokens);

        // The page list.
        let mut list_ui = ui.new_child(UiBuilder::new().id_salt("pages").max_rect(list_rect).layout(direction::column(ui)));
        list_ui.set_clip_rect(list_rect.intersect(ui.clip_rect()));
        if page_list.is_empty() {
            EmptyState::new(texts.no_matches.as_str()).status().show(&mut list_ui);
        } else {
            let order: Vec<RowInfo<String>> = page_list.iter().map(|(id, _, _)| RowInfo::leaf(id.clone())).collect();
            let current = shown_page.as_ref().map(|p| p.id.clone()).unwrap_or_default();
            let mut selection = Selection::only(current);
            let listed = List::new((id, "pages"), texts.pages_label.as_str(), order.as_slice())
                .selection(&mut selection)
                .mode(SelectionMode::Single)
                .allow_empty(false)
                .row_height(style.page_row_height)
                .max_height(list_rect.height())
                .show(&mut list_ui, |at, key| {
                    let row = ListRow::new(key.clone(), page_list[at].1.as_str()).dimmed(page_list[at].2);
                    if page_list[at].2 { row.description(texts.empty_page.as_str()) } else { row }
                });
            for event in listed.events {
                if let ListEvent::Selection(change) = event
                    && let Some(row) = change.row
                {
                    state.selected = row;
                }
            }
        }

        // The page.
        let mut rows_out = RowsOut::default();
        if let Some(page) = &shown_page {
            let reset = state.shown.as_deref() != Some(page.id.as_str());
            let mut page_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt("page")
                    .max_rect(page_rect)
                    .layout(direction::column(ui))
                    .accessibility_role(Role::Group)
                    .accessibility_label(page.title.as_str()),
            );
            page_ui.set_clip_rect(page_rect.intersect(ui.clip_rect()));
            let searching = !query.trim().is_empty();
            page_ui.add(Text::new(page.title.as_str()).role(TypeRole::Title));
            page_ui.add_space(gap);
            let mut area = ScrollArea::vertical().id_salt("page-scroll").auto_shrink([false, false]);
            if reset {
                area = area.vertical_scroll_offset(0.0);
            }
            let reset_clicked = area.show(&mut page_ui, |ui| show_page(ui, host, page, &texts, &numbers, searching, &mut rows_out)).inner;
            if reset_clicked {
                host.reset_page(&page.id);
                rows_out.edited = true;
            }
        } else {
            let mut page_ui = ui.new_child(UiBuilder::new().id_salt("page").max_rect(page_rect).layout(direction::column(ui)));
            EmptyState::new(texts.no_matches.as_str()).status().show(&mut page_ui);
        }
        state.shown = shown_page.as_ref().map(|p| p.id.clone());
        out.page = state.shown.clone();
        out.pick_color = rows_out.pick_color.take();
        if rows_out.edited {
            state.error = None;
        }

        // The error, then the buttons.
        let footer = Rect::from_min_max(pos2(rect.left(), content.bottom() + gap), rect.max);
        let mut footer_ui = ui.new_child(UiBuilder::new().id_salt("footer").max_rect(footer).layout(direction::column(ui)));
        if let Some(error) = &state.error {
            let shown = Notice::new(error.as_str()).tone(Tone::Bad).id_salt((id, "error")).show(&mut footer_ui);
            let measured = shown.rect.height();
            if (measured - state.notice_height).abs() > 0.5 {
                state.notice_height = measured;
                ctx.request_discard("preferences error notice height");
            }
            footer_ui.add_space(gap);
        }
        let blocked = host.nested_open();
        let buttons = vec![
            FooterButton::new(ButtonRole::Default, texts.ok.as_str()).enabled(host.can_ok() && !blocked),
            FooterButton::new(ButtonRole::Cancel, texts.cancel.as_str()),
            FooterButton::new(ButtonRole::Apply, texts.apply.as_str()).enabled(host.differs() && !blocked),
        ];
        let mut row = FooterButtons::new((id, "buttons"), buttons).default_hint(texts.default_hint.as_str());
        if let Some(platform) = self.platform {
            row = row.platform(platform);
        }
        let pressed = row.show(&mut footer_ui);
        ui.allocate_rect(rect, Sense::hover());

        // What answered the window, in the order the dialog rules give.
        let mut answer = self.cancel.then_some(ButtonRole::Cancel);
        if answer.is_none() && !blocked {
            answer = pressed.clicked.map(|c| c.role);
        }
        if answer.is_none() && keys {
            // Read after the rows: a field's gesture takes the Esc that cancels it.
            let escape = ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none());
            // Esc and Enter in the search field are the field's: it clears, then leaves.
            let in_search = search_focused;
            let enter = ctx.input(|i| i.events.iter().any(is_enter_press)) && !in_search;
            if escape && in_search {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                if search_before.is_empty() {
                    search.response.surrender_focus();
                }
            } else {
                let resolved = resolve_keys(KeyInput {
                    enter,
                    escape,
                    modal: true,
                    active: true,
                    footer: true,
                    popup_open: egui::Popup::is_any_open(&ctx) || crate::popup_list::any_open(&ctx),
                    button_focused: pressed.focused,
                    default_enabled: pressed.default.is_some(),
                    enter_rule: EnterRule::Default,
                });
                answer = match resolved {
                    Some(crate::dialog::Outcome::Ok) => Some(ButtonRole::Default),
                    Some(crate::dialog::Outcome::Cancel) => Some(ButtonRole::Cancel),
                    _ => None,
                };
                if answer.is_some() {
                    ctx.input_mut(|i| {
                        i.consume_key(Modifiers::NONE, Key::Escape);
                        i.events.retain(|e| !is_enter_press(e));
                    });
                }
            }
        }
        match answer {
            Some(ButtonRole::Default) => match host.apply() {
                Ok(()) => {
                    state.error = None;
                    out.applied = true;
                    out.closed = Some(Closed::Ok);
                }
                Err(message) => state.error = Some(message),
            },
            Some(ButtonRole::Apply) => match host.apply() {
                Ok(()) => {
                    state.error = None;
                    out.applied = true;
                }
                Err(message) => state.error = Some(message),
            },
            Some(ButtonRole::Cancel) => {
                host.cancel();
                out.closed = Some(Closed::Cancel);
            }
            _ => {}
        }

        host.nested(&ctx);
        state.store(&ctx, id);
        out
    }
}

/// A plain press of Enter that is not a key repeat.
fn is_enter_press(event: &Event) -> bool {
    matches!(event, Event::Key { key: Key::Enter, pressed: true, repeat: false, modifiers, .. } if Modifiers { shift: false, ..*modifiers }.is_none())
}

/// The part of `page` a search shows.
fn shown_page(query: &str, listed: &Listed, page: &super::PageSpec<'_>) -> ShownPage {
    let groups = page
        .groups
        .iter()
        .filter_map(|g| {
            let rows: Vec<ShownRow> = g.rows.iter().filter(|r| search::row_shown(query, listed, r)).map(ShownRow::of).collect();
            (!rows.is_empty()).then(|| ShownGroup { title: g.title.map(str::to_owned), columns: g.columns, rows })
        })
        .collect();
    ShownPage { id: page.id.to_owned(), title: page.title.to_owned(), custom: page.custom, footnote: page.footnote.map(str::to_owned), groups }
}

/// The page's groups, footnote and Reset Section. Returns whether Reset Section was pressed.
fn show_page<H: PreferencesModel + ?Sized>(
    ui: &mut Ui,
    host: &mut H,
    page: &ShownPage,
    texts: &PreferencesTexts,
    numbers: &LocaleNumbers,
    searching: bool,
    out: &mut RowsOut,
) -> bool {
    let tokens = Tokens::of(ui);
    let gap = tokens.space.gap.stack;
    ui.spacing_mut().item_spacing.y = gap;
    if page.custom {
        ui.scope(|ui| host.custom_page(ui, &page.id));
    } else if page.groups.is_empty() && !searching {
        EmptyState::new(texts.empty_page.as_str()).show(ui);
    }
    for (at, group) in page.groups.iter().enumerate() {
        let mut rows = |ui: &mut Ui| {
            crate::composites::FieldGroup::new((&page.id, at)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = gap;
                for row in &group.rows {
                    show_row(ui, host, row, numbers, &texts.hex_label, out, group.columns == Columns::Checks);
                }
            });
        };
        match &group.title {
            Some(title) => {
                GroupBox::new(title.as_str()).id_salt(at).show(ui, rows);
                ui.add_space(GroupBox::gap(ui) - gap);
            }
            None => rows(ui),
        }
    }
    if let Some(footnote) = &page.footnote {
        ui.add(Text::new(footnote.as_str()).role(craft_theme::TypeRole::Caption).content(Content::Muted));
    }
    let mut reset = false;
    if !searching && (page.custom || !page.groups.is_empty()) {
        reset = ui.add(Button::new(texts.reset_section.as_str()).variant(craft_theme::ButtonVariant::Secondary).size(ControlSize::Md)).clicked();
    }
    reset
}
