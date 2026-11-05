//! Cursor chat: your bubble at the pointer, and everyone's bubbles (N5.09).

use craft_ui::controls::TextField;
use craft_ui::edit::EditEvent;
use craft_ui::focus::key_owner::{KeyOwner, OwnerGate, owner};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _, UiTokens, role_format};
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, CornerRadius, Event, Frame, IdSalt, Key, Margin, Modifiers, Order, Painter, Pos2, Rect, Ui, vec2};

use super::presence::{ink_on, tag_offset};
use super::style::ReviewStyle;
use crate::collab::{CHAT_FADE_S, CHAT_HOLD_S, CollabRequest, chat_alpha};

/// How much of the ink the name over a bubble's text keeps (FigCraft `live.rs:236`).
const NAME_INK: f32 = 0.8;

craft_ui::words! {
    /// The chat box's words, English by default (FigCraft's).
    #[non_exhaustive]
    pub struct ChatLabels {
        name: "chat.name" = "Cursor chat";
        hint: "chat.hint" = "Say something";
    }
}

/// A chat bubble with its corner at `tip` (beside the pointer): the name over the text when
/// given, in `fill` (the person's colour) at `alpha`. It is as wide as its text, which wraps at
/// `review.chat_width`; rounded `review.chat_radius` except the corner at the pointer, which is
/// square (FigCraft's bubble).
// figcraft: crates/ui-egui/src/live.rs:232 `chat_bubble`.
pub fn paint_chat_bubble(painter: &Painter, tokens: &UiTokens, tip: Pos2, name: Option<&str>, text: &str, fill: Color32, alpha: f32) {
    let style = ReviewStyle::of(tokens);
    let on = ink_on(&style, fill).gamma_multiply(alpha);
    let mut body = LayoutJob::single_section(text.to_owned(), role_format(tokens, style.body_text, on));
    body.wrap.max_width = style.chat_width;
    let body = painter.layout_job(body);
    let head = name.map(|name| {
        painter.layout_job(LayoutJob::single_section(name.to_owned(), role_format(tokens, style.caption_text, on.gamma_multiply(NAME_INK))))
    });
    let pad = vec2(tokens.space.pad.control, tokens.space.gap.inline);
    let gap = tokens.space.gap.inline / 2.0;
    let head_height = head.as_ref().map_or(0.0, |galley| galley.size().y + gap);
    let width = head.as_ref().map_or(0.0, |galley| galley.size().x).max(body.size().x);
    let rect = Rect::from_min_size(tip + tag_offset(tokens), vec2(width, head_height + body.size().y) + 2.0 * pad);
    let radius = bubble_radius(&style);
    painter.rect_filled(rect, radius, fill.gamma_multiply(alpha));
    let mut at = rect.min + pad;
    if let Some(head) = head {
        let height = head.size().y;
        painter.galley(at, head, on);
        at.y += height + gap;
    }
    painter.galley(at, body, on);
}

/// The bubble's corners: square at the pointer (top left).
fn bubble_radius(style: &ReviewStyle) -> CornerRadius {
    let round = style.chat_radius.round().clamp(0.0, 255.0) as u8;
    CornerRadius { nw: 0, ne: round, sw: round, se: round }
}

/// Your cursor chat: `/` over the canvas opens a bubble at the pointer with a text field (the
/// focus moves into it); each change comes back as a [`CollabRequest::Chat`] for the app to
/// send; Enter sends and keeps the bubble, Esc or an empty Enter closes it (an empty `Chat`).
/// The bubble follows the pointer. The text field is the kernel's
/// [`TextField`](craft_ui::controls::TextField); `/` is taken only while the canvas holds the
/// keys ([`focus::key_owner`](craft_ui::focus::key_owner)).
pub struct CursorChatBox<'a> {
    id_salt: IdSalt,
    text: &'a mut String,
    open: &'a mut bool,
    color: Color32,
    labels: Option<ChatLabels>,
}

impl<'a> CursorChatBox<'a> {
    /// `color` is your own colour in the session ([`Participant::color`](crate::collab::Participant::color)):
    /// your bubble is drawn in it, as the others see it.
    pub fn new(id_salt: impl AsIdSalt, text: &'a mut String, open: &'a mut bool, color: [u8; 3]) -> Self {
        let [r, g, b] = color;
        Self { id_salt: IdSalt::new(id_salt), text, open, color: Color32::from_rgb(r, g, b), labels: None }
    }

    pub fn labels(mut self, labels: ChatLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/live.rs:353 `open_chat`, `:362` `chat_box`.
    pub fn show(self, ui: &mut Ui, pointer: Option<Pos2>) -> CursorChatBoxOutput {
        let Self { id_salt, text, open, color: fill, labels } = self;
        let labels = labels.unwrap_or_else(|| ChatLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id_salt);
        let field_id = id.with("field");
        let mut requests = Vec::new();
        let now = ctx.input(|input| input.time);
        let opened = ChatState { editing: true, changed: now, focus: true, at: pointer };
        let mut state: ChatState = ctx.data(|data| data.get_temp(id)).unwrap_or_else(|| opened.clone());

        if !*open {
            // `/` opens the box at the pointer; the key's text must not land in the field.
            let slash = pointer.is_some()
                && owner(&ctx, &OwnerGate::default()) == KeyOwner::App
                && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Slash));
            if slash {
                ctx.input_mut(|input| input.events.retain(|event| !matches!(event, Event::Text(typed) if typed == "/")));
                *open = true;
                state = opened;
            } else {
                return CursorChatBoxOutput { requests };
            }
        }
        state.at = pointer.or(state.at);
        let at = state.at.unwrap_or_else(|| ui.max_rect().center());
        let idle = now - state.changed;

        if state.editing && idle >= CHAT_HOLD_S + CHAT_FADE_S {
            close(text, open, &mut requests);
        } else if state.editing {
            ctx.request_repaint_after_secs((CHAT_HOLD_S + CHAT_FADE_S - idle) as f32);
            let on = ink_on(&style, fill);
            let pad = Margin::symmetric(tokens.space.pad.control.round() as i8, tokens.space.gap.inline.round() as i8);
            let mut submitted = false;
            let mut escaped = false;
            let mut left = false;
            egui::Area::new(id.with("area")).order(Order::Foreground).fixed_pos(at + tag_offset(&tokens)).show(&ctx, |ui| {
                Frame::NONE.fill(fill).corner_radius(bubble_radius(&style)).inner_margin(pad).show(ui, |ui| {
                    ui.visuals_mut().override_text_color = Some(on);
                    let field = TextField::new(id.with("text"), &labels.name, text.as_str())
                        .id(field_id)
                        .placeholder(&labels.hint)
                        .width(style.chat_width)
                        .show(ui);
                    if state.focus {
                        ui.memory_mut(|memory| memory.request_focus(field_id));
                        state.focus = false;
                    }
                    if let Some(value) = field.event.as_ref().and_then(EditEvent::value)
                        && value != text.as_str()
                    {
                        value.clone_into(text);
                        state.changed = now;
                        requests.push(CollabRequest::Chat(text.clone()));
                    }
                    submitted = field.submitted;
                    escaped = field.escaped;
                    left = field.response.lost_focus();
                });
            });
            if escaped || ((submitted || left) && text.trim().is_empty()) {
                close(text, open, &mut requests);
            } else if submitted || left {
                state.editing = false;
                state.changed = now;
            }
        } else {
            let age = now - state.changed;
            let alpha = chat_alpha(age, false);
            if alpha <= 0.0 || text.is_empty() {
                close(text, open, &mut requests);
            } else {
                ctx.request_repaint_after_secs((if age < CHAT_HOLD_S { CHAT_HOLD_S - age } else { 0.0 }) as f32);
                let painter = ctx.layer_painter(egui::LayerId::new(Order::Foreground, id.with("bubble")));
                paint_chat_bubble(&painter, &tokens, at, None, text, fill, alpha);
            }
        }
        if *open {
            ctx.data_mut(|data| data.insert_temp(id, state));
        } else {
            ctx.data_mut(|data| data.remove_temp::<ChatState>(id));
        }
        CursorChatBoxOutput { requests }
    }
}

/// Your box's state between passes.
#[derive(Clone, Default)]
struct ChatState {
    /// The field is open (otherwise the sent text fades).
    editing: bool,
    /// When the text last changed (egui time).
    changed: f64,
    /// The field takes the focus on its first pass.
    focus: bool,
    /// The pointer's last position.
    at: Option<Pos2>,
}

/// Closes the box and tells the others (an empty chat).
fn close(text: &mut String, open: &mut bool, requests: &mut Vec<CollabRequest>) {
    text.clear();
    *open = false;
    requests.push(CollabRequest::Chat(String::new()));
}

/// What the chat box did this pass.
#[must_use]
pub struct CursorChatBoxOutput {
    pub requests: Vec<CollabRequest>,
}
