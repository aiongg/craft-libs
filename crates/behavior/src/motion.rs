//! The motion level an animation runs at.

/// How much motion the interface shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MotionMode {
    /// Every animation runs.
    #[default]
    Full,
    /// Travel and loops are replaced by short fades.
    Reduced,
    /// Nothing animates.
    Off,
}

impl MotionMode {
    /// The lower of two modes: a scope can lower the motion mode, never raise it.
    pub const fn lowered_by(self, scope: MotionMode) -> MotionMode {
        if scope.level() > self.level() { scope } else { self }
    }

    const fn level(self) -> u8 {
        match self {
            MotionMode::Full => 0,
            MotionMode::Reduced => 1,
            MotionMode::Off => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MotionMode::*;

    #[test]
    fn a_scope_lowers_the_mode_and_never_raises_it() {
        assert_eq!(Full.lowered_by(Reduced), Reduced);
        assert_eq!(Off.lowered_by(Full), Off);
        assert_eq!(Reduced.lowered_by(Off), Off);
    }
}
