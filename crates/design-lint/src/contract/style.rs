//! `style/*`: looks come from style parts built in `fn style` (spec 4.2, 4.3, 4.5). Component code never picks a colour from
//! response state, matches on `State` or `ControlSize`, holds per-state fields, reads tokens, or takes a per-instance style.
//!
//! The rules read layers 5 to 8; `craft_ui::style` is layer 2 and craft-theme layer 0, so the resolver and the
//! caches are outside them.

use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::spanned::Spanned as _;
use syn::visit::{self, Visit};
use syn::{Attribute, ImplItem, Item, ItemImpl, ItemStruct, Type};

use crate::report::Finding;
use crate::rules::is_test_attr;

use super::Cx;
use super::sites::summarize;
use super::types::{derives, flat_items, self_name};

/// A colour chosen with an `if` on response state.
pub const STATE_PICK: &str = "style/state-pick";
/// A `match` with an arm on a `State` variant.
pub const MATCH_STATE: &str = "style/match-state";
/// A style struct with `States<_>` or per-state copy fields.
pub const STATES_FIELD: &str = "style/states-field";
/// A `match` with an arm on a `ControlSize` variant.
pub const SIZE_ARM: &str = "style/size-arm";
/// A token read outside a function that takes a `StyleCx`.
pub const TOKEN_READ: &str = "style/token-read";
/// A builder method that sets a style on one instance.
pub const INSTANCE_SETTER: &str = "style/instance-setter";
/// `.to_egui()` in a style builder.
pub const TO_EGUI: &str = "style/to-egui";

/// Response queries a hand-written colour pick branches on.
const RESPONSE_STATE: [&str; 9] =
    ["hovered", "is_hovered", "contains_pointer", "is_pointer_button_down_on", "has_focus", "highlighted", "clicked", "is_pressed", "dragged"];

const STATE_VARIANTS: [&str; 4] = ["Rest", "Hover", "Pressed", "Disabled"];

/// Methods that read tokens from an env or a `Ui`.
const TOKEN_METHODS: [&str; 3] = ["tokens", "ui_tokens", "token_group"];

/// Builder names that set a style or palette on one instance.
const INSTANCE_NAMES: [&str; 12] = [
    "style",
    "with_style",
    "style_override",
    "override_style",
    "look",
    "with_look",
    "colors",
    "with_colors",
    "palette",
    "with_palette",
    "skin",
    "with_skin",
];

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    if !matches!(cx.scope.layer(cx.path), Some(5..=8)) {
        return;
    }
    for item in flat_items(&cx.file.items) {
        match item {
            Item::Struct(s) => style_fields(cx, s, out),
            Item::Impl(imp) if imp.trait_.is_none() => instance_setters(cx, imp, out),
            _ => {}
        }
    }
    let mut walk = Walk { cx, out, in_style: false };
    walk.visit_file(cx.file);
}

fn is_style_type(s: &ItemStruct) -> bool {
    let name = s.ident.to_string();
    name.ends_with("Style") || name.ends_with("Look") || derives(&s.attrs).iter().any(|d| d == "ComponentStyle")
}

fn per_state_name(name: &str) -> bool {
    ["hover", "pressed", "disabled"].iter().any(|s| name.ends_with(&format!("_{s}")) || name.starts_with(&format!("{s}_")))
        || name.starts_with("selected_")
        || name.ends_with("_selected")
}

fn style_fields(cx: &Cx<'_>, s: &ItemStruct, out: &mut Vec<Finding>) {
    if !is_style_type(s) {
        return;
    }
    for f in &s.fields {
        if let Some(name) = f.ident.as_ref().map(ToString::to_string).filter(|n| per_state_name(n)) {
            cx.flag_span(
                out,
                STATES_FIELD,
                STATES_FIELD,
                f.span(),
                format!("`{}::{name}` is a per-state copy field; hold one `Stateful<_>`", s.ident),
            );
        }
        if names_type(&f.ty, "States") {
            cx.flag_span(
                out,
                STATES_FIELD,
                STATES_FIELD,
                f.ty.span(),
                format!("`{}` holds `States<_>`; the one per-state type is `Stateful<_>`", s.ident),
            );
        }
    }
}

fn names_type(ty: &Type, name: &str) -> bool {
    struct Find<'n>(&'n str, bool);
    impl<'ast> Visit<'ast> for Find<'_> {
        fn visit_path_segment(&mut self, s: &'ast syn::PathSegment) {
            self.1 |= s.ident == self.0;
            visit::visit_path_segment(self, s);
        }
    }
    let mut find = Find(name, false);
    find.visit_type(ty);
    find.1
}

fn instance_setters(cx: &Cx<'_>, imp: &ItemImpl, out: &mut Vec<Finding>) {
    let owner = self_name(imp).unwrap_or_default();
    for m in imp.items.iter().filter_map(|i| if let ImplItem::Fn(m) = i { Some(m) } else { None }) {
        let by_value = m.sig.receiver().is_some_and(|r| r.reference.is_none());
        let name = m.sig.ident.to_string();
        let takes_style = m.sig.inputs.iter().any(|a| matches!(a, syn::FnArg::Typed(t) if names_style_type(&t.ty)));
        if by_value && (INSTANCE_NAMES.contains(&name.as_str()) || takes_style) {
            cx.flag_span(
                out,
                INSTANCE_SETTER,
                INSTANCE_SETTER,
                m.sig.ident.span(),
                format!("`{owner}::{name}` sets a style on one instance; styles come from `fn style(cx, key)` and `Looks`"),
            );
        }
    }
}

fn names_style_type(ty: &Type) -> bool {
    let mut text = Vec::new();
    summarize(ty.to_token_stream(), &mut text);
    text.iter().any(|t| t.ends_with("Style") || t.ends_with("Look"))
}

struct Walk<'c, 'a> {
    cx: &'c Cx<'a>,
    out: &'c mut Vec<Finding>,
    in_style: bool,
}

fn is_style_fn(sig: &syn::Signature) -> bool {
    sig.ident == "style" || sig.inputs.iter().any(|a| matches!(a, syn::FnArg::Typed(t) if names_type(&t.ty, "StyleCx")))
}

fn skipped(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

impl Walk<'_, '_> {
    fn in_fn(&mut self, sig: &syn::Signature, walk: impl FnOnce(&mut Self)) {
        let before = std::mem::replace(&mut self.in_style, is_style_fn(sig));
        walk(self);
        self.in_style = before;
    }

    fn paths_in_arms(&mut self, e: &syn::ExprMatch) {
        struct Paths(Vec<Vec<String>>);
        impl<'ast> Visit<'ast> for Paths {
            fn visit_path(&mut self, p: &'ast syn::Path) {
                self.0.push(p.segments.iter().map(|s| s.ident.to_string()).collect());
            }
        }
        for arm in &e.arms {
            let mut paths = Paths(Vec::new());
            paths.visit_pat(&arm.pat);
            for p in paths.0 {
                let [.., owner, variant] = p.as_slice() else { continue };
                if owner == "State" && STATE_VARIANTS.contains(&variant.as_str()) {
                    self.cx.flag_span(
                        self.out,
                        MATCH_STATE,
                        MATCH_STATE,
                        arm.pat.span(),
                        format!("`State::{variant}` arm: read the look from `Stateful<_>`, not from a `match` on `State`"),
                    );
                } else if owner == "ControlSize" {
                    self.cx.flag_span(
                        self.out,
                        SIZE_ARM,
                        SIZE_ARM,
                        arm.pat.span(),
                        format!("`ControlSize::{variant}` arm: sizes come from a `BySize` table and `ControlMetrics`"),
                    );
                }
            }
        }
    }
}

fn idents(tokens: TokenStream) -> Vec<String> {
    let mut out = Vec::new();
    summarize(tokens, &mut out);
    out
}

fn is_colour_ident(i: &str) -> bool {
    let lower = i.to_ascii_lowercase();
    matches!(i, "Color32" | "Color" | "StyleColor" | "Rgba" | "DataColor")
        || lower.contains("color")
        || lower.contains("colour")
        || lower.contains("fill")
}

impl<'ast> Visit<'ast> for Walk<'_, '_> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if !skipped(&i.attrs) {
            self.in_fn(&i.sig, |w| visit::visit_item_fn(w, i));
        }
    }
    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        if !skipped(&i.attrs) {
            self.in_fn(&i.sig, |w| visit::visit_impl_item_fn(w, i));
        }
    }
    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        if !skipped(&i.attrs) {
            visit::visit_item_mod(self, i);
        }
    }
    fn visit_expr_if(&mut self, e: &'ast syn::ExprIf) {
        let on_response = idents(e.cond.to_token_stream()).iter().any(|i| RESPONSE_STATE.contains(&i.as_str()));
        let mut branches = e.then_branch.to_token_stream();
        if let Some((_, other)) = &e.else_branch {
            branches.extend(other.to_token_stream());
        }
        if on_response && idents(branches).iter().any(|i| is_colour_ident(i)) {
            self.cx.flag_span(
                self.out,
                STATE_PICK,
                STATE_PICK,
                e.if_token.span,
                "a colour picked from response state; the style's `Stateful<_>` and `cx.resolve` do this".to_owned(),
            );
        }
        visit::visit_expr_if(self, e);
    }
    fn visit_expr_match(&mut self, e: &'ast syn::ExprMatch) {
        self.paths_in_arms(e);
        visit::visit_expr_match(self, e);
    }
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        let name = e.method.to_string();
        if name == "to_egui" && self.in_style {
            self.cx.flag_span(
                self.out,
                TO_EGUI,
                TO_EGUI,
                e.method.span(),
                "`.to_egui()` in a style builder; `cx.color(..)` makes the `StyleColor`".to_owned(),
            );
        } else if TOKEN_METHODS.contains(&name.as_str()) && !self.in_style {
            self.cx.flag_span(
                self.out,
                TOKEN_READ,
                TOKEN_READ,
                e.method.span(),
                format!("`.{name}(..)` reads tokens outside `fn style`; read them in the style and keep the result in the `Style`"),
            );
        }
        visit::visit_expr_method_call(self, e);
    }
    fn visit_expr_path(&mut self, e: &'ast syn::ExprPath) {
        let names: Vec<String> = e.path.segments.iter().map(|s| s.ident.to_string()).collect();
        let reads = names.windows(2).any(|w| matches!(w[0].as_str(), "Tokens" | "UiTokens") && w[1] == "of");
        if reads && !self.in_style {
            self.cx.flag_span(self.out, TOKEN_READ, TOKEN_READ, e.span(), format!("`{}` reads tokens outside `fn style`", names.join("::")));
        }
        visit::visit_expr_path(self, e);
    }
}
