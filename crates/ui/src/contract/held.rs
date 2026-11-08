//! `Held`: a caller's binding read and written from inside a container body.

use super::{Bind, BodyCx};
use craft_behavior::{EditEvent, GestureId, Mixed, Value};
use std::sync::Arc;

/// The caller's binding of a value a pattern edits in its body: the body reads the value at the start of its pass and
/// writes the edited one back as a single set edit.
pub struct Held<'a, T>(Bind<'a, T>);

impl<'a, T: Value> Held<'a, T> {
    /// Holds `bind`.
    pub fn new(bind: Bind<'a, T>) -> Self {
        Held(bind)
    }

    /// The bound value; `None` while a mixed or kept binding holds none.
    pub fn read(&mut self, cx: &mut BodyCx<'_>) -> Option<T> {
        match self.0.reborrow().read(cx) {
            Some(Mixed::Same(value)) => Some(value),
            _ => None,
        }
    }

    /// Replaces `before` with `after`, the edit named `name`.
    pub fn write(&mut self, cx: &mut BodyCx<'_>, before: &T, after: T, name: &str) {
        let event = EditEvent::set(GestureId::default(), Mixed::Same(before.clone()), after, Arc::from(name), None);
        let _ = self.0.reborrow().write(cx, &event);
    }
}
