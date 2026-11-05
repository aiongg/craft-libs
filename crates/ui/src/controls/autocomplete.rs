//! [`Autocomplete`]: a suggestion list under a text editor the host owns, with keyboard selection,
//! a description of the active suggestion, and an inline completion.

use std::hash::{Hash, Hasher};

use egui::accesskit::Role;
use egui::{
    Align2, Area, AsIdSalt, EventFilter, Frame, Galley, Id, IdSalt, Key, Label, Margin, Modifiers, Order, Pos2, Rect, Response, RichText, Shape, Ui,
    pos2, vec2,
};

use super::option_list::{Hover, OptionRow, OptionRows, option_id, text_galley};
use craft_theme::{ControlSize, TypeRole};
use egui::Color32;

use super::{ListboxStyle, Select};
use crate::behavior::Listbox;
use crate::direction;
use crate::localized::Labels as _;
use crate::popup_list::edge_width;
use crate::primitives::SurfaceStyle;
use crate::theme::UiTokens;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, apply_popup_style};

crate::words! {
    /// The list's words, English by default.
    #[non_exhaustive]
    pub struct AutocompleteLabels {
        /// The popup's accessible name (a `ListBox`).
        suggestions: "autocomplete.suggestions" = "Suggestions";
    }
}

/// One suggestion: what it inserts, an optional mark before it (GridCraft's "ƒ" for a function)
/// and an optional description shown while it is active.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    /// The text the suggestion inserts.
    pub text: String,
    /// A short mark drawn before the text.
    pub mark: Option<String>,
    /// A description shown while the suggestion is active.
    pub description: Option<String>,
}

impl Suggestion {
    /// A suggestion that inserts `text`, with no mark or description.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), mark: None, description: None }
    }

    /// Sets the mark drawn before the text.
    pub fn mark(mut self, mark: impl Into<String>) -> Self {
        self.mark = Some(mark.into());
        self
    }

    /// Sets the description shown while the suggestion is active.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// The keys the list took this pass, from [`Autocomplete::take_keys`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AutocompleteKeys {
    /// Tab (or Enter, with [`Autocomplete::enter_accepts`]) accepted the active suggestion.
    pub accepted: bool,
    /// Esc closed the list; the editor keeps its text and the Esc goes no further.
    pub dismissed: bool,
}

/// What the list reported this pass.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AutocompleteResponse {
    /// The popup's response while it shows.
    pub popup: Option<Response>,
    /// The suggestion accepted this pass (a key, or a click on a row), by index: the host
    /// replaces the word being typed with its text and keeps the focus in its editor.
    pub accepted: Option<usize>,
    /// The list was dismissed this pass (Esc, or the host's text no longer asks for one).
    pub dismissed: bool,
}

/// A suggestion list for a text editor, after GridCraft's formula AutoComplete.
///
/// The list is two calls around the host's editor, because its keys must be taken before the
/// editor sees them:
///
/// ```ignore
/// let ac = Autocomplete::new("formula", editor_id, &suggestions);
/// let keys = ac.take_keys(ui);                       // ↑ ↓ Tab Esc, while the list shows
/// let field = TextEdit::singleline(&mut text).show(ui);
/// let out = ac.show(ui, field.response.rect, keys);  // the list under the editor
/// if let Some(i) = out.accepted { complete_word(&mut text, &suggestions[i].text) }
/// ```
///
/// - **When.** The list shows, and takes its keys, only while its editor (the id passed to
///   [`new`](Self::new)) has the keyboard focus, `suggestions` is not empty and the list has not
///   been dismissed; the host decides what to suggest (it recomputes them as the user types). A
///   dismissed list shows again when the suggestions change.
/// - **Look.** A popup surface 4 points under `anchor`, kept inside the window (or the host's
///   bounds, [`bounds`](Self::bounds)), on the tooltip layer so it is above the editor's panel.
///   Rows are list box rows (`listbox.*`) at least 220 points wide, the mark then the text in
///   the `Mono` role; the active row is `listbox.active.*`. Under them, past a separator, the
///   active suggestion's description in the `Caption` role, wrapping to the list's width.
/// - **Keys** (in [`take_keys`](Self::take_keys), while the list shows): ↑ and ↓ move the active
///   row and wrap; Tab accepts it; Enter accepts it only with
///   [`enter_accepts`](Self::enter_accepts) (GridCraft: Enter commits the cell); Esc closes the
///   list. The keys are consumed in `take_keys`, before the editor and anything drawn after it
///   read the pass's input, so the editor never sees them and a dialog's Esc doesn't cancel
///   (the field reports no [`escaped`](crate::edit::EditResponse::escaped)). Other keys stay
///   the editor's, in the same pass. With the list closed nothing is taken. The list is not a
///   keyboard-owning popup: [`focus::key_owner`](crate::focus::key_owner) keeps reporting the
///   focused text field.
/// - **Pointer.** A click on a row accepts it.
/// - **AccessKit.** The popup is a `ListBox` named [`AutocompleteLabels::suggestions`], with the
///   active row as the editor's `active_descendant` while it shows.
pub struct Autocomplete<'a> {
    id_salt: IdSalt,
    editor: Id,
    suggestions: &'a [Suggestion],
    enter_accepts: bool,
    bounds: Option<Rect>,
    labels: Option<AutocompleteLabels>,
}

impl<'a> Autocomplete<'a> {
    /// A list of `suggestions` for the text editor whose widget id is `editor`.
    pub fn new(id_salt: impl AsIdSalt, editor: Id, suggestions: &'a [Suggestion]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), editor, suggestions, enter_accepts: false, bounds: None, labels: None }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: AutocompleteLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Enter accepts the active suggestion too (a code editor's); off by default.
    pub fn enter_accepts(mut self, on: bool) -> Self {
        self.enter_accepts = on;
        self
    }

    /// The rect the popup stays inside (GridCraft keeps a cell's list inside the sheet); the
    /// window by default.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Takes the list's keys from this pass's input; call before the editor is drawn.
    // gridcraft: crates/ui-egui/src/formula_bar.rs:538 (↑ ↓ over the list), :485 (Tab), :501 (Esc)
    pub fn take_keys(&self, ui: &mut Ui) -> AutocompleteKeys {
        let id = ui.make_persistent_id(self.id_salt);
        let mut pass = self.load(ui, id);
        let mut keys = AutocompleteKeys::default();
        let count = self.suggestions.len();
        if count > 0 && !pass.memory.dismissed && self.has_focus(ui.ctx(), id) {
            let mut active = pass.listbox.active().filter(|&a| a < count).unwrap_or(0);
            let mut moved = false;
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    active = (active + 1) % count;
                    moved = true;
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    active = (active + count - 1) % count;
                    moved = true;
                }
                keys.accepted = i.consume_key(Modifiers::NONE, Key::Tab)
                    || i.consume_key(Modifiers::SHIFT, Key::Tab)
                    || (self.enter_accepts && i.consume_key(Modifiers::NONE, Key::Enter));
                keys.dismissed = i.consume_key(Modifiers::NONE, Key::Escape);
            });
            if moved {
                pass.listbox.reveal(Some(active), true);
            }
            pass.memory.dismissed |= keys.dismissed;
        }
        pass.store(ui, id);
        keys
    }

    /// Draws the list under `anchor` (the editor's rect) and reports what was accepted.
    // gridcraft: crates/ui-egui/src/formula_bar.rs:858 show_editor_suggestions
    pub fn show(self, ui: &mut Ui, anchor: Rect, keys: AutocompleteKeys) -> AutocompleteResponse {
        let id = ui.make_persistent_id(self.id_salt);
        let ctx = ui.ctx().clone();
        let mut pass = self.load(ui, id);
        let count = self.suggestions.len();
        let mut out = AutocompleteResponse { popup: None, accepted: None, dismissed: keys.dismissed };
        let showing = count > 0 && !pass.memory.dismissed && self.has_focus(&ctx, id);
        out.dismissed |= pass.memory.shown && count == 0;
        pass.memory.shown = showing;
        if !showing {
            pass.store(ui, id);
            return out;
        }

        // egui moves or drops the focus on Tab, Esc and ↑ ↓ at the start of the next pass, before
        // `take_keys` runs; the filter keeps the focus on the editor while the list shows.
        ctx.memory_mut(|m| {
            if m.has_focus(self.editor) {
                m.set_focus_lock_filter(self.editor, EventFilter { tab: true, escape: true, horizontal_arrows: true, vertical_arrows: true });
            }
        });
        let labels = self.labels.unwrap_or_else(|| AutocompleteLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = Autocomplete::style(&tokens);
        let rows_style = style.list;
        let texts: Vec<String> =
            self.suggestions.iter().map(|s| s.mark.as_ref().map_or_else(|| s.text.clone(), |mark| format!("{mark}  {}", s.text))).collect();
        let widest =
            texts.iter().map(|t| text_galley(ui, rows_style.label, t, egui::Color32::PLACEHOLDER, f32::INFINITY).size().x).fold(0.0, f32::max);
        let width = style.min_width.max(2.0 * rows_style.pad_x + rows_style.icon + rows_style.gap + widest);
        let active = pass.listbox.active().filter(|&a| a < count).unwrap_or(0);
        if pass.listbox.active() != Some(active) {
            pass.listbox.reveal(Some(active), false);
        }
        let bounds = self.bounds.unwrap_or_else(|| ctx.content_rect());
        let rtl = direction::is_rtl(ui);
        let (origin, pivot) = if rtl {
            (pos2(anchor.right(), anchor.bottom() + style.gap), Align2::RIGHT_TOP)
        } else {
            (pos2(anchor.left(), anchor.bottom() + style.gap), Align2::LEFT_TOP)
        };
        let mut clicked = None;
        let shown = Area::new(id.with("popup"))
            .order(Order::Tooltip)
            .fixed_pos(origin)
            .pivot(pivot)
            .constrain_to(bounds)
            .role(Role::ListBox)
            .accessible_name(labels.suggestions.as_str())
            .show(&ctx, |ui| {
                apply_popup_style(ui);
                let slot = ui.painter().add(Shape::Noop);
                let margin = ui.spacing().menu_margin + Margin::same(edge_width(&style.popup).ceil() as i8);
                let framed = Frame::NONE.inner_margin(margin).show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let rows = OptionRows::new(ui, &rows_style, ControlSize::Md, id, false, true, false).hover(Hover::Highlights);
                    let reveal = if ui.is_sizing_pass() { None } else { pass.listbox.take_reveal() };
                    egui::ScrollArea::vertical().max_height(rows_style.max_height).auto_shrink([true, true]).show(ui, |ui| {
                        for (row, text) in texts.iter().enumerate() {
                            let outcome = rows.show(
                                ui,
                                &mut pass.listbox,
                                &OptionRow {
                                    option: row,
                                    text,
                                    icon: None,
                                    leading: None,
                                    chosen: false,
                                    enabled: true,
                                    tooltip: None,
                                    shortcut: None,
                                },
                            );
                            if outcome.picked {
                                clicked = Some(row);
                            }
                            if reveal == Some(row) {
                                ui.scroll_to_rect(outcome.rect, None);
                            }
                        }
                    });
                    if let Some(description) = self.suggestions[active].description.as_deref() {
                        ui.add_space(rows_style.separator_pad);
                        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), egui::Sense::hover());
                        ui.painter().rect_filled(line, 0.0, rows_style.separator);
                        ui.add_space(rows_style.separator_pad);
                        let text = RichText::new(description).font(tokens.font(style.description)).color(rows_style.text);
                        ui.add(Label::new(text).wrap());
                    }
                });
                ui.painter().set(slot, Shape::Vec(style.popup.shapes(framed.response.rect, ui.pixels_per_point())));
            });
        out.popup = Some(shown.response);
        if let Some(row) = clicked {
            pass.listbox.reveal(Some(row), false);
            out.accepted = Some(row);
        } else if keys.accepted {
            out.accepted = Some(active);
        }
        if clicked.is_some() {
            ctx.memory_mut(|m| m.request_focus(self.editor));
        }
        ctx.accesskit_node_builder(self.editor, |node| node.set_active_descendant(option_id(id, active).accesskit_id()));
        pass.store(ui, id);
        out
    }
}

/// The suggestion list's sizes and colours, from [`Autocomplete::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AutocompleteStyle {
    /// The list's narrowest width: `autocomplete.min_width`.
    pub min_width: f32,
    /// Between the editor and the list: `style.gap`.
    pub gap: f32,
    /// The rows: a select list's, in the monospace role to match the editor.
    pub list: ListboxStyle,
    /// The list's surface: [`SurfaceKind::Popup`](crate::primitives::SurfaceKind::Popup).
    pub popup: SurfaceStyle,
    /// The active suggestion's description: its text role.
    pub description: TypeRole,
    /// The inline completion's box: `field.selection`.
    pub completion_fill: Color32,
    /// The inline completion's text: `field.selected_text`.
    pub completion_text: Color32,
}

impl Autocomplete<'static> {
    /// The list's sizes and colours.
    pub fn style(tokens: &UiTokens) -> AutocompleteStyle {
        let select = Select::style(tokens, ControlSize::Md);
        let (autocomplete, field) = (&tokens.component.autocomplete, &tokens.component.field);
        let mut list = select.list;
        list.label = TypeRole::Mono;
        AutocompleteStyle {
            min_width: autocomplete.min_width,
            gap: autocomplete.gap,
            list,
            popup: select.popup,
            description: TypeRole::Caption,
            completion_fill: field.selection.to_egui(),
            completion_text: field.selected_text.to_egui(),
        }
    }
}

/// The rest of `full` after what the user typed, when `full` starts with `typed` ignoring case
/// and is longer: GridCraft's column AutoComplete, which the editor shows selected after the
/// caret ([`paint_inline_completion`]). `None` otherwise. The rest starts after as many
/// characters of `full` as `typed` has (GridCraft's; its caller had checked the prefix, which
/// this function checks itself).
// gridcraft: crates/ui-egui/src/editor.rs:388 completion_rest
pub fn completion_rest<'s>(full: &'s str, typed: &str) -> Option<&'s str> {
    let mut full_chars = full.char_indices();
    for t in typed.chars() {
        let (_, f) = full_chars.next()?;
        if !f.to_lowercase().eq(t.to_lowercase()) {
            return None;
        }
    }
    let start = full_chars.next()?.0;
    full.get(start..)
}

/// Paints `rest` as selected text right after `galley`, the editor's laid-out text drawn at
/// `text_pos`: the `field.selection.fill` box and `field.selection.text`, in the galley's font
/// (GridCraft's column AutoComplete: accent fill, on-accent text).
// gridcraft: crates/ui-egui/src/formula_bar.rs:861 (inline completion)
pub fn paint_inline_completion(ui: &Ui, text_pos: Pos2, galley: &Galley, rest: &str) {
    let style = Autocomplete::style(&Tokens::of(ui));
    let (fill, ink) = (style.completion_fill, style.completion_text);
    let font = galley.job.sections.first().map(|s| s.format.font_id.clone()).unwrap_or_default();
    let pos = text_pos + vec2(galley.rect.right(), 0.0);
    let painted = ui.painter().layout_no_wrap(rest.to_owned(), font, ink);
    ui.painter().rect_filled(Rect::from_min_size(pos, painted.size()), 0.0, fill);
    ui.painter().galley(pos, painted, ink);
}

/// What the list keeps between passes under its id.
#[derive(Clone, Default)]
struct Memory {
    /// The suggestions the state below was made for; other suggestions start it again.
    signature: u64,
    dismissed: bool,
    /// The list was showing in the last pass of `show`.
    shown: bool,
}

struct Pass {
    memory: Memory,
    listbox: Listbox,
}

impl Pass {
    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|d| d.insert_temp(id.with("memory"), self.memory));
        self.listbox.store(ui);
    }
}

impl Autocomplete<'_> {
    /// The editor or one of the rows (just pressed) has the keyboard focus.
    fn has_focus(&self, ctx: &egui::Context, id: Id) -> bool {
        ctx.memory(|m| m.has_focus(self.editor) || (0..self.suggestions.len()).any(|row| m.has_focus(option_id(id, row))))
    }

    /// The state as the last pass left it, started again when the suggestions changed.
    fn load(&self, ui: &Ui, id: Id) -> Pass {
        let mut memory = ui.data(|d| d.get_temp::<Memory>(id.with("memory"))).unwrap_or_default();
        let mut listbox = Listbox::load(ui, id.with("list"));
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for s in self.suggestions {
            (&s.text, &s.mark, &s.description).hash(&mut hasher);
        }
        let signature = hasher.finish();
        if memory.signature != signature {
            memory.signature = signature;
            memory.dismissed = false;
            listbox.reveal(Some(0), false);
        }
        Pass { memory, listbox }
    }
}
