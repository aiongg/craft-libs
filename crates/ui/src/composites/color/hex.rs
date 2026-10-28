//! The colour picker's hex field: the colour as six hex digits, typed as a draft that applies
//! once it is a colour (PhotoCraft's hex field).

use craft_theme::{ControlSize, Rgba, TypeRole};
use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use egui::{Align2, Context, Id, Shape, Ui, vec2};

use crate::behavior::{Codec, NotAValue, Typing, TypingLook};
use crate::controls::text_field::{BoxState, field_box};
use crate::controls::{TextField, TooltipExt as _};
use crate::direction;
use crate::edit::{EditPolicy, EditResponse};
use crate::theme::{Tokens, TokensOf, margin_x};

/// Writes a colour as uppercase `RRGGBB` and reads six digits back, with or without `#`; eight
/// set the alpha too when the picker edits it.
#[derive(Clone, Debug)]
pub(super) struct HexCodec {
    pub alpha: bool,
    /// The invalid field's tooltip for text that can't become a colour, translated.
    pub reason: String,
}

impl Codec for HexCodec {
    type Value = Rgba;

    fn format(&self, value: &Rgba) -> String {
        let [r, g, b, _] = value.0;
        format!("{r:02X}{g:02X}{b:02X}")
    }

    fn parse(&self, text: &str, start: &Rgba) -> Result<Rgba, NotAValue> {
        parse_hex(text, *start, self.alpha).map_err(|partial| NotAValue { reason: self.reason.clone(), partial })
    }

    fn applies_as_typed(&self, _text: &str) -> bool {
        true
    }
}

/// The colour `text` stands for: six hex digits, or eight with `alpha`, after an optional `#`;
/// six keep `start`'s alpha. Otherwise whether the text may still become one as the user types
/// on (fewer digits). Signs, spaces inside and non-ASCII text never do.
pub(super) fn parse_hex(text: &str, start: Rgba, alpha: bool) -> Result<Rgba, bool> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(false);
    }
    let byte = |i: usize| u8::from_str_radix(&digits[2 * i..2 * i + 2], 16).map_err(|_| false);
    match digits.len() {
        6 => Ok(Rgba([byte(0)?, byte(1)?, byte(2)?, start.0[3]])),
        8 if alpha => Ok(Rgba([byte(0)?, byte(1)?, byte(2)?, byte(3)?])),
        n => Err(n < 6 || (alpha && n < 8)),
    }
}

/// The hex field's widget id under the picker `picker`.
pub(super) fn hex_id(picker: Id) -> Id {
    picker.with("craft_ui::color_picker::hex")
}

/// Focuses the hex field under the picker `picker`; it selects its text when it next shows.
pub(super) fn focus(ctx: &Context, picker: Id) {
    let id = hex_id(picker);
    ctx.memory_mut(|m| m.request_focus(id));
    ctx.data_mut(|d| d.insert_temp(select_key(id), true));
    ctx.request_repaint();
}

fn select_key(id: Id) -> Id {
    id.with("craft_ui::color_picker::select_hex")
}

/// Shows the hex field, as wide as the `Ui`'s justified width (a text field's otherwise), for
/// the colour `value`.
pub(super) fn show(ui: &mut Ui, id: Id, label: &str, value: Rgba, codec: HexCodec, policy: EditPolicy, enabled: bool) -> EditResponse<Rgba> {
    let tokens = Tokens::of(ui);
    let style = TextField::style(&tokens, ControlSize::Md);
    let enabled = enabled && ui.is_enabled();
    let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.width };
    let look = TypingLook {
        size: vec2(width, style.height),
        margin: margin_x(style.pad_x, style.pad_x),
        align: direction::align2(ui, Align2::LEFT_CENTER),
        font: tokens.font(TypeRole::Mono),
        text: if enabled { style.text } else { style.text_disabled },
        placeholder: if enabled { style.placeholder } else { style.placeholder_disabled },
        selection: style.selection,
        selected_text: style.selected_text,
        caret: style.caret,
    };
    if ui.data_mut(|d| d.remove_temp::<bool>(select_key(id))).is_some() {
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::select_all_str(&codec.format(&value))));
        state.store(ui.ctx(), id);
    }
    // No character limit: it would cut pasted text (spaces round it, a "#", eight digits with
    // the alpha off) into something else before `parse_hex` sees it.
    let typing = Typing::with_codec(id, label, value, codec).policy(policy).enabled(enabled).keep_rejected(false);
    // The box goes under the text, which `TextEdit` paints as it runs.
    let under = ui.painter().add(Shape::Noop);
    let typed = typing.show(ui, &look);
    let response = typed.response().clone();
    let invalid = typed.rejected().is_some();
    ui.painter().set(under, Shape::Vec(field_box(ui, &style, response.rect, BoxState::of(&response, enabled, invalid))));
    if let Some(reason) = typed.rejected().map(str::to_owned) {
        if response.has_focus() {
            response.show_tooltip_text(reason.as_str());
        }
        let _ = response.clone().tooltip(reason);
    }
    typed.end(ui)
}
