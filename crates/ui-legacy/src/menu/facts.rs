//! The id a panel keeps its panel-menu facts under.

use egui::{Id, IdSalt};

/// The id under which the panel named by `id_salt` keeps what its panel menu needs from its last
/// pass (enabled rows, a pending command). Panel menus run with a `Context` only, so the id is
/// global rather than scoped to a `Ui`: the salt must be unique to the panel in the app.
///
/// A public entry point that takes the raw salt (`X::panel_menu(ctx, id_salt, ..)`,
/// `X::run_command`) passes `IdSalt::new(id_salt)`; the panel's `show` passes the `IdSalt` it
/// stored. Both give the same id because the salt goes to [`Id::with_salt`], which doesn't hash it
/// again.
pub fn facts_id(id_salt: IdSalt) -> Id {
    Id::NULL.with_salt(id_salt)
}
