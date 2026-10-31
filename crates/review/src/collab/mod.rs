//! Collaboration: presence, voting, cursor chat and joining a shared document by code
//! (N5.07–N5.09, N5.11), over a [`CollabTransport`] the app implements. The library has no
//! network: FigCraft's sessions, servers and codes stay FigCraft's; the widgets read and ask
//! through the trait.

pub mod chat;
pub mod join;
pub mod presence;
pub mod vote;

pub use chat::{CHAT_FADE_S, CHAT_HOLD_S, CHAT_WIDTH, CursorChat, chat_alpha};
pub use join::JoinState;
pub use presence::{GLIDE, PEOPLE_COLORS, Participant, ParticipantId, glide, initials};
pub use vote::{VoteScope, VoteSession};

/// The app's side of a shared session: who is here and what they do, and the requests the
/// widgets make. Implemented by the app on its session (FigCraft: over its `collab.*` commands);
/// the widgets call it during `show`, so a method should not block (queue and return).
///
/// ```ignore
/// impl CollabTransport for Live<'_> {
///     fn me(&self) -> ParticipantId { ParticipantId(self.session.me) }
///     fn participants(&self) -> Vec<Participant> { self.session.peers().map(to_participant).collect() }
///     fn send_cursor(&mut self, at: Option<[f64; 2]>) { self.app.run("collab.cursor", json!({"at": at})); }
///     // …
/// }
/// ```
pub trait CollabTransport {
    fn me(&self) -> ParticipantId;

    /// Everyone in the session, you included.
    fn participants(&self) -> Vec<Participant>;

    /// Your pointer on the canvas (document units), `None` when it left.
    fn send_cursor(&mut self, at: Option<[f64; 2]>);

    /// Your cursor chat text; empty closes the bubble.
    fn send_chat(&mut self, text: &str);

    /// Follow someone's view, or stop.
    fn follow(&mut self, who: Option<ParticipantId>);

    /// Ask everyone to follow you, or stop.
    fn spotlight(&mut self, on: bool);

    /// The running or last vote.
    fn vote(&self) -> Option<VoteSession>;

    fn start_vote(&mut self, per_person: u32, scope: VoteScope);

    fn end_vote(&mut self);

    /// Join with what the user typed (a code or an invite link). The transport parses it.
    fn join(&mut self, typed: &str);

    fn join_state(&self) -> JoinState;

    fn cancel_join(&mut self);
}
