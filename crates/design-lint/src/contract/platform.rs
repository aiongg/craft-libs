//! `contract/platform`: target checks, clipboard writes and URL opening in UI modules. URL opening is matched by the
//! egui path (`ctx.open_url`, `egui::OpenUrl`), so a component method named `open_url` is not flagged.

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
    cx.flag_methods(out, ID, ID, "writes the clipboard through egui; use `cx.clipboard()`", |name, _| name == "copy_text");
    for site in &cx.sites {
        if let Kind::Method { name, receiver, .. } = &site.kind
            && name == "open_url"
            && receiver.last().is_some_and(|r| r == "context" || r.ends_with("ctx"))
        {
            cx.flag(out, ID, ID, site, format!("`{}.open_url` opens a URL through egui; use `Cx::open_url`", receiver.join(".")));
        }
    }
    cx.flag_paths(out, ID, ID, "opens a URL through egui; use `Cx::open_url`", |path| path.iter().any(|segment| segment == "OpenUrl"));
}
