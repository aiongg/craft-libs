//! Avatars of the people in a shared document and their cursors on the canvas.

use craft_behavior::kurbo::Point;
use craft_behavior::{IconId, Input, Metrics};
use craft_theme::Srgba;
use craft_theme::variant::Variant;
use craft_ui::buttons::IconButton;
use craft_ui::canvas::canvas_space;
use craft_ui::contract::{
    Commands as _, Common, Component, Cx, Entry, MeasureCx, NoValue, Outcome, PaintUi, Pattern, Props, Role, Show as _, ShowContainer as _, Trigger,
};
use craft_ui::display::Badge;
use craft_ui::layout_components::{Row, Stack};
use craft_ui::menu::Menu;
use craft_ui::primitives::{Draw, PaintShape, StrokeWidth};
use craft_ui::runtime::env::{Env, Label};
use craft_ui::style::{ComponentStyle, DataColor, StyleColor, StyleCx};
use egui::{Color32, CornerRadius, Pos2, Rect, Response, Ui, Vec2};

use super::avatar::Avatar;
use super::chat::{BubbleLook, paint_chat_bubble};
use crate::collab::{CollabRequest, CollabTransport, Participant, ParticipantId, chat_alpha};

#[cfg(feature = "screen-testing")]
mod stories;

/// Most selection outlines drawn for one person.
const MAX_OUTLINES: usize = 500;
/// Most other people shown as avatars before the "+N" badge.
const MAX_SHOWN: usize = 5;
/// The pointer arrow's corners (left, notch, right) as fractions of its size; the tip is the origin.
const POINTER_SHAPE: [[f32; 2]; 3] = [[3.5 / 16.0, 15.5 / 16.0], [7.5 / 16.0, 8.5 / 16.0], [15.5 / 16.0, 4.0 / 16.0]];

craft_ui::words! {
    /// The people bar's words, English by default.
    #[non_exhaustive]
    pub struct PresenceWords {
        follow: template "presence.follow" = "Follow {name}";
        stop_following: "presence.stopFollowing" = "Stop following";
        spotlight: "presence.spotlight" = "Spotlight me";
        stop_spotlight: "presence.stopSpotlight" = "Stop spotlight";
        more: template "presence.more" = "+{n}";
        you: template "presence.you" = "{name} (you)";
        viewer: "presence.viewer" = "Can view";
        following: template "presence.following" = "Following {name} \u{b7} {key} to stop";
        follow_viewer: template "presence.followViewer" = "{action} ({viewer})";
        remove: template "presence.remove" = "Remove {name} from session";
        people: "presence.people" = "People in this file";
        nobody: "presence.nobody" = "Nobody else is here yet";
        not_live: "presence.notLive" = "Not in a live session";
        stop_key: "presence.stopKey" = "Esc";
    }
}

fn swatch(color: [u8; 3]) -> Srgba {
    let [r, g, b] = color.map(|channel| f32::from(channel) / 255.0);
    Srgba::rgb(r, g, b)
}

#[derive(Clone, Copy, PartialEq)]
enum PeopleCommand {
    /// Your own row: it lists you and does nothing.
    You,
    Follow(ParticipantId),
    Remove(ParticipantId),
}

/// The avatars at the top of the right panel: yours first, then up to five others and a "+N" badge for the rest and a
/// chevron that opens the menu of everyone (you first; a check marks who you follow; a line says when nobody else is
/// here). A click on someone follows their view, a click on yours toggles Spotlight, and a context menu on someone
/// offers Remove when the transport allows it. Each avatar is a Tab stop named by the person.
#[derive(Pattern, Props)]
pub struct PeopleBar<'a> {
    label: Label,
    transport: &'a dyn CollabTransport,
    /// The person being followed, if any.
    #[prop]
    following: Option<ParticipantId>,
}

impl<'a> PeopleBar<'a> {
    /// A bar of the people in `transport`'s session.
    pub fn new(transport: &'a dyn CollabTransport) -> Self {
        PeopleBar { label: Label::Empty, transport, following: None }
    }

    /// Shows the bar. Follow, Spotlight and Remove come back as [`CollabRequest`]s.
    pub fn show(self, ui: &mut Ui) -> PeopleBarOutput {
        let PeopleBar { label, transport, following } = self;
        let env = Env::of(ui);
        let words = env.words::<PresenceWords>();
        let me_id = transport.me();
        let people = transport.participants();
        let me = people.iter().find(|person| person.id == me_id);
        let others: Vec<&Participant> = people.iter().filter(|person| person.id != me_id).collect();
        let shown = others.len().min(MAX_SHOWN);
        let hidden = others.len() - shown;
        let mut requests = Vec::new();
        let output = Stack::new().label(label).show(ui, |ui, _| {
            Row::new().show(ui, |ui, _| {
                if let Some(me) = me {
                    let action = if me.spotlight { words.stop_spotlight.clone() } else { words.spotlight.clone() };
                    let avatar =
                        Avatar::new(me.name.clone()).fill(me.color).pressable(true).selected(me.spotlight).description(action.as_str().to_owned());
                    if avatar.show(ui).clicked {
                        requests.push(CollabRequest::Spotlight(!me.spotlight));
                    }
                }
                for person in others.iter().take(shown) {
                    let followed = following == Some(person.id);
                    let action =
                        if followed { words.stop_following.as_str().to_owned() } else { words.follow.fill(&[("name", person.name.as_str())]) };
                    let name = if person.viewer {
                        words.follow_viewer.fill(&[("action", action.as_str()), ("viewer", words.viewer.as_str())])
                    } else {
                        action
                    };
                    let avatar = Avatar::new(person.name.clone()).fill(person.color).pressable(true).selected(followed).description(name).show(ui);
                    if avatar.clicked {
                        requests.push(CollabRequest::Follow(if followed { None } else { Some(person.id) }));
                    }
                    if transport.can_remove() {
                        let remove = words.remove.fill(&[("name", person.name.as_str())]);
                        let entries = [Entry::new(PeopleCommand::Remove(person.id), remove.clone())];
                        if let Some(PeopleCommand::Remove(who)) = Menu::context(remove, &avatar.response).pick(ui, &entries).picked {
                            requests.push(CollabRequest::Remove(who));
                        }
                    }
                }
                if hidden > 0 {
                    Badge::new(words.more.fill(&[("n", env.format().integer(hidden as u64).as_str())])).show(ui);
                }
                let chevron = IconButton::new(IconId::new("chevron_down")).label(words.people.clone()).show(ui);
                let mut entries = Vec::new();
                if let Some(me) = me {
                    entries.push(Entry::new(PeopleCommand::You, words.you.fill(&[("name", me.name.as_str())])).swatch(swatch(me.color)));
                }
                for person in &others {
                    let followed = following == Some(person.id);
                    entries.push(Entry::new(PeopleCommand::Follow(person.id), person.name.clone()).swatch(swatch(person.color)).checked(followed));
                }
                if others.is_empty() {
                    let note = if me.is_some() { &words.nobody } else { &words.not_live };
                    entries.push(Entry::new(PeopleCommand::You, note.clone()).enabled(false));
                }
                if let Some(PeopleCommand::Follow(who)) =
                    Menu::new(words.people.clone()).opened_by(&chevron, Trigger::Press).pick(ui, &entries).picked
                {
                    requests.push(CollabRequest::Follow(if following == Some(who) { None } else { Some(who) }));
                }
            });
        });
        PeopleBarOutput { response: output.output, requests }
    }
}

/// What the people bar did this pass.
#[derive(Debug)]
#[non_exhaustive]
#[must_use]
pub struct PeopleBarOutput {
    /// The bar's response.
    pub response: Response,
    /// Follow, Spotlight and Remove, in order.
    pub requests: Vec<CollabRequest>,
}

impl Outcome for PeopleBarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of the cursor and follow-frame painters.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CursorStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The pointer arrow's edge.
    halo: StyleColor,
    /// The pointer arrow's size.
    pointer: f32,
    bubble: BubbleLook,
}

fn cursor_style(cx: &StyleCx<'_>) -> CursorStyle {
    CursorStyle { metrics: Metrics::new(), halo: cx.color(cx.tokens.color.overlay.halo), pointer: cx.tokens.size.icon.md, bubble: BubbleLook::of(cx) }
}

fn data(color: [u8; 3]) -> DataColor {
    let [r, g, b] = color;
    DataColor::new(Color32::from_rgb(r, g, b))
}

/// A remote pointer arrow with its tip at `tip`, `size` points tall, in `fill` with the halo round it.
pub fn paint_pointer(draw: &Draw<'_>, halo: StyleColor, size: f32, tip: Pos2, fill: [u8; 3]) {
    let [left, notch, right] = POINTER_SHAPE.map(|[x, y]| tip + Vec2::new(x, y) * size);
    draw.data_fill(&PaintShape::Polygon(vec![tip, left, notch]), data(fill));
    draw.data_fill(&PaintShape::Polygon(vec![tip, notch, right]), data(fill));
    draw.stroke(&PaintShape::Path(vec![tip, left, notch, right, tip]), StrokeWidth::Thin, halo);
}

/// A name on a person's colour, its top left corner at `at`.
fn paint_name_tag(ui: &PaintUi<'_>, look: &BubbleLook, at: Pos2, name: &str, fill: [u8; 3]) {
    let draw = ui.draw();
    let ink = ink_on_person(ui, fill);
    let run = craft_ui::primitives::TextRun::new(name).role(craft_ui::style::TypeRole::Caption).color(ink);
    let width = draw.layout(&run, look.width).size().x;
    let pad = Vec2::new(look.pad.y, look.pad.y / 2.0);
    let rect = Rect::from_min_size(at, Vec2::new(width, look.caption.1) + 2.0 * pad);
    draw.data_fill(&PaintShape::Rect(rect, CornerRadius::same(look.radius)), data(fill));
    draw.text(Rect::from_min_size(rect.min + pad, Vec2::new(width, look.caption.1)), &run);
}

/// The ink that reads on a person's colour.
fn ink_on_person(ui: &PaintUi<'_>, fill: [u8; 3]) -> StyleColor {
    ui.with_style(Variant::SECONDARY, |cx| cx.ink_on(data(fill)))
}

/// Other people's cursors over the canvas: a pointer and a name tag in their colour at each reported position, their
/// selection outlines (at most 500) and their cursor chat bubbles. `now` is the clock the chat stamps use; a bubble
/// shows for its hold and fades out. Not interactive.
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(canvas_control)]
pub struct RemoteCursors<'a> {
    #[common]
    common: Common,
    transport: &'a dyn CollabTransport,
    now: f64,
    /// Chat bubbles do not fade (the app's reduced-motion setting).
    #[prop(name = "reduced_motion")]
    reduced: bool,
}

impl<'a> RemoteCursors<'a> {
    /// Cursors for the people in `transport`'s session, named `label`.
    pub fn new(label: impl Into<Label>, transport: &'a dyn CollabTransport, now: f64) -> Self {
        RemoteCursors { common: Common::new(), transport, now, reduced: false }.label(label)
    }
}

impl Component for RemoteCursors<'_> {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    const INPUT: Input = Input::Display;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = CursorStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: RemoteCursorsStyleKey) -> CursorStyle {
        cursor_style(cx)
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn access(&self, access: &mut craft_ui::contract::Access, _: &Cx<'_, Self>) {
        access.role(Role::None);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let (view, style) = (space.view, cx.style);
        let draw = ui.draw();
        let me = self.transport.me();
        let screen = |p: [f64; 2]| view.to_screen(Point::new(p[0], p[1]));
        for person in self.transport.participants().iter().filter(|person| person.id != me) {
            for [min, max] in person.selection.iter().take(MAX_OUTLINES) {
                let outline = Rect::from_two_pos(screen(*min), screen(*max));
                draw.data_stroke(&PaintShape::Rect(outline, CornerRadius::ZERO), StrokeWidth::Thin, data(person.color));
            }
            let Some(cursor) = person.cursor else { continue };
            let tip = screen(cursor);
            paint_pointer(&draw, style.halo, style.pointer, tip, person.color);
            let bubble = person.chat.as_ref().filter(|chat| !chat.text.is_empty()).and_then(|chat| {
                let alpha = chat_alpha(self.now - chat.changed, self.reduced);
                (alpha > 0.0).then_some((chat, alpha))
            });
            match bubble {
                Some((chat, alpha)) => paint_chat_bubble(ui, &style.bubble, tip, Some(&person.name), &chat.text, person.color, alpha),
                None => paint_name_tag(ui, &style.bubble, tip + style.bubble.offset, &person.name, person.color),
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The commands of the follow frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui::contract::CommandSet)]
#[command(prefix = "review")]
#[non_exhaustive]
pub enum FollowCommand {
    /// Stop following the person whose view the canvas shows.
    #[command(label = "Stop following", default = "Escape")]
    StopFollowing,
}

/// A frame in the followed person's colour round the canvas, with "Following {name} · Esc to stop" at the top. The
/// app shows it in the canvas body while it follows someone and stops following when the output says `stopped`
/// (Escape, [`FollowCommand::StopFollowing`]).
#[derive(craft_ui::contract::Component)]
#[component(canvas_control)]
pub struct FollowFrame {
    #[common]
    common: Common,
    color: [u8; 3],
    text: String,
    stopped: bool,
}

/// What the follow frame did this pass.
#[derive(Debug)]
#[non_exhaustive]
#[must_use]
pub struct FollowFrameOutput {
    /// The frame's response.
    pub response: Response,
    /// The user asked to stop following (Escape).
    pub stopped: bool,
}

impl Outcome for FollowFrameOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl FollowFrame {
    /// A frame in `color` with `text` (built from [`PresenceWords`]'s `following`) at the top.
    pub fn new(label: impl Into<Label>, color: [u8; 3], text: impl Into<String>) -> Self {
        FollowFrame { common: Common::new(), color, text: text.into(), stopped: false }.label(label)
    }
}

impl Component for FollowFrame {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    const INPUT: Input = Input::Display;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = CursorStyle;
    type Output = FollowFrameOutput;

    fn style(cx: &StyleCx<'_>, _: FollowFrameStyleKey) -> CursorStyle {
        cursor_style(cx)
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        while cx.take_command::<FollowCommand>().is_some() {
            self.stopped = true;
        }
    }

    fn access(&self, access: &mut craft_ui::contract::Access, _: &Cx<'_, Self>) {
        access.role(Role::None);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let (area, look) = (space.view.rect, &cx.style.bubble);
        let draw = ui.draw();
        draw.data_stroke(&PaintShape::Rect(area, CornerRadius::ZERO), StrokeWidth::Thick, data(self.color));
        let run =
            craft_ui::primitives::TextRun::new(self.text.as_str()).role(craft_ui::style::TypeRole::Caption).color(ink_on_person(ui, self.color));
        let width = draw.layout(&run, look.width * 2.0).size().x;
        let pad = Vec2::new(look.pad.x, look.pad.y / 2.0);
        let tag = Rect::from_min_size(Pos2::new(area.center().x - width / 2.0 - pad.x, area.top()), Vec2::new(width, look.caption.1) + 2.0 * pad);
        draw.data_fill(&PaintShape::Rect(tag, CornerRadius { nw: 0, ne: 0, sw: look.radius, se: look.radius }), data(self.color));
        draw.text(Rect::from_min_size(tag.min + pad, Vec2::new(width, look.caption.1)), &run);
    }

    fn finish(self, cx: Cx<'_, Self>) -> FollowFrameOutput {
        FollowFrameOutput { response: cx.response.clone(), stopped: self.stopped }
    }
}
