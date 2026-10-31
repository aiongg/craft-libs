//! Review: comment threads, annotations, signatures, version history and revisions; with the
//! `collab` feature, presence, voting, cursor chat and joining over the app's transport. With
//! the `ui` feature, [`ui`] has the review widgets.
//!
//! Review data is the app's document data: every widget reads it and reports edits; nothing here
//! owns or stores it.

pub mod annotate;
pub mod comments;
pub mod revisions;
pub mod signature;
pub mod versions;

#[cfg(feature = "collab")]
pub mod collab;

#[cfg(feature = "ui")]
pub mod ui;

#[cfg(test)]
mod annotate_tests;
#[cfg(test)]
mod comments_tests;
#[cfg(test)]
mod signature_tests;
#[cfg(test)]
mod versions_tests;

pub use annotate::{Annotation, AnnotationEdit, AnnotationGroup, AnnotationHandle, AnnotationHit, AnnotationId, AnnotationKind, AnnotationTool};
pub use comments::{CommentAnchor, CommentMessage, CommentSort, CommentThread, Reaction, ReviewStatus, ThreadId};
pub use revisions::{Revision, RevisionAction, RevisionKind};
pub use signature::{SavedSignature, SignatureDraft, SignatureImage, SignatureTab};
pub use versions::{Branch, BranchId, BranchState, HistoryRow, Version, VersionId};
