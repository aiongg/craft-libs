//! Avatars of the people in a shared document and their cursors on the canvas (N5.07).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Color32, IdSalt, Painter, Pos2, Rect, Response, Ui};

use crate::collab::{CollabRequest, CollabTransport, ParticipantId};

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
        following: "presence.following" = "Following {name}";
    }
}

/// A round avatar of radius `radius` in `fill` with the initials in a legible text colour
/// (black or white by the fill's luminance).
// figcraft: crates/ui-egui/src/live.rs:188 `avatar`, `:182` `text_on`.
pub fn paint_avatar(painter: &Painter, center: Pos2, radius: f32, name: &str, fill: Color32) {
    todo!()
}

/// A remote pointer arrow with its tip at `tip`.
// figcraft: crates/ui-egui/src/live.rs:195 `pointer`.
pub fn paint_pointer(painter: &Painter, tip: Pos2, fill: Color32) {
    todo!()
}

/// The avatars at the top of the right panel (FigCraft): yours last, up to five others and a
/// "+N" overflow menu; a click on someone follows their view (a frame in their colour around the
/// canvas while following), a click on yours opens Spotlight me. Each avatar is a Tab stop named
/// by the person, Enter follows.
pub struct PeopleBar<'a> {
    id_salt: IdSalt,
    transport: &'a dyn CollabTransport,
    following: Option<ParticipantId>,
    labels: Option<PresenceLabels>,
}

impl<'a> PeopleBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, transport: &'a dyn CollabTransport) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, following: None, labels: None }
    }

    pub fn following(mut self, who: Option<ParticipantId>) -> Self {
        self.following = who;
        self
    }

    pub fn labels(mut self, labels: PresenceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Follow and Spotlight come back as [`CollabRequest`]s.
    // figcraft: crates/ui-egui/src/live.rs:473 `people_bar`, `:569` `people_menu`, `:461`
    // `follow_frame`.
    pub fn show(self, ui: &mut Ui) -> PeopleBarOutput {
        todo!()
    }
}

/// What the people bar did this pass.
#[must_use]
pub struct PeopleBarOutput {
    pub response: Response,
    /// Follow and Spotlight, in order.
    pub requests: Vec<CollabRequest>,
}

/// Other people's cursors over the canvas: a pointer and a name tag in their colour, gliding to
/// each reported position ([`glide`](crate::collab::glide)), their selection outlines (at most
/// 500), and their cursor chat bubbles ([`paint_chat_bubble`](super::paint_chat_bubble)).
/// Repaints only while something glides or fades. Not interactive.
pub struct RemoteCursors<'a> {
    id_salt: IdSalt,
    transport: &'a dyn CollabTransport,
    canvas: Rect,
}

impl<'a> RemoteCursors<'a> {
    pub fn new(id_salt: impl AsIdSalt, transport: &'a dyn CollabTransport, canvas: Rect) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, canvas }
    }

    /// `to_screen` maps document units to screen points.
    // figcraft: crates/ui-egui/src/live.rs:273 `draw`, `:205` `name_tag`.
    pub fn paint(&self, ui: &Ui, to_screen: impl Fn([f64; 2]) -> Pos2) {
        todo!()
    }
}
