//! The changes between a document and a revised copy, to accept or reject one by one (N5.10).

/// What a change did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RevisionKind {
    Inserted,
    Deleted,
    Changed,
}

/// One change, as the app's compare found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub kind: RevisionKind,
    /// The app's heading, a link to the place ("Slide 3 changed", "Slide inserted after 2").
    pub heading: String,
    /// Up to eight detail lines are shown ("Title text changed").
    pub details: Vec<String>,
}

/// What the revisions panel asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionAction {
    /// The heading was clicked: go to the change.
    Go(usize),
    Accept(usize),
    Reject(usize),
    AcceptAll,
    RejectAll,
    /// "End Review" once no differences are left.
    EndReview,
}
