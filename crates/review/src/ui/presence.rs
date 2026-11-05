//! Avatars of the people in a shared document and their cursors on the canvas (N5.07).

use std::collections::HashMap;

use craft_theme::ControlSize;
use craft_ui::behavior::ClaimedKey;
use craft_ui::controls::{IconButton, Kbd};
use craft_ui::focus::key_owner::{KeyOwner, OwnerGate, owner};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, PressMenu};
use craft_ui::motion::{MIN_MOVE, glides_allowed};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use egui::text::LayoutJob;
use egui::{
    AsIdSalt, Color32, CornerRadius, IdSalt, Key, KeyboardShortcut, LayerId, Modifiers, Order, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

use super::chat::paint_chat_bubble;
use super::style::ReviewStyle;
use crate::collab::{CHAT_FADE_S, CHAT_HOLD_S, CollabRequest, CollabTransport, Participant, ParticipantId, chat_alpha, glide};

/// Most selection outlines drawn for one person.
const MAX_OUTLINES: usize = 500;
/// Most other people shown as avatars before the "+N" menu.
const MAX_SHOWN: usize = 5;
/// An avatar's text height as a fraction of its radius (FigCraft `live.rs:190`, `semibold(r * 0.8)`).
const AVATAR_TEXT: f32 = 0.8;
/// Distance between neighbouring avatars as a fraction of the avatar's diameter, they overlap
/// (FigCraft `live.rs:474-475`: `STEP` 20 over `R` 12, so 20 / 24).
const AVATAR_STEP: f32 = 20.0 / 24.0;
/// Width of the rings round avatars and of the following frame (FigCraft `live.rs:479`, `:458`).
const FRAME_WIDTH: f32 = 2.0;
/// Width of a selection outline and of the pointer's edge (FigCraft `live.rs:203`).
const OUTLINE_WIDTH: f32 = 1.5;
/// The weights of red, green and blue in a colour's luminance (FigCraft `live.rs:182`).
const LUMA_WEIGHTS: [f32; 3] = [0.299, 0.587, 0.114];
/// A person colour with a luminance above this gets the dark ink (FigCraft `live.rs:183`).
const DARK_INK_ABOVE: f32 = 170.0;
/// The key that stops following; the label names it.
const STOP_FOLLOWING_KEY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Escape);
/// The pointer arrow's corners (left, notch, right) as fractions of its size; the tip is the
/// origin.
const POINTER_SHAPE: [[f32; 2]; 3] = [[3.5 / 16.0, 15.5 / 16.0], [7.5 / 16.0, 8.5 / 16.0], [15.5 / 16.0, 4.0 / 16.0]];
/// Where a name tag or chat bubble sits from the pointer's tip, as fractions of the pointer's size.
const TAG_OFFSET: [f32; 2] = [12.0 / 16.0, 1.0];

craft_ui::words! {
    /// The people bar's words, English by default (FigCraft's).
    #[non_exhaustive]
    pub struct PresenceLabels {
        follow: "presence.follow" = "Follow {name}";
        stop_following: "presence.stopFollowing" = "Stop following";
        spotlight: "presence.spotlight" = "Spotlight me";
        stop_spotlight: "presence.stopSpotlight" = "Stop spotlight";
        more: "presence.more" = "+{n}";
        you: "presence.you" = "{name} (you)";
        viewer: "presence.viewer" = "Can view";
        following: "presence.following" = "Following {name} · {key} to stop";
        follow_viewer: "presence.followViewer" = "{action} ({viewer})";
        remove: "presence.remove" = "Remove {name} from session";
        people: "presence.people" = "People in this file";
        following_hint: "presence.followingHint" = "Following";
        nobody: "presence.nobody" = "Nobody else is here yet";
        not_live: "presence.notLive" = "Not in a live session";
    }
}

/// A round avatar of radius `radius` in `fill` with the initials ([`initials`](crate::collab::initials))
/// in the dark or light ink the fill's luminance asks for (`ink_on`).
// figcraft: crates/ui-egui/src/live.rs:188 `avatar`, `:182` `text_on`.
pub fn paint_avatar(painter: &Painter, tokens: &UiTokens, center: Pos2, radius: f32, name: &str, fill: Color32) {
    let on = ink_on(&ReviewStyle::of(tokens), fill);
    painter.circle_filled(center, radius, fill);
    let mut format = role_format(tokens, ReviewStyle::of(tokens).caption_text, on);
    format.font_id.size = radius * AVATAR_TEXT;
    let galley = painter.layout_job(LayoutJob::single_section(crate::collab::initials(name), format));
    painter.galley(center - galley.size() / 2.0, galley, on);
}

/// A remote pointer arrow with its tip at `tip`.
// figcraft: crates/ui-egui/src/live.rs:195 `pointer`.
pub fn paint_pointer(painter: &Painter, tokens: &UiTokens, tip: Pos2, fill: Color32) {
    let size = tokens.size.icon.md;
    let [left, notch, right] = POINTER_SHAPE.map(|[x, y]| tip + vec2(x, y) * size);
    painter.add(Shape::convex_polygon(vec![tip, left, notch], fill, Stroke::NONE));
    painter.add(Shape::convex_polygon(vec![tip, notch, right], fill, Stroke::NONE));
    painter.add(Shape::closed_line(vec![tip, left, notch, right], Stroke::new(OUTLINE_WIDTH, tokens.color.overlay.halo.to_egui())));
}

/// The ink that reads on a person's colour `fill`: `review.ink_dark` when its luminance is above
/// [`DARK_INK_ABOVE`], else `review.ink_light` (FigCraft `live.rs:182`).
pub(super) fn ink_on(style: &ReviewStyle, fill: Color32) -> Color32 {
    let [r, g, b, _] = fill.to_array();
    let luminance: f32 = [r, g, b].iter().zip(LUMA_WEIGHTS).map(|(channel, weight)| f32::from(*channel) * weight).sum();
    if luminance > DARK_INK_ABOVE { style.ink_dark } else { style.ink_light }
}

/// Where a name tag or chat bubble starts from a pointer's tip.
pub(super) fn tag_offset(tokens: &UiTokens) -> Vec2 {
    Vec2::from(TAG_OFFSET) * tokens.size.icon.md
}

fn person_color(person: &Participant) -> Color32 {
    let [r, g, b] = person.color;
    Color32::from_rgb(r, g, b)
}

/// A name in a person's colour, its top left corner at `at`.
fn paint_name_tag(painter: &Painter, tokens: &UiTokens, at: Pos2, name: &str, fill: Color32) {
    let style = ReviewStyle::of(tokens);
    let on = ink_on(&style, fill);
    let galley = painter.layout_job(LayoutJob::single_section(name.to_owned(), role_format(tokens, style.caption_text, on)));
    let pad = vec2(tokens.space.gap.inline, tokens.space.gap.inline / 2.0);
    let rect = Rect::from_min_size(at, galley.size() + 2.0 * pad);
    painter.rect_filled(rect, tokens.radius.control, fill);
    painter.galley(rect.min + pad, galley, on);
}

/// The avatars at the top of the right panel (FigCraft): yours first, then up to five others and
/// a "+N" chip for the rest and a chevron that opens the menu of everyone (you first; Following
/// marks who you follow; a line says when nobody else is here); a click on someone follows their view (a frame
/// in their colour around the canvas while following, Esc stops), a click on yours opens
/// Spotlight me, and a right-click on someone offers Remove when the transport allows it. Each
/// avatar is a Tab stop named by the person, Enter follows.
pub struct PeopleBar<'a> {
    id_salt: IdSalt,
    transport: &'a dyn CollabTransport,
    following: Option<ParticipantId>,
    labels: Option<PresenceLabels>,
}

impl<'a> PeopleBar<'a> {
    /// A bar of the people in `transport`'s session.
    pub fn new(id_salt: impl AsIdSalt, transport: &'a dyn CollabTransport) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, following: None, labels: None }
    }

    /// The person being followed, if any.
    pub fn following(mut self, who: Option<ParticipantId>) -> Self {
        self.following = who;
        self
    }

    /// Uses these label texts instead of the ones read from the active locale's catalog.
    pub fn labels(mut self, labels: PresenceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Follow, Spotlight and Remove come back as [`CollabRequest`]s. `canvas` is the canvas'
    /// rectangle in screen points: the following frame is drawn round it.
    // figcraft: crates/ui-egui/src/live.rs:473 `people_bar`, `:569` `people_menu`, `:461`
    // `follow_frame`.
    pub fn show(self, ui: &mut Ui, canvas: Rect) -> PeopleBarOutput {
        let Self { id_salt, transport, following, labels } = self;
        let labels = labels.unwrap_or_else(|| PresenceLabels::of(ui));
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let me_id = transport.me();
        let people = transport.participants();
        let me = people.iter().find(|person| person.id == me_id);
        let others: Vec<&Participant> = people.iter().filter(|person| person.id != me_id).collect();
        let mut requests = Vec::new();

        let side = tokens.size.control.sm;
        let radius = side / 2.0;
        let step = side * AVATAR_STEP;
        let margin = FRAME_WIDTH * 2.0;
        // Room for avatars after yours, the "+N" chip counting as one; the chevron takes the last slot.
        let room = match ui.available_width() {
            width if width.is_finite() => ((width - side - 2.0 * margin) / step - 1.0).floor().max(0.0) as usize,
            _ => usize::MAX,
        };
        let shown = if others.len() <= room.min(MAX_SHOWN) { others.len() } else { room.min(MAX_SHOWN + 1).saturating_sub(1) };
        let hidden = others.len() - shown;
        let slots = usize::from(me.is_some()) + shown + usize::from(hidden > 0) + 1;
        let width = 2.0 * margin + side + slots.saturating_sub(1) as f32 * step;
        let (rect, response) = ui.allocate_exact_size(vec2(width, side + 2.0 * margin), Sense::hover());
        let mut center = pos2(rect.min.x + margin + radius, rect.center().y);
        let advance = |center: &mut Pos2| {
            let here = *center;
            center.x += step;
            here
        };

        if let Some(me) = me {
            let at = advance(&mut center);
            let name = fmt(&labels.you, &[("name", &me.name)]);
            let button = avatar_button(ui, &tokens, id.with("me"), at, side, &me.name, person_color(me), me.spotlight, &name);
            let entry = if me.spotlight { &labels.stop_spotlight } else { &labels.spotlight };
            let entries = [MenuEntry::from(MenuItem::new(entry.as_str(), PeopleCommand::Spotlight))];
            if let Some(PeopleCommand::Spotlight) = PressMenu::new(&entries).show(ui, &button).chosen {
                requests.push(CollabRequest::Spotlight(!me.spotlight));
            }
        }
        for person in others.iter().take(shown) {
            let at = advance(&mut center);
            let followed = following == Some(person.id);
            let action = if followed { labels.stop_following.clone() } else { fmt(&labels.follow, &[("name", &person.name)]) };
            let name = if person.viewer { fmt(&labels.follow_viewer, &[("action", &action), ("viewer", &labels.viewer)]) } else { action };
            let button = avatar_button(ui, &tokens, id.with(("avatar", person.id)), at, side, &person.name, person_color(person), followed, &name);
            if button.clicked() {
                requests.push(CollabRequest::Follow(if followed { None } else { Some(person.id) }));
            }
            if transport.can_remove() {
                let entries = [MenuEntry::from(MenuItem::new(fmt(&labels.remove, &[("name", &person.name)]), PeopleCommand::Remove(person.id)))];
                if let Some(PeopleCommand::Remove(who)) = ContextMenu::new(&entries).show(ui, &button).chosen {
                    requests.push(CollabRequest::Remove(who));
                }
            }
        }
        if hidden > 0 {
            let at = advance(&mut center);
            let name = fmt(&labels.more, &[("n", &hidden.to_string())]);
            ring(ui.painter(), &tokens, at, radius);
            ui.painter().circle_filled(at, radius, tokens.color.surface.sunken.to_egui());
            let muted = tokens.color.text.muted.to_egui();
            let galley = ui.painter().layout_job(LayoutJob::single_section(name, role_format(&tokens, ReviewStyle::of(&tokens).caption_text, muted)));
            ui.painter().galley(at - galley.size() / 2.0, galley, muted);
        }
        let at = advance(&mut center);
        let chevron =
            IconButton::new(Icon::ChevronDown, &labels.people).size(ControlSize::Sm).show_at(ui, Rect::from_center_size(at, Vec2::splat(side)));
        let entries = people_entries(&labels, me, &others, following);
        if let Some(command) = PressMenu::new(&entries).show(ui, &chevron).chosen {
            match command {
                PeopleCommand::Follow(who) => requests.push(CollabRequest::Follow(if following == Some(who) { None } else { Some(who) })),
                PeopleCommand::You | PeopleCommand::Spotlight | PeopleCommand::Remove(_) => {}
            }
        }

        if let Some(person) = following.and_then(|who| others.iter().find(|person| person.id == who)) {
            let key = Kbd::new(&STOP_FOLLOWING_KEY).text(ui.ctx().os());
            let text = fmt(&labels.following, &[("name", &person.name), ("key", &key)]);
            paint_follow_frame(ui, &tokens, id, canvas, &text, person_color(person));
            // Taken in the pass it is pressed, before the canvas sees it (FigCraft `live.rs:428`).
            let stop = owner(ui.ctx(), &OwnerGate::default()) == KeyOwner::App
                && ui.input_mut(|input| ClaimedKey::exact(STOP_FOLLOWING_KEY.modifiers, STOP_FOLLOWING_KEY.logical_key).consume(input));
            if stop {
                requests.push(CollabRequest::Follow(None));
            }
        }
        PeopleBarOutput { response, requests }
    }
}

#[derive(Clone, Copy)]
enum PeopleCommand {
    /// Your own row: it lists you and does nothing (FigCraft `live.rs:532`).
    You,
    Follow(ParticipantId),
    Remove(ParticipantId),
    Spotlight,
}

/// The chevron menu's rows: you, then everyone else with a mark on who is followed, or the line
/// that says why nobody else is listed (FigCraft `live.rs:569` `people_menu`).
fn people_entries(
    labels: &PresenceLabels,
    me: Option<&Participant>,
    others: &[&Participant],
    following: Option<ParticipantId>,
) -> Vec<MenuEntry<PeopleCommand>> {
    let mut entries = Vec::new();
    if let Some(me) = me {
        let name = fmt(&labels.you, &[("name", &me.name)]);
        entries.push(MenuItem::new(name, PeopleCommand::You).swatch(person_color(me)).into());
    }
    for person in others {
        let followed = following == Some(person.id);
        let row = MenuItem::new(person.name.as_str(), PeopleCommand::Follow(person.id)).swatch(person_color(person)).check(followed);
        entries.push(if followed { row.detail(&labels.following_hint) } else { row }.into());
    }
    if others.is_empty() {
        let note = if me.is_some() { &labels.nobody } else { &labels.not_live };
        entries.push(MenuItem::new(note.as_str(), PeopleCommand::You).enabled(false).into());
    }
    entries
}

/// The ring that separates overlapping avatars from the panel.
fn ring(painter: &Painter, tokens: &UiTokens, center: Pos2, radius: f32) {
    painter.circle_stroke(center, radius + FRAME_WIDTH / 2.0, Stroke::new(FRAME_WIDTH, tokens.color.surface.panel.to_egui()));
}

/// One avatar as a button named `name`; a ring in the person's colour shows while it is hovered,
/// focused or `active`.
#[expect(clippy::too_many_arguments, reason = "one avatar's look and name")]
fn avatar_button(
    ui: &Ui,
    tokens: &UiTokens,
    id: egui::Id,
    center: Pos2,
    side: f32,
    person: &str,
    fill: Color32,
    active: bool,
    name: &str,
) -> Response {
    let response = ui.interact(Rect::from_center_size(center, Vec2::splat(side)), id, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    ring(ui.painter(), tokens, center, side / 2.0);
    paint_avatar(ui.painter(), tokens, center, side / 2.0, person, fill);
    if active || response.hovered() || response.has_focus() {
        ui.painter().circle_stroke(center, side / 2.0 + FRAME_WIDTH, Stroke::new(FRAME_WIDTH, fill));
    }
    response.on_hover_text(name)
}

/// A frame in the followed person's colour round `area` (the canvas), with `text` at the top.
fn paint_follow_frame(ui: &Ui, tokens: &UiTokens, id: egui::Id, area: Rect, text: &str, fill: Color32) {
    let painter = ui.ctx().layer_painter(LayerId::new(Order::Foreground, id.with("follow"))).with_clip_rect(area);
    painter.rect_stroke(area.shrink(FRAME_WIDTH / 2.0), CornerRadius::ZERO, Stroke::new(FRAME_WIDTH, fill), StrokeKind::Inside);
    let style = ReviewStyle::of(tokens);
    let on = ink_on(&style, fill);
    let galley = painter.layout_job(LayoutJob::single_section(text.to_owned(), role_format(tokens, style.caption_text, on)));
    let pad = vec2(tokens.space.pad.control, tokens.space.gap.inline / 2.0);
    let tag =
        Rect::from_center_size(pos2(area.center().x, area.min.y + FRAME_WIDTH + (galley.size().y + 2.0 * pad.y) / 2.0), galley.size() + 2.0 * pad);
    painter.rect_filled(tag, CornerRadius { nw: 0, ne: 0, ..tokens.radius.control }, fill);
    painter.galley(tag.min + pad, galley, on);
}

/// What the people bar did this pass.
#[must_use]
pub struct PeopleBarOutput {
    /// The bar's response.
    pub response: Response,
    /// Follow and Spotlight, in order.
    pub requests: Vec<CollabRequest>,
}

/// Other people's cursors over the canvas: a pointer and a name tag in their colour, gliding to
/// each reported position ([`glide`]), their selection outlines (at most
/// 500), and their cursor chat bubbles ([`paint_chat_bubble`]).
/// Repaints only while something glides or fades. Not interactive.
pub struct RemoteCursors<'a> {
    id_salt: IdSalt,
    transport: &'a dyn CollabTransport,
}

impl<'a> RemoteCursors<'a> {
    /// Cursors for the people in `transport`'s session.
    pub fn new(id_salt: impl AsIdSalt, transport: &'a dyn CollabTransport) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport }
    }

    /// Paints inside `canvas`, the canvas' rectangle in screen points. `to_screen` maps document
    /// units to screen points.
    // figcraft: crates/ui-egui/src/live.rs:273 `draw`, `:205` `name_tag`.
    pub fn paint(&self, ui: &Ui, canvas: Rect, to_screen: impl Fn([f64; 2]) -> Pos2) {
        let ctx = ui.ctx();
        let tokens = Tokens::of(ui);
        let painter = ui.painter().with_clip_rect(canvas);
        let state_id = ui.make_persistent_id(self.id_salt);
        let mut state: Glides = ctx.data(|data| data.get_temp(state_id)).unwrap_or_default();
        let now = ctx.input(|input| input.time);
        let dt = now - state.last;
        let reduce = !glides_allowed(ctx);
        let me = self.transport.me();
        let offset = tag_offset(&tokens);
        let mut shown = HashMap::new();
        let mut gliding = false;
        for person in self.transport.participants().iter().filter(|person| person.id != me) {
            let fill = person_color(person);
            for [min, max] in person.selection.iter().take(MAX_OUTLINES) {
                let outline = Rect::from_two_pos(to_screen(*min), to_screen(*max));
                painter.rect_stroke(outline, CornerRadius::ZERO, Stroke::new(OUTLINE_WIDTH, fill), StrokeKind::Middle);
            }
            let Some(target) = person.cursor else { continue };
            let from = state.shown.get(&person.id).copied().unwrap_or(target);
            let mut next = glide(from, target, dt, reduce);
            if (to_screen(next) - to_screen(target)).length() < MIN_MOVE {
                next = target;
            } else {
                gliding = true;
            }
            shown.insert(person.id, next);
            let tip = to_screen(next);
            paint_pointer(&painter, &tokens, tip, fill);
            let bubble = person.chat.as_ref().filter(|chat| !chat.text.is_empty()).and_then(|chat| {
                let age = now - chat.changed;
                let alpha = chat_alpha(age, reduce);
                schedule_chat_frames(ctx, age, alpha);
                (alpha > 0.0).then_some((chat, alpha))
            });
            match bubble {
                Some((chat, alpha)) => paint_chat_bubble(&painter, &tokens, tip, Some(&person.name), &chat.text, fill, alpha),
                None => paint_name_tag(&painter, &tokens, tip + offset, &person.name, fill),
            }
        }
        state.shown = shown;
        state.last = now;
        ctx.data_mut(|data| data.insert_temp(state_id, state));
        if gliding {
            ctx.request_repaint();
        }
    }
}

/// Where each remote cursor is drawn now, and the time of the last pass.
#[derive(Clone, Default)]
struct Glides {
    shown: HashMap<ParticipantId, [f64; 2]>,
    last: f64,
}

/// Asks for the frame where a chat message next looks different: its fade, or the end of its hold.
fn schedule_chat_frames(ctx: &egui::Context, age: f64, alpha: f32) {
    if age < CHAT_HOLD_S {
        ctx.request_repaint_after_secs((CHAT_HOLD_S - age) as f32);
    } else if alpha > 0.0 && alpha < 1.0 {
        ctx.request_repaint();
    } else if alpha > 0.0 {
        ctx.request_repaint_after_secs((CHAT_HOLD_S + CHAT_FADE_S - age) as f32);
    }
}
