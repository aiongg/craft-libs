//! `StateSet`: what a style resolves against.

use craft_theme::{Flags, Mix, Resolve, State, StateMix, Stateful};

/// The state of a component or one of its parts at step 12: the interaction state, the value flags and the
/// animated blend. `StateSet.flags` is the only source of AccessKit states.
#[derive(Clone, Copy, Debug, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub struct StateSet {
    /// Disabled, pressed, hover or rest; `Flags::OPEN` counts as pressed.
    pub state: State,
    /// The merged value flags, read through [`StateSet::flags`] or [`StateSet::has`] so that a look branching on
    /// `FOCUSED` or `FOCUS_VISIBLE` notes the focus read.
    flags: Flags,
    /// The animated hover, press and on values.
    pub t: StateMix,
}

impl StateSet {
    /// A state set.
    pub const fn new(state: State, flags: Flags, t: StateMix) -> Self {
        StateSet { state, flags, t }
    }

    /// The merged value flags. Any of them may be a focus flag, so the read notes a read of the focus facts by the
    /// look being recorded (contract-rt 15.6 rule 4): a focus move reruns the node. [`StateSet::has`] reads one flag.
    pub fn flags(&self) -> Flags {
        note_focus_read();
        self.flags
    }

    /// Whether every flag in `flag` is set. Only a read of `FOCUSED` or `FOCUS_VISIBLE` notes the focus read.
    pub fn has(&self, flag: Flags) -> bool {
        if flag.intersects(Flags::FOCUSED | Flags::FOCUS_VISIBLE) {
            note_focus_read();
        }
        self.flags.contains(flag)
    }

    /// The flags without a read, for the framework code that builds a state set from another or reads no focus flag.
    pub(crate) fn flags_unread(&self) -> Flags {
        self.flags
    }

    /// The component is disabled.
    pub fn is_disabled(&self) -> bool {
        self.state == State::Disabled
    }

    /// The value of `look` in this state. While a paint look is recorded, resolving reads the
    /// pointer facts the look varies by in this state (contract-rt 15.6 rule 1): hover only when its
    /// hover look differs from its rest look, press only when its pressed look differs, none when it
    /// is disabled or a flag look wins. The run then records the hover look, and the press look for a
    /// node that takes presses, only for a look that changes with them.
    pub fn resolve<T: Mix>(&self, look: &Stateful<T>) -> T {
        let reads = look.pointer_dependence(self.state, self.flags);
        if reads.hover || reads.press {
            note_pointer_read(reads.hover, reads.press);
        }
        if look.flag_dependence(self.state, Flags::FOCUSED | Flags::FOCUS_VISIBLE) {
            note_focus_read();
        }
        look.resolve(self.state, self.flags, self.t)
    }
}

impl Default for StateSet {
    fn default() -> Self {
        StateSet::new(State::Rest, Flags::empty(), StateMix::REST)
    }
}

/// The interaction facts a paint look's recording read: `Hovered`, `Pressed`, and the focus facts a look with a
/// focus entry varies by (`Focused`, `FocusVisible`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PointerReads {
    pub hover: bool,
    pub press: bool,
    pub focus: bool,
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

/// Notes a read of the focus facts by the look being recorded. Outside a recording it does nothing.
fn note_focus_read() {
    POINTER_READS.with(|reads| {
        if let Some(mut read) = reads.get() {
            read.focus = true;
            reads.set(Some(read));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_theme::States;

    /// A look with no hover or press entry records no pointer read, so its node records no hover or
    /// press variant; a look with a hover entry records the hover read alone.
    #[test]
    fn resolve_reads_only_the_pointer_facts_the_look_varies_by() {
        let set = StateSet::new(State::Rest, Flags::empty(), StateMix::REST);
        let flat = Stateful::new(1.0_f32);
        let (_, read) = pointer_reads(|| set.resolve(&flat));
        assert_eq!(read, PointerReads::default());

        let hover = Stateful::new(1.0_f32).off(States::new(1.0, 0.5, 0.5, 0.2));
        let (_, read) = pointer_reads(|| set.resolve(&hover));
        assert!(read.hover);

        let disabled = StateSet::new(State::Disabled, Flags::empty(), StateMix::REST);
        let (_, read) = pointer_reads(|| disabled.resolve(&hover));
        assert_eq!(read, PointerReads::default());
    }
}
