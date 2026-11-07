//! `props/*`: the `Props` setters come from `#[derive(Props)]` and the `Props` trait, never from hand-written code (spec 3.2).

use proc_macro2::TokenTree;
use syn::spanned::Spanned as _;
use syn::{Expr, ImplItem, ImplItemFn, Item, ItemImpl, ItemStruct, Stmt, Type};

use crate::report::Finding;

use super::Cx;
use super::types::{derives, flat_items, self_name};

/// An inherent builder method named like a `Props` setter.
pub const RESERVED: &str = "props/reserved-setter";
/// An inherent builder method that only stores its argument in one field.
pub const HAND_SETTER: &str = "props/hand-setter";
/// A component field that `Common` already holds.
pub const OWN_FIELD: &str = "props/own-field";
/// A `#[prop]` field whose generated setter shadows a `Props` setter.
pub const SHADOW: &str = "props/shadow";
/// A `#[prop]` field of type `f32` or `Vec2`.
pub const LITERAL_SIZE: &str = "props/literal-size";

/// The `Props` setters and the `Common` fields they write.
const SETTERS: [&str; 13] =
    ["id_salt", "id", "label", "enabled", "read_only", "size", "variant", "tooltip", "description", "selected", "doc_key", "command", "keeps_name"];

/// Fields a component takes from `Common`.
const COMMON_FIELDS: [&str; 3] = ["enabled", "id_salt", "tooltip"];

/// Identifiers a value may mention and still be a plain store: conversions and wrappers.
const WRAPPERS: [&str; 10] = ["Some", "into", "to_owned", "to_string", "Into", "from", "Box", "Arc", "new", "String"];

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    let component_layer = matches!(cx.scope.layer(cx.path), Some(5..=8));
    for item in flat_items(&cx.file.items) {
        match item {
            Item::Impl(imp) if imp.trait_.is_none() && component_layer && builds_props(cx, imp) => setters(cx, imp, out),
            Item::Struct(s) => fields(cx, s, out),
            _ => {}
        }
    }
}

/// Whether the impl is of a type known to be a component or a `Props` type.
pub(super) fn builds_props(cx: &Cx<'_>, imp: &ItemImpl) -> bool {
    self_name(imp).is_some_and(|name| cx.types.get(&name).is_some_and(|f| f.component || f.props))
}

fn setters(cx: &Cx<'_>, imp: &ItemImpl, out: &mut Vec<Finding>) {
    let owner = self_name(imp).unwrap_or_default();
    for m in imp.items.iter().filter_map(|i| if let ImplItem::Fn(m) = i { Some(m) } else { None }) {
        if !is_builder(m, &owner) {
            continue;
        }
        let name = m.sig.ident.to_string();
        if SETTERS.contains(&name.as_str()) {
            cx.flag_span(out, RESERVED, RESERVED, m.sig.ident.span(), format!("`{owner}::{name}` is a `Props` setter name; `Props` provides it"));
        }
        if only_stores(m) {
            cx.flag_span(
                out,
                HAND_SETTER,
                HAND_SETTER,
                m.sig.ident.span(),
                format!("`{owner}::{name}` only stores its argument; mark the field `#[prop]` and let `#[derive(Props)]` write the setter"),
            );
        }
    }
}

/// `fn name(self, ..) -> Self` (or the owner's name): a by-value receiver and a returned owner.
fn is_builder(m: &ImplItemFn, owner: &str) -> bool {
    let by_value = m.sig.receiver().is_some_and(|r| r.reference.is_none() && r.colon_token.is_none());
    let returns_owner = match &m.sig.output {
        syn::ReturnType::Type(_, ty) => {
            matches!(&**ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Self" || s.ident == owner))
        }
        syn::ReturnType::Default => false,
    };
    by_value && returns_owner
}

/// The body stores the one parameter, possibly converted or wrapped, in one field of `self`: `self.field = value; self` or
/// `Self { field: value, ..self }`.
fn only_stores(m: &ImplItemFn) -> bool {
    let mut params = m.sig.inputs.iter().filter_map(|a| if let syn::FnArg::Typed(t) = a { Some(t) } else { None });
    let (Some(param), None) = (params.next(), params.next()) else { return false };
    let syn::Pat::Ident(param) = &*param.pat else { return false };
    let param = param.ident.to_string();
    match m.block.stmts.as_slice() {
        [Stmt::Expr(Expr::Assign(assign), Some(_)), Stmt::Expr(Expr::Path(tail), None)] => {
            tail.path.is_ident("self") && is_self_field(&assign.left) && mentions_only(&assign.right, &param)
        }
        [Stmt::Expr(Expr::Struct(update), None)] => {
            let rest_is_self = matches!(update.rest.as_deref(), Some(Expr::Path(p)) if p.path.is_ident("self"));
            rest_is_self && update.qself.is_none() && update.fields.len() == 1 && mentions_only(&update.fields[0].expr, &param)
        }
        _ => false,
    }
}

/// Exactly `self.<field>`: the derive cannot write a nested path.
fn is_self_field(e: &Expr) -> bool {
    matches!(e, Expr::Field(f) if matches!(&*f.base, Expr::Path(p) if p.path.is_ident("self")))
}

fn mentions_only(e: &Expr, param: &str) -> bool {
    let mut idents = Vec::new();
    super::sites::summarize(quote::ToTokens::to_token_stream(e), &mut idents);
    idents.iter().all(|i| i == param || WRAPPERS.contains(&i.as_str()))
}

fn fields(cx: &Cx<'_>, s: &ItemStruct, out: &mut Vec<Finding>) {
    let named = |f: &syn::Field| f.ident.as_ref().map(ToString::to_string);
    let derived = derives(&s.attrs);
    let component =
        derived.iter().any(|d| d == "Component" || d == "Props") || s.fields.iter().any(|f| f.attrs.iter().any(|a| a.path().is_ident("common")));
    for f in &s.fields {
        let Some(field) = named(f) else { continue };
        if component && COMMON_FIELDS.contains(&field.as_str()) {
            cx.flag_span(out, OWN_FIELD, OWN_FIELD, f.span(), format!("`{}::{field}` duplicates the `Common` field of that name", s.ident));
        }
        let Some(prop) = f.attrs.iter().find(|a| a.path().is_ident("prop")) else { continue };
        let setter = setter_name(prop).unwrap_or_else(|| field.clone());
        if SETTERS.contains(&setter.as_str()) {
            cx.flag_span(out, SHADOW, SHADOW, f.span(), format!("the setter `{}::{setter}` shadows the `Props` setter of that name", s.ident));
        }
        if is_literal_size(&f.ty) {
            cx.flag_span(
                out,
                LITERAL_SIZE,
                LITERAL_SIZE,
                f.ty.span(),
                format!("`{}::{field}` is a size in points; sizes come from `ControlSize`, `ColumnWidth` or a binding", s.ident),
            );
        }
    }
}

/// The `name = ".."` of a `#[prop(..)]` attribute.
fn setter_name(attr: &syn::Attribute) -> Option<String> {
    let syn::Meta::List(list) = &attr.meta else { return None };
    let tokens: Vec<TokenTree> = list.tokens.clone().into_iter().collect();
    tokens.windows(3).find_map(|w| match w {
        [TokenTree::Ident(name), TokenTree::Punct(eq), TokenTree::Literal(lit)] if name == "name" && eq.as_char() == '=' => {
            Some(lit.to_string().trim_matches('"').to_owned())
        }
        _ => None,
    })
}

/// `f32`, `Vec2` or an `Option` of either.
fn is_literal_size(ty: &Type) -> bool {
    let Type::Path(p) = ty else { return false };
    let Some(last) = p.path.segments.last() else { return false };
    match last.ident.to_string().as_str() {
        "f32" | "Vec2" => true,
        "Option" => {
            matches!(&last.arguments, syn::PathArguments::AngleBracketed(a) if a.args.iter().any(|g| matches!(g, syn::GenericArgument::Type(t) if is_literal_size(t))))
        }
        _ => false,
    }
}
