//! `craft_dialogs::save`, with a scripted `Files` and an inline `FileWatch`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Save of a document with a file writes in place with no picker; Save As and a new document open
// the picker in the document's folder with name and extension suggested, and the answer becomes
// Write with the picked path.
#[test]
#[ignore = "skeleton"]
fn save_writes_in_place_and_save_as_asks_where() {
    todo!()
}

// After another writer changed the document's own file, Save asks; Replace writes, Save As opens
// the picker, Cancel cancels; a file the app wrote itself (written) never asks.
#[test]
#[ignore = "skeleton"]
fn a_file_changed_on_disk_is_asked_about_once() {
    todo!()
}

// Saving a mail attachment's temporary copy asks where to keep it; the Outlook and Apple Mail
// paths match on either separator and in any case, an ordinary folder named Content.Outlook
// doesn't.
#[test]
#[ignore = "skeleton"]
fn mail_temporary_copies_are_saved_elsewhere() {
    todo!()
}

// A second start while a picker, question or write is pending answers Busy; another feature's
// answer stays with the file service; cancel forgets the flow's ticket, so its late answer is
// never taken.
#[test]
#[ignore = "skeleton"]
fn one_save_at_a_time_and_cancel_forgets_the_ticket() {
    todo!()
}
