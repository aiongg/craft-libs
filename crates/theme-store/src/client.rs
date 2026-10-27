//! [`SuiteClient`]: the app side of the suite selection protocol over a [`SuiteWatch`], shared by
//! every app so that none re-implements it.

use std::sync::Arc;

use craft_theme::{SuiteTheme, ThemeId};

use crate::{Accepted, Op, Problem, SelectionChange, Snapshot, StoreError, SuiteChoice, SuiteWatch};

/// A change sent to the watch and not answered yet.
#[derive(Clone, Debug)]
struct Pending {
    id: u64,
    change: SelectionChange,
}

/// The app's view of the suite: the accepted state, the user's changes on their way, and the
/// notices to show.
///
/// Each frame the app calls [`sync`](Self::sync) (one atomic load unless something changed),
/// shows [`shown`](Self::shown) with its own defaults, and resolves with
/// [`theme`](Self::theme) as the lookup. User picks go to [`change`](Self::change).
///
/// At most one change is in flight. Picks made meanwhile merge into one queued change (a later
/// pick of a field replaces the earlier one), which goes out with the new accepted state as
/// base when the first result arrives, so quick picks never conflict with each other.
pub struct SuiteClient {
    watch: SuiteWatch,
    generation: Option<u64>,
    snapshot: Arc<Snapshot>,
    pending: Option<Pending>,
    queued: Option<SelectionChange>,
    conflict: Option<(Accepted, SelectionChange)>,
    failure: Option<(SelectionChange, StoreError)>,
    app_theme: Option<ThemeId>,
}

impl SuiteClient {
    pub fn new(watch: SuiteWatch) -> Self {
        let snapshot = watch.snapshot();
        SuiteClient { watch, generation: None, snapshot, pending: None, queued: None, conflict: None, failure: None, app_theme: None }
    }

    /// An in-memory suite, for app constructors, tests and web builds.
    pub fn memory() -> Self {
        Self::new(SuiteWatch::memory())
    }

    /// Takes a new snapshot when the watch has one, handles the answer to the change in flight
    /// and sends the queued one. Returns whether anything the app shows may have changed.
    pub fn sync(&mut self) -> bool {
        let generation = self.watch.generation();
        if self.generation == Some(generation) {
            return false;
        }
        self.generation = Some(generation);
        self.snapshot = self.watch.snapshot();
        self.settle();
        true
    }

    fn settle(&mut self) {
        let Some(pending) = &self.pending else {
            return;
        };
        let result = match self.snapshot.result(pending.id) {
            Some(result) => Some(result.clone()),
            // Not run yet, unless newer results have pushed it out of the snapshot.
            None if self.snapshot.results.first().is_some_and(|(id, _)| *id > pending.id) => None,
            None => return,
        };
        let Some(pending) = self.pending.take() else {
            return;
        };
        match result {
            Some(Err(StoreError::Conflict { disk, mine })) => self.conflict = Some((disk, mine)),
            Some(Err(error)) => self.failure = Some((pending.change, error)),
            Some(Ok(_)) | None => {}
        }
        if let Some(queued) = self.queued.take() {
            self.send(queued);
        }
    }

    /// The user changed the suite's theme or mode. Notices about earlier changes are cleared.
    pub fn change(&mut self, change: SelectionChange) {
        self.dismiss();
        self.submit(change);
    }

    fn submit(&mut self, change: SelectionChange) {
        if change.is_empty() {
            return;
        }
        if self.pending.is_none() {
            self.send(change);
            return;
        }
        self.queued = Some(match self.queued.take() {
            None => change,
            Some(queued) => SelectionChange { theme: change.theme.or(queued.theme), mode: change.mode.or(queued.mode) },
        });
    }

    fn send(&mut self, change: SelectionChange) {
        let base = self.snapshot.accepted.clone();
        let id = self.watch.request(Op::Update { base, change: change.clone() });
        self.pending = Some(Pending { id, change });
    }

    /// The suite choice to show: the accepted one with the changes on their way applied. A
    /// picked library theme the snapshot hasn't loaded yet shows only once it has, so the app
    /// never falls back to its own palette in between.
    pub fn shown(&self) -> SuiteChoice {
        let accepted = &self.snapshot.accepted.suite;
        let mut shown = accepted.clone();
        if let Some(pending) = &self.pending {
            shown = pending.change.applied_to(&shown);
        }
        if let Some(queued) = &self.queued {
            shown = queued.applied_to(&shown);
        }
        if shown.theme.as_ref().is_some_and(|id| self.is_loading(id)) {
            shown.theme = accepted.theme.clone();
        }
        shown
    }

    /// A built-in or a loaded library theme, for [`craft_theme::resolve`]'s `lookup`.
    pub fn theme(&self, id: &ThemeId) -> Option<SuiteTheme> {
        self.snapshot.theme(id)
    }

    /// True while `id` is a library theme the snapshot hasn't loaded yet: one just picked for the
    /// suite or named with [`set_app_theme`](Self::set_app_theme), with no problem reported for
    /// it. An app showing its own theme keeps its current look while this is true.
    pub fn is_loading(&self, id: &ThemeId) -> bool {
        !id.is_builtin() && self.snapshot.theme(id).is_none() && self.is_requested(id) && !self.has_problem(id)
    }

    fn is_requested(&self, id: &ThemeId) -> bool {
        let picked = |c: &SelectionChange| c.theme.as_ref() == Some(id);
        self.app_theme.as_ref() == Some(id) || self.pending.as_ref().is_some_and(|p| picked(&p.change)) || self.queued.as_ref().is_some_and(picked)
    }

    fn has_problem(&self, id: &ThemeId) -> bool {
        self.snapshot.problems.iter().any(|p| matches!(p, Problem::Theme { stem, .. } if stem == id.as_str()))
    }

    /// The library theme the app uses for its own choice, or `None` for the suite's or a
    /// built-in. Sent to the watch only when it changes.
    pub fn set_app_theme(&mut self, id: Option<ThemeId>) {
        let id = id.filter(|id| !id.is_builtin());
        if id != self.app_theme {
            self.app_theme = id.clone();
            self.watch.request(Op::AppTheme(id));
        }
    }

    /// The state the suite last confirmed.
    pub fn accepted(&self) -> &Accepted {
        &self.snapshot.accepted
    }

    /// True while a change is on its way.
    pub fn is_pending(&self) -> bool {
        self.pending.is_some() || self.queued.is_some()
    }

    /// The last change another app's choice blocked: the suite's state and the user's change.
    /// Show "Changed in another Craft app; your choice wasn't applied" with "Apply mine".
    pub fn conflict(&self) -> Option<&(Accepted, SelectionChange)> {
        self.conflict.as_ref()
    }

    /// The last change that failed, with the reason. Show it with "Retry".
    pub fn failure(&self) -> Option<&(SelectionChange, StoreError)> {
        self.failure.as_ref()
    }

    /// Sends the change from the last conflict again, on the state the suite has now.
    pub fn apply_mine(&mut self) {
        if let Some((_, mine)) = self.conflict.take() {
            self.submit(mine);
        }
    }

    /// Sends the last failed change again.
    pub fn retry(&mut self) {
        if let Some((change, _)) = self.failure.take() {
            self.submit(change);
        }
    }

    /// Clears the conflict and failure notices.
    pub fn dismiss(&mut self) {
        self.conflict = None;
        self.failure = None;
    }

    /// Files the suite can't use right now; each has a `Display` for the app's notice.
    pub fn problems(&self) -> &[Problem] {
        &self.snapshot.problems
    }

    pub fn snapshot(&self) -> &Arc<Snapshot> {
        &self.snapshot
    }

    /// For the other operations: import, delete, export, rescan.
    pub fn watch(&self) -> &SuiteWatch {
        &self.watch
    }
}

#[cfg(test)]
mod tests;
