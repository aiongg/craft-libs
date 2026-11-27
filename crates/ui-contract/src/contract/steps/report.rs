//! Step 16, report: the words of a value's validity. The access node itself is built once per run
//! by the driver (`contract/rt/run.rs`, `report`).

use crate::contract::Validity;
use crate::runtime::env::Message;

/// The message of a validity that warns, refuses or errs, and whether it warns.
pub(crate) fn validity_of<V>(validity: &Validity<V>) -> Option<(&Message, bool)> {
    match validity {
        Validity::Warn(message) => Some((message, true)),
        Validity::Refuse(message) | Validity::Error(message) => Some((message, false)),
        _ => None,
    }
}
