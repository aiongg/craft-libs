//! The hex field, [`HexField`]: the colour as hex digits, typed as a draft that applies once it
//! is a colour (PhotoCraft's hex field). The colour picker's own hex field is this one without
//! the `#`.

use craft_theme::{ControlSize, Rgba, TypeRole};
use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use egui::{Align2, AsIdSalt, Color32, Context, Id, IdSalt, Pos2, Shape, Ui, vec2};

use crate::behavior::{Codec, NotAValue, Typing, TypingLook};
use crate::controls::text_field::{BoxState, field_box};
use crate::controls::{TextField, TooltipExt as _};
use crate::direction;
use crate::edit::{EditPolicy, EditResponse};
use crate::theme::{Tokens, TokensOf, margin_x};

/// Writes a colour as uppercase `RRGGBB` (`RRGGBBAA` with `write_alpha`) and reads six
/// digits back, with or without `#`; eight set the alpha too when the field edits it. With one
/// digit per channel it writes `RGB` (`RGBA`) with each channel on its nearest doubled digit and
/// reads three, or four with alpha, doubling each.
#[derive(Clone, Debug)]
pub(super) struct HexCodec {
    /// Whether eight (four) digits are read as an alpha too.
    pub alpha: bool,
    /// Whether the alpha is written; the picker's field leaves it to the opacity row.
    pub write_alpha: bool,
    /// Digits per channel, 1 or 2.
    pub digits: usize,
    /// The invalid field's tooltip for text that can't become a colour, translated.
    pub reason: String,
}

impl Codec for HexCodec {
    type Value = Rgba;

    fn format(&self, value: &Rgba) -> String {
        let [r, g, b, a] = value.0;
        if self.digits == 1 {
            let nibble = |v: u8| (u16::from(v) + 8) / 17;
            let alpha = if self.write_alpha { format!("{:X}", nibble(a)) } else { String::new() };
            return format!("{:X}{:X}{:X}{alpha}", nibble(r), nibble(g), nibble(b));
        }
        let alpha = if self.write_alpha { format!("{a:02X}") } else { String::new() };
        format!("{r:02X}{g:02X}{b:02X}{alpha}")
    }

    fn parse(&self, text: &str, start: &Rgba) -> Result<Rgba, NotAValue> {
        parse_hex(text, *start, self.alpha, self.digits).map_err(|partial| NotAValue { reason: self.reason.clone(), partial })
    }

    fn applies_as_typed(&self, _text: &str) -> bool {
        true
    }
}

/// The colour `text` stands for: six hex digits, or eight with `alpha`, after an optional `#`;
/// six keep `start`'s alpha. With one `digits` per channel: three, or four with `alpha`, each
/// doubled. Otherwise whether the text may still become one as the user types on (fewer
/// digits). Signs, spaces inside and non-ASCII text never do.
pub(super) fn parse_hex(text: &str, start: Rgba, alpha: bool, digits: usize) -> Result<Rgba, bool> {
    let text = text.trim();
    let text = text.strip_prefix('#').unwrap_or(text);
    if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(false);
    }
    let digits = digits.clamp(1, 2);
    let channel =
        |i: usize| u8::from_str_radix(&text[digits * i..digits * (i + 1)], 16).map(|v| if digits == 1 { v * 17 } else { v }).map_err(|_| false);
    let (plain, with_alpha) = (3 * digits, 4 * digits);
    match text.len() {
        n if n == plain => Ok(Rgba([channel(0)?, channel(1)?, channel(2)?, start.0[3]])),
        n if n == with_alpha && alpha => Ok(Rgba([channel(0)?, channel(1)?, channel(2)?, channel(3)?])),
        n => Err(n < plain || (alpha && n < with_alpha)),
    }
}

pub(super) const NOT_HEX: &str = "Not a hex colour";

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

/// How a hex field behaves besides its colour and codec.
#[derive(Clone, Copy, Debug)]
pub(super) struct Options {
    pub policy: EditPolicy,
    pub enabled: bool,
    /// Whether a fixed `#` stands before the digits.
    pub prefix: bool,
    /// The most characters typed or pasted.
    pub char_limit: usize,
}

impl Options {
    /// The picker's hex field: no `#`, and no limit, which would cut pasted text (spaces round
    /// it, a "#", eight digits with the alpha off) into something else before `parse_hex` sees it.
    pub(super) fn picker(policy: EditPolicy, enabled: bool) -> Options {
        Options { policy, enabled, prefix: false, char_limit: usize::MAX }
    }
}

/// Shows the hex field, as wide as the `Ui`'s justified width (a text field's otherwise), for
/// the colour `value`.
pub(super) fn show(ui: &mut Ui, id: Id, label: &str, value: Rgba, codec: HexCodec, options: Options) -> EditResponse<Rgba> {
    let Options { policy, enabled, prefix, char_limit } = options;
    let tokens = Tokens::of(ui);
    let style = TextField::style(&tokens, ControlSize::Md);
    let enabled = enabled && ui.is_enabled();
    let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.width };
    let font = tokens.font(TypeRole::Mono);
    let text_color = if enabled { style.text } else { style.text_disabled };
    // The `#` sits at the reading start, so the digits start after it.
    let prefix = prefix.then(|| ui.fonts_mut(|f| f.layout_no_wrap("#".to_owned(), font.clone(), text_color)));
    let inset = prefix.as_ref().map_or(0.0, |p| p.size().x);
    let (start, end) = (style.pad_x + inset, style.pad_x);
    let rtl = direction::is_rtl(ui);
    let look = TypingLook {
        size: vec2(width, style.height),
        margin: if rtl { margin_x(end, start) } else { margin_x(start, end) },
        align: direction::align2(ui, Align2::LEFT_CENTER),
        font,
        text: text_color,
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
    let typing = Typing::with_codec(id, label, value, codec).policy(policy).enabled(enabled).keep_rejected(false).char_limit(char_limit);
    // The box goes under the text, which `TextEdit` paints as it runs.
    let under = ui.painter().add(Shape::Noop);
    let typed = typing.show(ui, &look);
    let response = typed.response().clone();
    let invalid = typed.rejected().is_some();
    ui.painter().set(under, Shape::Vec(field_box(ui, &style, response.rect, BoxState::of(&response, enabled, invalid))));
    if let Some(galley) = prefix {
        let rect = response.rect;
        let x = if rtl { rect.right() - style.pad_x - galley.size().x } else { rect.left() + style.pad_x };
        let at = Pos2::new(x, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, Color32::PLACEHOLDER);
    }
    if let Some(reason) = typed.rejected().map(str::to_owned) {
        if response.has_focus() {
            response.show_tooltip(reason.as_str());
        }
        let _ = response.clone().tooltip(reason);
    }
    typed.end(ui)
}

/// A colour as hex digits behind a fixed `#`, in the mono type role: PhotoCraft's hex field of
/// the preferences and the Color panel's readout. The text is a draft that applies as soon as it
/// is a colour ([`EditResponse`] reports each applied value); a draft that can't become one
/// marks the field invalid, with the reason as its tooltip.
///
/// ```
/// use craft_theme::Rgba;
/// use craft_ui::composites::HexField;
/// # egui::__run_test_ui(|ui| { let mut color = Rgba::rgb(0x33, 0x66, 0x99);
/// let edit = HexField::new("fill-hex", "Hex colour", color).digits(1).show(ui);
/// if let Some(c) = edit.event.as_ref().and_then(|e| e.value()) {
///     color = *c;
/// }
/// # });
/// ```
///
/// - Digits: [`digits(2)`](HexField::digits) (the default) takes `RRGGBB`; `digits(1)` takes `RGB`
///   and doubles each digit (a web colour is three digits). A leading `#` in typed or pasted
///   text is dropped. With [`alpha`](HexField::alpha) the field also shows and takes the alpha
///   digits (`RRGGBBAA`, or `RGBA` with one digit); typing six keeps the colour's alpha.
/// - Without an alpha digit pair a typed colour keeps the colour's alpha.
/// - AccessKit: a `TextInput` named `label`; the `#` is decoration. Focus, selection and the
///   edit menu are the text field's.
#[derive(Clone, Debug)]
#[must_use = "show the field with `show`"]
pub struct HexField {
    id_salt: IdSalt,
    label: String,
    value: Rgba,
    digits: usize,
    alpha: bool,
    reason: String,
    policy: EditPolicy,
    enabled: bool,
}

impl HexField {
    /// The words the field shows besides its label, English: the keys of [`reason`](HexField::reason).
    pub const TEXTS: [&'static str; 1] = [NOT_HEX];

    /// A field named `label` (its accessible name, translated by the host: "Hex colour") showing
    /// `value`; `id_salt` keeps its draft and focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: Rgba) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            digits: 2,
            alpha: false,
            reason: NOT_HEX.to_owned(),
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// Digits per channel: 2 (`RRGGBB`, the default) or 1 (`RGB`, each digit doubled on read).
    /// Other counts are the nearest of those.
    pub fn digits(mut self, per_channel: usize) -> Self {
        self.digits = per_channel.clamp(1, 2);
        self
    }

    /// Whether the field also takes the alpha digits. Off by default.
    pub fn alpha(mut self, on: bool) -> Self {
        self.alpha = on;
        self
    }

    /// The tooltip of a draft that can't become a colour, translated by the host; "Not a hex
    /// colour" without it.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    /// How the gesture reports ([`EditPolicy`]); the default reports every change at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the field, as wide as the `Ui`'s justified width (a text field's otherwise).
    pub fn show(self, ui: &mut Ui) -> EditResponse<Rgba> {
        let id = ui.make_persistent_id(self.id_salt);
        // The limit leaves room for a pasted `#`.
        let char_limit = (3 + usize::from(self.alpha)) * self.digits + 1;
        let codec = HexCodec { alpha: self.alpha, write_alpha: self.alpha, digits: self.digits, reason: self.reason };
        show(ui, id, &self.label, self.value, codec, Options { policy: self.policy, enabled: self.enabled, prefix: true, char_limit })
    }
}
