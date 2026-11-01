//! The review widgets, for egui. The look follows each source app's original: Acrobat for the
//! comments panel, annotations and signatures (PdfCraft), Figma for threads, pins, versions and
//! collaboration (FigCraft), PowerPoint for revisions (DeckCraft).
//!
//! - [`comments`] has the [`CommentsPanel`].
//! - [`thread`] has [`CommentPins`], the [`ThreadPopover`] and the [`CommentComposer`].
//! - [`annotate`] has the [`AnnotateTool`] (a canvas tool), the [`AnnotationOverlay`] and the
//!   [`AnnotationToolbar`].
//! - [`signature`] has the [`SignaturePadDialog`] and [`paint_signature`].
//! - [`versions`] has the [`VersionHistoryPanel`] and the [`VersionPreviewBanner`].
//! - [`revisions`] has the [`RevisionsPanel`].
//! - [`style`] has [`ReviewStyle`], every size the widgets read (`review.*` tokens).
//! - Collaboration: [`presence`] ([`PeopleBar`], [`RemoteCursors`]), [`vote`] ([`VotePill`]),
//!   [`chat`] ([`CursorChatBox`]) and [`join`] ([`JoinField`]).

pub mod annotate;
pub mod comments;
pub mod revisions;
pub mod signature;
pub mod style;
pub mod thread;
pub mod versions;

pub mod chat;
pub mod join;
pub mod presence;
pub mod vote;

mod stories;

#[cfg(test)]
mod annotate_tests;
#[cfg(test)]
mod collab_tests;
#[cfg(test)]
mod comments_tests;
#[cfg(test)]
mod revisions_tests;
#[cfg(test)]
mod signature_tests;
#[cfg(test)]
mod thread_tests;
#[cfg(test)]
mod versions_tests;

pub use annotate::{AnnotateLabels, AnnotateTool, AnnotationOverlay, AnnotationToolbar};
pub use comments::{CommentEdit, CommentsLabels, CommentsPanel, CommentsPanelOutput, CommentsPanelState};
pub use revisions::{RevisionsLabels, RevisionsPanel, RevisionsPanelOutput};
pub use signature::{SignatureLabels, SignaturePadDialog, SignatureSettings, paint_signature};
pub use style::ReviewStyle;
pub use thread::{
    CommentComposer, CommentComposerResponse, CommentPins, CommentPinsOutput, PinCluster, PinMove, REACTIONS, ThreadLabels, ThreadPopover,
    ThreadPopoverOutput, cluster, insert_mention, mention_matches, mention_query,
};
pub use versions::{VersionAction, VersionHistoryPanel, VersionHistoryPanelOutput, VersionPreviewBanner, VersionsLabels};

pub use chat::{CursorChatBox, CursorChatBoxOutput, paint_chat_bubble};
pub use join::{JoinField, JoinFieldOutput, JoinLabels};
pub use presence::{PeopleBar, PeopleBarOutput, PresenceLabels, RemoteCursors, paint_avatar, paint_pointer};
pub use vote::{VoteLabels, VotePill, VotePillOutput};

use craft_ui::localized::{Labels, Word};

/// The stories of the review widgets (not registered yet; see the spec).
pub fn stories() -> Vec<craft_ui::story::Story> {
    stories::stories()
}

/// The labels tables of the review widgets, for the words registry.
pub const WORDS: &[&[Word]] = &[
    <CommentsLabels as Labels>::WORDS,
    <ThreadLabels as Labels>::WORDS,
    <AnnotateLabels as Labels>::WORDS,
    <SignatureLabels as Labels>::WORDS,
    <VersionsLabels as Labels>::WORDS,
    <RevisionsLabels as Labels>::WORDS,
    <PresenceLabels as Labels>::WORDS,
    <VoteLabels as Labels>::WORDS,
    <JoinLabels as Labels>::WORDS,
];
