//! [`FieldTab`]: a dialog's focus scope, in which Tab and Shift+Tab visit only the text and number
//! fields, as in PhotoCraft's dialogs.

use egui::{AsIdSalt, Context, Event, FocusDirection, Id, IdSalt, InnerResponse, Key, LayerId, Modifiers, Popup, Rect, Ui, UiBuilder};

/// A dialog's focus scope: wrap a dialog's contents in it, and Tab and Shift+Tab visit only its
/// text and number fields, as in PhotoCraft (Name, Width, Height, Resolution in New Document),
/// never its sliders, lists, check boxes or buttons:
///
/// ```
/// # use craft_ui::behavior::FieldTab;
/// # use craft_ui::controls::{Button, TextField};
/// # egui::__run_test_ui(|ui| { let name = String::from("Untitled");
/// FieldTab::new("new-document").show(ui, |ui| {
///     let _ = TextField::new("name", "Name", &name).show(ui);
///     ui.add(Button::new("Create"));
/// });
/// # });
/// ```
///
/// - **Tab, Shift+Tab** move the focus to the next or previous field in layout order, wrapping
///   around; from no field (nothing focused, or another control), Tab goes to the first field
///   and Shift+Tab to the last. The field reached has its text selected, so typing replaces it. In
///   a scope without fields Tab does nothing.
/// - **Ctrl+Tab, Ctrl+Shift+Tab** (Control on macOS too) move the focus as Tab does outside a
///   scope: through every control, in egui's order. With them every control stays reachable from
///   a field, whose arrow keys are its own. It is the key GTK and macOS give for leaving a control
///   that keeps Tab; in PhotoCraft it switches documents, which no dialog lets through.
/// - **Arrow keys** on a control other than a field move the focus by where the controls sit
///   (egui's focus movement), as anywhere else.
///
/// The fields are the controls built on [`Typing`](super::Typing):
/// [`TextField`](crate::controls::TextField), [`SearchField`](crate::controls::SearchField),
/// [`NumberField`](crate::controls::NumberField) and [`ComboBox`](crate::controls::ComboBox),
/// while enabled. A host's own field joins with [`register`](FieldTab::register).
///
/// The scope takes these keys while it holds the focus: the focused control is one of its fields
/// or lies inside the scope on its layer, or nothing is focused and the scope's layer is the
/// topmost [`Modal`](egui::Modal). It leaves them alone while any popup is open (an open list
/// keeps its own Tab) and under a modal layer above it. Outside a scope Tab keeps egui's order
/// over every control. Scopes don't nest: one per dialog.
#[derive(Clone, Copy, Debug)]
pub struct FieldTab {
    id_salt: IdSalt,
}

/// What a scope saw on its last pass: where it was and its fields in layout order.
#[derive(Clone, Debug)]
struct Record {
    rect: Rect,
    fields: Vec<Id>,
}

/// A scope whose contents are being laid out, collecting its fields.
#[derive(Clone, Debug)]
struct Collecting {
    layer: LayerId,
    fields: Vec<Id>,
}

/// A press of Tab this pass, as the scope reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    /// Tab (+1) or Shift+Tab (−1): the next or previous field.
    Field(i32),
    /// Ctrl+Tab or Ctrl+Shift+Tab: egui's order over every control.
    Any(FocusDirection),
}

impl FieldTab {
    /// A scope; `id_salt` keeps what it saw on the last pass when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt) }
    }

    /// Shows `add_contents` in the scope. Begin the scope before any control of the dialog, so
    /// that it takes Tab before a control on the way acts on it.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let id = ui.make_persistent_id(self.id_salt).with("field-tab-record");
        let ctx = ui.ctx().clone();
        let layer = ui.layer_id();
        let record = ctx.data(|d| d.get_temp::<Record>(id));
        let press = if holds_focus(&ctx, layer, record.as_ref()) { take_press(&ctx) } else { None };

        let stack = collecting_id(&ctx);
        ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Collecting>>(stack).push(Collecting { layer, fields: Vec::new() }));
        let inner = ui.scope_builder(UiBuilder::new().id_salt(self.id_salt), add_contents);
        let fields = ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Collecting>>(stack).pop()).map(|c| c.fields).unwrap_or_default();

        if let Some(Press::Field(step)) = press
            && let Some(next) = step_through(&fields, ctx.memory(|m| m.focused()), step)
        {
            // The field sees the focus arrive on the next pass and selects its text.
            ctx.memory_mut(|m| m.request_focus(next));
            ctx.request_repaint();
        }
        ctx.data_mut(|d| d.insert_temp(id, Record { rect: inner.response.rect, fields }));
        inner
    }

    /// Adds the field `id` to the Tab order of the scope `ui` is in, in layout order; outside a
    /// scope, or on another layer than the scope's (a list's popup), it does nothing. The
    /// [`Typing`](super::Typing) controls call it; a host calls it for a field of its own, each
    /// pass, while the field is enabled.
    pub fn register(ui: &Ui, id: Id) {
        let layer = ui.layer_id();
        let stack = collecting_id(ui.ctx());
        ui.data_mut(|d| {
            if let Some(scope) = d.get_temp_mut_or_default::<Vec<Collecting>>(stack).last_mut().filter(|c| c.layer == layer) {
                scope.fields.push(id);
            }
        });
    }
}

fn collecting_id(ctx: &Context) -> Id {
    Id::unique("craft-ui-field-tab-collecting").with(ctx.viewport_id())
}

/// Whether the scope on `layer` that saw `record` on its last pass takes this pass's Tab.
fn holds_focus(ctx: &Context, layer: LayerId, record: Option<&Record>) -> bool {
    if Popup::is_any_open(ctx) || !ctx.memory(|m| m.allows_interaction(layer)) {
        return false;
    }
    match ctx.memory(|m| m.focused()) {
        Some(focused) => record.is_some_and(|record| {
            record.fields.contains(&focused)
                || ctx.read_response(focused).is_some_and(|r| r.layer_id == layer && record.rect.contains(r.rect.center()))
        }),
        None => ctx.memory(|m| m.top_modal_layer()) == Some(layer),
    }
}

/// Takes this pass's Tab presses from the input, so nothing else acts on them, and stops egui's
/// own focus move for a plain Tab (egui read the key before the pass began). A Ctrl+Tab becomes
/// egui's move.
fn take_press(ctx: &Context) -> Option<Press> {
    let mut press = None;
    ctx.input_mut(|i| {
        i.events.retain(|event| {
            let Event::Key { key: Key::Tab, pressed, modifiers, .. } = event else { return true };
            let Some(this) = read_tab(*modifiers) else { return true };
            if *pressed {
                press = Some(match (press, this) {
                    (Some(Press::Field(sum)), Press::Field(step)) => Press::Field(sum + step),
                    (_, this) => this,
                });
            }
            false
        });
    });
    match press {
        Some(Press::Field(_)) => ctx.memory_mut(|m| m.move_focus(FocusDirection::None)),
        Some(Press::Any(direction)) => ctx.memory_mut(|m| m.move_focus(direction)),
        None => {}
    }
    press
}

/// What Tab with `modifiers` does in a scope; `None` for a chord the scope leaves alone (Alt+Tab,
/// Cmd+Tab).
fn read_tab(m: Modifiers) -> Option<Press> {
    if m.alt || m.mac_cmd {
        return None;
    }
    let back = m.shift;
    Some(match (m.ctrl, back) {
        (false, false) => Press::Field(1),
        (false, true) => Press::Field(-1),
        (true, false) => Press::Any(FocusDirection::Next),
        (true, true) => Press::Any(FocusDirection::Previous),
    })
}

/// The field `step` fields on from `focused` in `fields`, wrapping around; from a focus that is
/// none of them, the first for a step forward and the last for one back.
fn step_through(fields: &[Id], focused: Option<Id>, step: i32) -> Option<Id> {
    let count = i32::try_from(fields.len()).ok().filter(|&n| n > 0 && step != 0)?;
    let at = focused.and_then(|f| fields.iter().position(|&id| id == f)).and_then(|i| i32::try_from(i).ok());
    let next = match at {
        Some(i) => (i + step).rem_euclid(count),
        None if step > 0 => (step - 1).rem_euclid(count),
        None => (count + step).rem_euclid(count),
    };
    usize::try_from(next).ok().and_then(|i| fields.get(i)).copied()
}
