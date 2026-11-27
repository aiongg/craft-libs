//! `StateSet`: what a style resolves against.

use craft_theme::{Flags, Mix, Resolve, State, StateMix, Stateful};

/// The state of a component or one of its parts at step 12: the interaction state, the value flags and the
/// animated blend. `StateSet.flags` is the only source of AccessKit states.
#[derive(Clone, Copy, Debug, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub struct StateSet {
    /// Disabled, pressed, hover or rest; `Flags::OPEN` counts as pressed.
    pub state: State,
    /// The merged value flags.
    pub flags: Flags,
    /// The animated hover, press and on values.
    pub t: StateMix,
}

impl StateSet {
    /// A state set.
    pub const fn new(state: State, flags: Flags, t: StateMix) -> Self {
        StateSet { state, flags, t }
    }

    /// The component is disabled.
    pub fn is_disabled(&self) -> bool {
        self.state == State::Disabled
    }

    /// The value of `look` in this state. While a paint look is recorded, resolving an enabled state
    /// reads the pointer: the run then records the hover look, and the press look for a node that
    /// takes presses (contract-rt 15.6).
    pub fn resolve<T: Mix>(&self, look: &Stateful<T>) -> T {
        if self.state != State::Disabled {
            note_pointer_read(true, true);
        }
        look.resolve(self.state, self.flags, self.t)
    }
}

impl Default for StateSet {
    fn default() -> Self {
        StateSet::new(State::Rest, Flags::empty(), StateMix::REST)
    }
}

/// The pointer facts a paint look's recording read (`Hovered`, `Pressed`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PointerReads {
    pub hover: bool,
    pub press: bool,
}

thread_local! {
    /// The reads of the look being recorded on this thread; `None` outside a recording.
    static POINTER_READS: std::cell::Cell<Option<PointerReads>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` (one look's state and paint) and returns the pointer facts it read.
pub(crate) fn pointer_reads<R>(f: impl FnOnce() -> R) -> (R, PointerReads) {
    let outer = POINTER_READS.with(|reads| reads.replace(Some(PointerReads::default())));
    let out = f();
    let read = POINTER_READS.with(|reads| reads.replace(outer)).unwrap_or_default();
    (out, read)
}

/// Notes a read of the hover and press facts by the look being recorded (`Styled::state`, a face's
/// part state). Outside a recording it does nothing.
pub(crate) fn note_pointer_read(hover: bool, press: bool) {
    POINTER_READS.with(|reads| {
        if let Some(mut read) = reads.get() {
            read.hover |= hover;
            read.press |= press;
            reads.set(Some(read));
        }
    });
}
