//! `contract/shadow`, `contract/widget`, `contract/overlay-flag`: what the derive writes is not written by hand (spec 1.2).

use syn::{ImplItem, Item};

use crate::report::Finding;

use super::Cx;
use super::sites::{Kind, is_egui};
use super::types::{flat_items, self_name, trait_name};

/// An inherent show function on a component.
pub const SHADOW: &str = "contract/shadow";
/// A hand-written `impl egui::Widget`.
pub const WIDGET: &str = "contract/widget";
/// An overridden `overlay` on a type without `#[component(overlay)]`.
pub const OVERLAY_FLAG: &str = "contract/overlay-flag";

/// The names of the sealed show traits' functions; an inherent function of one of them hides the trait's.
const SHOW_NAMES: [&str; 5] = ["show", "show_at", "show_in", "show_with_tip", "show_at_with_tip"];

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    let widget_names = egui_widget_names(cx);
    for item in flat_items(&cx.file.items) {
        let Item::Impl(imp) = item else { continue };
        let Some(owner) = self_name(imp) else { continue };
        let fns = imp.items.iter().filter_map(|i| if let ImplItem::Fn(m) = i { Some(m) } else { None });
        match trait_name(imp).as_deref() {
            None if cx.types.get(&owner).is_some_and(|f| f.component) => {
                for m in fns.filter(|m| SHOW_NAMES.contains(&m.sig.ident.to_string().as_str())) {
                    cx.flag_span(
                        out,
                        SHADOW,
                        SHADOW,
                        m.sig.ident.span(),
                        format!("`{owner}::{}` hides the library's `show`; the pipeline writes it", m.sig.ident),
                    );
                }
            }
            Some("Widget") if is_egui_widget(imp, &widget_names) => {
                cx.flag_span(out, WIDGET, WIDGET, imp.impl_token.span, format!("`impl egui::Widget for {owner}`; `#[component(widget)]` writes it"));
            }
            Some("Component") => {
                let flagged = cx.types.get(&owner).is_some_and(|f| f.defined && !f.overlay_attr);
                if let Some(m) = fns.into_iter().find(|m| m.sig.ident == "overlay").filter(|_| flagged) {
                    cx.flag_span(
                        out,
                        OVERLAY_FLAG,
                        OVERLAY_FLAG,
                        m.sig.ident.span(),
                        format!("`{owner}` overrides `overlay` without `#[component(overlay)]`"),
                    );
                }
            }
            _ => {}
        }
    }
}

/// The names under which the file imports `egui::..::Widget`, and whether it globs egui.
struct WidgetNames {
    named: bool,
    glob: bool,
}

fn egui_widget_names(cx: &Cx<'_>) -> WidgetNames {
    let mut names = WidgetNames { named: false, glob: false };
    for site in &cx.sites {
        if let Kind::Path { path, in_use: true, .. } = &site.kind
            && is_egui(path)
        {
            names.named |= path.last().is_some_and(|l| l == "Widget");
            names.glob |= path.last().is_some_and(|l| l == "egui" || l == "widgets" || l == "prelude");
        }
    }
    names
}

fn is_egui_widget(imp: &syn::ItemImpl, names: &WidgetNames) -> bool {
    let Some((_, path, _)) = &imp.trait_ else { return false };
    let first = path.segments.first().map(|s| s.ident.to_string()).unwrap_or_default();
    if path.segments.len() > 1 {
        return super::sites::EGUI_ROOTS.contains(&first.as_str());
    }
    names.named || names.glob
}
