//! Cursor chat: your box at the pointer, and everyone's bubbles.

use craft_ui::contract::{Bind, NoValue, Outcome, Pattern, Props, Remember as _, Show as _, ShowContainer as _};
use craft_ui::fonts::FontRef;
use craft_ui::layout_components::Stack;
use craft_ui::popover::Popover;
use craft_ui::primitives::{DocRun, DocText, Draw, PaintShape, TextRun, Wrap};
use craft_ui::runtime::env::{Env, Label};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::style::{DataColor, StyleCx, TypeRole};
use craft_ui::text::TextField;
use egui::{Color32, CornerRadius, Pos2, Rect, Response, Ui, Vec2};

use super::avatar::ink_on;
use crate::collab::CollabRequest;

#[cfg(feature = "screen-testing")]
mod stories;

/// The bubble's text wraps at this many control heights.
const WIDTH_IN_HEIGHTS: f32 = 10.0;
/// How much of the ink the name over a bubble's text keeps.
const NAME_INK: f32 = 0.8;
/// Where a name tag or bubble sits from the pointer's tip, as fractions of the pointer's size.
pub(super) const TAG_OFFSET: [f32; 2] = [12.0 / 16.0, 1.0];

craft_ui::words! {
    /// The chat box's words, English by default.
    #[non_exhaustive]
    pub struct ChatWords {
        name: "chat.name" = "Cursor chat";
        hint: "chat.hint" = "Say something";
    }
}

/// The sizes of a bubble, from the style context of the component that paints it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BubbleLook {
    /// Room round the text.
    pub pad: Vec2,
    /// Between the name and the text.
    pub gap: f32,
    /// The corner radius, square at the pointer.
    pub radius: u8,
    /// From the pointer's tip to the bubble's corner.
    pub offset: Vec2,
    /// The width the text wraps at.
    pub width: f32,
    /// The body text size and line height.
    pub body: (f32, f32),
    /// The name's size and line height.
    pub caption: (f32, f32),
    /// The least contrast ratio of text on a person's colour.
    pub contrast: f64,
}

impl BubbleLook {
    /// The look the style context gives.
    pub fn of(cx: &StyleCx<'_>) -> Self {
        let (control, tokens) = (cx.control(), cx.tokens);
        let (body, caption) = (tokens.typography.get(TypeRole::Body), tokens.typography.get(TypeRole::Caption));
        BubbleLook {
            pad: Vec2::new(control.pad_x, tokens.space.gap.inline),
            gap: tokens.space.gap.inline / 2.0,
            radius: tokens.shape.radius.popup.round().clamp(0.0, 255.0) as u8,
            offset: Vec2::from(TAG_OFFSET) * tokens.size.icon.md,
            width: WIDTH_IN_HEIGHTS * control.height,
            body: (body.size, body.line_height),
            caption: (caption.size, caption.line_height),
            contrast: craft_theme::contrast::text_contrast_min(cx.contrast()),
        }
    }
}

/// A chat bubble with its corner beside the pointer's `tip`: the name over the text when given, on `fill` (the
/// person's colour) at `alpha`. As wide as its text, which wraps at the look's width; rounded except the corner at
/// the pointer, which is square.
pub fn paint_chat_bubble(draw: &Draw<'_>, look: &BubbleLook, tip: Pos2, name: Option<&str>, text: &str, fill: [u8; 3], alpha: f32) {
    let ink = ink_on(fill, look.contrast);
    let measured = draw.layout(&TextRun::new(text).role(TypeRole::Body).wrap(Wrap::Wrap), look.width).size();
    let head = name.map(|name| draw.layout(&TextRun::new(name).role(TypeRole::Caption), look.width).size());
    let head_height = head.map_or(0.0, |size| size.y + look.gap);
    let width = head.map_or(0.0, |size| size.x).max(measured.x);
    let rect = Rect::from_min_size(tip + look.offset, Vec2::new(width, head_height + measured.y) + 2.0 * look.pad);
    let round = look.radius;
    let [r, g, b] = fill;
    let background = Color32::from_rgb(r, g, b).gamma_multiply(alpha);
    draw.data_fill(&PaintShape::Rect(rect, CornerRadius { nw: 0, ne: round, sw: round, se: round }), DataColor::new(background));
    let mut at = rect.min + look.pad;
    if let Some(name) = name {
        let runs = [DocRun::new(name.to_owned(), FontRef::proportional(), look.caption.0, DataColor::new(ink.gamma_multiply(NAME_INK * alpha)))];
        draw.doc(Rect::from_min_size(at, Vec2::new(width, look.caption.1)), &DocText::new(&runs));
        at.y += head_height;
    }
    let runs = [DocRun::new(text.to_owned(), FontRef::proportional(), look.body.0, DataColor::new(ink.gamma_multiply(alpha)))];
    draw.doc(Rect::from_min_size(at, Vec2::new(width, measured.y)), &DocText::new(&runs));
}

/// The review commands outside a widget: `/` opens the cursor chat box. The app binds the chord and sets
/// [`CursorChatBox`]'s `open` flag when the command runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui::contract::CommandSet)]
#[command(prefix = "review")]
#[non_exhaustive]
pub enum ReviewCommand {
    /// Open the cursor chat box at the pointer.
    #[command(label = "Cursor chat", icon = "message_square", default = "/")]
    CursorChat,
}

/// Where the box was last shown, kept while it is open.
#[derive(Clone, Default)]
struct Last(Option<Pos2>);

/// Your cursor chat: while `open`, a text box at the pointer; each change comes back as a [`CollabRequest::Chat`] for
/// the app to send. Enter sends and closes the box, Escape or an empty Enter closes it and clears the message (an
/// empty `Chat`). The app opens it from [`ReviewCommand::CursorChat`].
#[derive(Pattern, Props)]
pub struct CursorChatBox<'a> {
    label: Label,
    text: &'a mut String,
    open: &'a mut bool,
}

impl<'a> CursorChatBox<'a> {
    /// A box that edits `text` while `open`.
    pub fn new(text: &'a mut String, open: &'a mut bool) -> Self {
        CursorChatBox { label: Label::Empty, text, open }
    }

    /// Shows the box and returns the requests it made. `pointer` is the pointer position in screen points, `None` off
    /// the canvas; an open box keeps its last position then.
    pub fn show(self, ui: &mut Ui, pointer: Option<Pos2>) -> CursorChatBoxOutput {
        let CursorChatBox { label: _, text, open } = self;
        let words = Env::of(ui).words::<ChatWords>();
        let mut requests = Vec::new();
        let shown = Stack::new().show(ui, |ui, cx| {
            if !*open {
                cx.keep::<Last>().0 = None;
                return;
            }
            let last = cx.keep::<Last>();
            last.0 = pointer.or(last.0);
            let Some(at) = last.0 else { return };
            let before = text.clone();
            let mut shown = true;
            let mut submitted = false;
            let popover = Popover::new(words.name.clone(), Bind::bound(&mut shown)).at_point(at).show(ui, |ui, _| {
                let field = TextField::new(words.name.clone(), Bind::bound(&mut *text)).placeholder(words.hint.clone()).show(ui);
                submitted = field.submitted;
            });
            if *text != before {
                requests.push(CollabRequest::Chat(text.clone()));
            }
            let escaped = popover.output.closed == Some(CloseReason::Escape);
            if escaped || (submitted && text.trim().is_empty()) {
                text.clear();
                requests.push(CollabRequest::Chat(String::new()));
                *open = false;
            } else if submitted {
                *open = false;
            }
        });
        CursorChatBoxOutput { response: shown.output, requests }
    }
}

/// What the chat box did this pass.
#[derive(Debug)]
#[non_exhaustive]
#[must_use]
pub struct CursorChatBoxOutput {
    /// The box's response.
    pub response: Response,
    /// The chat messages to send, in order.
    pub requests: Vec<CollabRequest>,
}

impl Outcome for CursorChatBoxOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}
