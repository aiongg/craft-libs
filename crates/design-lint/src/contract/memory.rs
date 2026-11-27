//! `contract/memory`: egui memory and id derivation in library and app code.

use crate::report::Finding;

use super::Cx;
use super::sites::Kind;

pub const ID: &str = "contract/memory";

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    let why = "reads or writes egui memory or derives an id by hand; use `Remember` and `craft_ui::Id`";
    cx.flag_methods(out, ID, ID, why, |name, args| match name {
        "get_temp" | "insert_temp" | "remove_temp" | "remove_temp_raw" | "get_persisted" | "insert_persisted" | "data_mut" => true,
        "make_persistent_id" | "next_auto_id" | "push_id" => true,
        "data" | "scope" => args.has_closure,
        "scope_builder" => args.idents.iter().any(|i| i == "id_salt"),
        _ => false,
    });
    for site in &cx.sites {
        match &site.kind {
            Kind::Method { name, receiver, .. } if name == "id_salt" && receiver.iter().any(|i| i == "UiBuilder") => {
                cx.flag(out, ID, ID, site, format!("{} sets an egui id salt; use `Ui::push_id` or `UiBuilder::id_salt`", super::call(name)));
            }
            _ => {}
        }
    }
    cx.flag_paths(out, ID, ID, "mints an egui id; use `craft_ui::Id`", |p| p.ends_with(&["Id".to_owned(), "unique".to_owned()]));
}
