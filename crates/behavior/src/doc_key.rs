//! A stable, crate-prefixed semantic key for presence, automation, undo names and programmatic focus.

/// A document key such as `"craft-ui.find-field"`.
// F2-MERGE: props (`DocKey` is a contract base type no F2 track owns).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DocKey(&'static str);

impl DocKey {
    /// A key named `name`.
    pub const fn new(name: &'static str) -> DocKey {
        DocKey(name)
    }

    /// The name the key was declared with.
    pub const fn name(self) -> &'static str {
        self.0
    }
}
