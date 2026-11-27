//! `app/no-egui`: a use of the `egui` crate in a crate that depends on `craft-ui-contract`.

use crate::report::Finding;

use super::Cx;
use super::sites::Kind;

pub const ID: &str = "app/no-egui";

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    if !cx.scope.contract_client(cx.path) {
        return;
    }
    for site in &cx.sites {
        if let Kind::Path { path, .. } = &site.kind
            && path.first().is_some_and(|f| f == "egui")
        {
            cx.flag(out, ID, ID, site, format!("`{}` names the egui crate; use the `craft_ui` equivalent", path.join("::")));
        }
    }
}
