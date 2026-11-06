//! `contract/platform`: target checks, clipboard writes and URL opening in UI modules.

use crate::report::Finding;

use super::Cx;
use super::sites::{Kind, names_platform};

pub const ID: &str = "contract/platform";

/// Permission to check the target: `craft-host::target` and `craft-platform`.
const TARGET: &str = "contract/platform:target";

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    for site in &cx.sites {
        match &site.kind {
            Kind::Cfg { idents } if names_platform(idents) => {
                cx.flag(out, ID, TARGET, site, "cfg on the target OS or wasm; ask `craft-host::target`".to_owned());
            }
            Kind::Macro { name, idents } if name == "cfg" && names_platform(idents) => {
                cx.flag(out, ID, TARGET, site, "cfg! on the target OS or wasm; ask `craft-host::target`".to_owned());
            }
            _ => {}
        }
    }
    cx.flag_methods(
        out,
        ID,
        ID,
        "writes the clipboard or opens a URL through egui; use `cx.clipboard()` or the runtime's output commands",
        |name, _| matches!(name, "copy_text" | "open_url"),
    );
}
