//! `contract/egui-widget`: egui's widget constructors and `Ui` shortcuts outside `runtime::egui`.

use crate::report::Finding;

use super::Cx;
use super::sites::Kind;

pub const ID: &str = "contract/egui-widget";

const SHORTCUTS: [&str; 12] = [
    "button",
    "small_button",
    "checkbox",
    "selectable_label",
    "selectable_value",
    "radio",
    "radio_value",
    "hyperlink",
    "hyperlink_to",
    "link",
    "separator",
    "image",
];

const WIDGETS: [&str; 5] = ["Button", "TextEdit", "DragValue", "Slider", "ComboBox"];

/// An argument that reads a design token: it mentions `tokens`, `Tokens` or `token`.
fn is_token(idents: &[String]) -> bool {
    idents.iter().any(|i| matches!(i.as_str(), "tokens" | "Tokens" | "token"))
}

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    cx.flag_methods(out, ID, ID, "is an egui widget shortcut; the contract's parts draw widgets", |name, _| SHORTCUTS.contains(&name));
    // Literal arguments are `literal-size/add-space`.
    cx.flag_methods(out, ID, ID, "takes a space that is not a token", |name, args| {
        name == "add_space" && !args.has_literal && !is_token(&args.idents)
    });
    for site in &cx.sites {
        if let Kind::Path { call: true, .. } = &site.kind
            && let Some(path) = site.egui_path()
            && path.iter().any(|s| WIDGETS.contains(&s.as_str()))
        {
            cx.flag(out, ID, ID, site, format!("`{}` constructs an egui widget; the contract's parts draw widgets", path.join("::")));
        }
    }
}
