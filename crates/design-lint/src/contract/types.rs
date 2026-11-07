//! What one file says about its types and impls, shared by the rules that read item shapes.
//!
//! A type is a component when the file derives or attributes it as one, or implements `Component` for it.
//! Types defined in another file are not known; rules that need a type's definition skip them.

use std::collections::HashMap;

use syn::{Attribute, Item, ItemImpl, Visibility};

use crate::rules::is_test_attr;

/// Facts about one type name in the file.
#[derive(Debug, Default)]
pub(crate) struct TypeFacts {
    /// The file defines the type.
    pub defined: bool,
    /// Declared `pub` without a restriction.
    pub public: bool,
    /// Derived, attributed or implemented as a `Component`.
    pub component: bool,
    /// Derives `Props` or has a `#[prop]` field.
    pub props: bool,
    /// Implements or derives the `Pattern` marker.
    pub pattern: bool,
    /// Carries `#[component(.., overlay, ..)]`.
    pub overlay_attr: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Types {
    by_name: HashMap<String, TypeFacts>,
}

impl Types {
    pub fn get(&self, name: &str) -> Option<&TypeFacts> {
        self.by_name.get(name)
    }
}

/// Every item of the file, inline modules included; test code is left out.
pub(crate) fn flat_items(items: &[Item]) -> Vec<&Item> {
    let mut out = Vec::new();
    walk(items, &mut out);
    out
}

fn walk<'a>(items: &'a [Item], out: &mut Vec<&'a Item>) {
    for item in items {
        if attrs_of(item).iter().any(is_test_attr) {
            continue;
        }
        out.push(item);
        if let Item::Mod(m) = item
            && let Some((_, inner)) = &m.content
        {
            walk(inner, out);
        }
    }
}

fn attrs_of(item: &Item) -> &[Attribute] {
    match item {
        Item::Struct(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        _ => &[],
    }
}

/// The last identifier of a `derive(..)` list entry, for every derive attribute in `attrs`.
pub(crate) fn derives(attrs: &[Attribute]) -> Vec<String> {
    let mut names = Vec::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("derive")) {
        let _ = attr.parse_nested_meta(|meta| {
            if let Some(last) = meta.path.segments.last() {
                names.push(last.ident.to_string());
            }
            Ok(())
        });
    }
    names
}

/// Whether the attribute is `name(..)` with `word` among its tokens.
pub(crate) fn attr_names(attr: &Attribute, name: &str, word: &str) -> bool {
    attr.path().is_ident(name) && matches!(&attr.meta, syn::Meta::List(l) if tokens_contain(l.tokens.clone(), word))
}

fn tokens_contain(tokens: proc_macro2::TokenStream, word: &str) -> bool {
    tokens.into_iter().any(|t| match t {
        proc_macro2::TokenTree::Ident(i) => i == word,
        proc_macro2::TokenTree::Group(g) => tokens_contain(g.stream(), word),
        _ => false,
    })
}

/// The last identifier of an impl's self type.
pub(crate) fn self_name(imp: &ItemImpl) -> Option<String> {
    match &*imp.self_ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// The last identifier of an impl's trait path.
pub(crate) fn trait_name(imp: &ItemImpl) -> Option<String> {
    imp.trait_.as_ref().and_then(|(_, path, _)| path.segments.last()).map(|s| s.ident.to_string())
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

pub(crate) fn collect(file: &syn::File) -> Types {
    let mut types = Types::default();
    let items = flat_items(&file.items);
    for item in &items {
        let (ident, attrs, vis) = match item {
            Item::Struct(s) => (&s.ident, &s.attrs, &s.vis),
            Item::Enum(e) => (&e.ident, &e.attrs, &e.vis),
            _ => continue,
        };
        let derived = derives(attrs);
        let facts = types.by_name.entry(ident.to_string()).or_default();
        facts.defined = true;
        facts.public = is_public(vis);
        facts.component |= derived.iter().any(|d| d == "Component") || attrs.iter().any(|a| a.path().is_ident("component"));
        facts.props |= derived.iter().any(|d| d == "Props")
            || match item {
                Item::Struct(s) => s.fields.iter().any(|f| f.attrs.iter().any(|a| a.path().is_ident("prop"))),
                _ => false,
            };
        facts.pattern |= derived.iter().any(|d| d == "Pattern");
        facts.overlay_attr |= attrs.iter().any(|a| attr_names(a, "component", "overlay"));
    }
    for item in &items {
        if let Item::Impl(imp) = item
            && let (Some(name), Some(tr)) = (self_name(imp), trait_name(imp))
        {
            let facts = types.by_name.entry(name).or_default();
            facts.component |= tr == "Component";
            facts.pattern |= tr == "Pattern";
        }
    }
    types
}
