//! `#[derive(Props)]`: one setter per `#[prop]` field, and the forwarding setters of a `#[collection]` or
//! `#[container]` field.

use crate::util::{is_named, named_fields, wrapped};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Field, GenericArgument, Ident, LitStr, Meta, PathArguments, Type, TypeParamBound};

#[derive(Default)]
struct Prop {
    into: bool,
    flag: bool,
    boxed: bool,
    name: Option<Ident>,
    func: Option<TypeParamBound>,
}

/// The declared quantity of a `quantity = ".."` option: it states that a numeric prop is not a size, for the
/// `props/point-size` lint. It changes no generated code.
fn read_quantity(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    let quantity: LitStr = meta.value()?.parse()?;
    if quantity.value().trim().is_empty() {
        return Err(syn::Error::new(quantity.span(), "`quantity` names what the number measures, such as \"opacity\" or \"count\""));
    }
    Ok(())
}

fn parse_prop(attr: &Attribute) -> syn::Result<Prop> {
    let mut prop = Prop::default();
    if matches!(attr.meta, Meta::Path(_)) {
        return Ok(prop);
    }
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("into") {
            prop.into = true;
        } else if meta.path.is_ident("flag") {
            prop.flag = true;
        } else if meta.path.is_ident("boxed") {
            prop.boxed = true;
        } else if meta.path.is_ident("name") {
            let name: LitStr = meta.value()?.parse()?;
            prop.name = Some(name.parse().map_err(|_| syn::Error::new(name.span(), "`name` is the setter's identifier"))?);
        } else if meta.path.is_ident("func") {
            let bound: LitStr = meta.value()?.parse()?;
            prop.func = Some(bound.parse().map_err(|_| syn::Error::new(bound.span(), "`func` is a closure bound such as \"Fn(&K) -> bool\""))?);
        } else if meta.path.is_ident("quantity") {
            read_quantity(&meta)?;
        } else {
            return Err(
                meta.error("unknown `prop` option; expected `into`, `flag`, `boxed`, `name = \"..\"`, `func = \"..\"` or `quantity = \"..\"`")
            );
        }
        Ok(())
    })?;
    Ok(prop)
}

/// The first lifetime argument of a path type such as `RowTest<'a, K>`.
fn lifetime_of(ty: &Type) -> Option<&syn::Lifetime> {
    let Type::Path(path) = ty else { return None };
    let PathArguments::AngleBracketed(arguments) = &path.path.segments.last()?.arguments else { return None };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Lifetime(lifetime) => Some(lifetime),
        _ => None,
    })
}

fn setter(field: &Field, attr: &Attribute) -> syn::Result<TokenStream> {
    let Some(field_name) = &field.ident else {
        return Err(syn::Error::new_spanned(field, "`#[prop]` needs a named field"));
    };
    let prop = parse_prop(attr)?;
    let docs: Vec<_> = field.attrs.iter().filter(|attr| attr.path().is_ident("doc")).collect();
    if docs.is_empty() {
        return Err(syn::Error::new_spanned(field_name, "a `#[prop]` needs a doc comment: it becomes the setter's documentation"));
    }
    let name = prop.name.as_ref().unwrap_or(field_name);
    let ty = &field.ty;
    let (parameter, stored) = if prop.flag {
        if prop.into || !is_named(ty, "bool") {
            return Err(syn::Error::new_spanned(ty, "`flag` is for a `bool` field and takes no `into`"));
        }
        let body = quote!(self.#field_name = true;);
        return Ok(quote! {
            #(#docs)*
            pub fn #name(mut self) -> Self {
                #body
                self
            }
        });
    } else if prop.boxed {
        let object = wrapped(ty, "Option").and_then(|inner| wrapped(inner, "Box")).and_then(|boxed| match boxed {
            Type::TraitObject(object) => Some(object),
            _ => None,
        });
        let Some(object) = object else {
            return Err(syn::Error::new_spanned(ty, "`boxed` is for an `Option<Box<dyn Trait>>` field: the setter takes any `T: Trait`"));
        };
        let bounds = &object.bounds;
        return Ok(quote! {
            #(#docs)*
            pub fn #name<T: #bounds>(mut self, value: T) -> Self {
                self.#field_name = ::core::option::Option::Some(::std::boxed::Box::new(value));
                self
            }
        });
    } else if let Some(bound) = &prop.func {
        let Some(inner) = wrapped(ty, "Option") else {
            return Err(syn::Error::new_spanned(ty, "`func` is for an `Option<Named>` field, where `Named::new` takes the closure"));
        };
        let lifetime =
            lifetime_of(inner).ok_or_else(|| syn::Error::new_spanned(inner, "`func` needs the named type to carry the closure's lifetime"))?;
        (quote!(value: impl #bound + #lifetime), quote!(::core::option::Option::Some(<#inner>::new(value))))
    } else if let Some(inner) = wrapped(ty, "Option") {
        if prop.into {
            (quote!(value: impl ::core::convert::Into<#inner>), quote!(::core::option::Option::Some(value.into())))
        } else {
            (quote!(value: #inner), quote!(::core::option::Option::Some(value)))
        }
    } else if prop.into || is_named(ty, "String") {
        (quote!(value: impl ::core::convert::Into<#ty>), quote!(value.into()))
    } else {
        (quote!(value: #ty), quote!(value))
    };
    Ok(quote! {
        #(#docs)*
        pub fn #name(mut self, #parameter) -> Self {
            self.#field_name = #stored;
            self
        }
    })
}

/// A forwarded builder: its name, its parameter type as written in the expansion, and its doc line.
type Forwarded = (&'static str, &'static str, &'static str);

/// The public `CollectionProps` builders a `#[collection]` field forwards. A new `CollectionProps` builder is added
/// here.
const COLLECTION: &[Forwarded] = &[
    ("select", "::craft_ui::contract::SelectMode", "What the user can select."),
    ("reorder", "::craft_ui::contract::ReorderRule", "Rows reorder by dragging and by Alt with the arrow keys."),
    ("rename", "bool", "Rows rename in place: a double click on the label, F2 or the rename command opens the editor."),
    ("columns", "::std::vec::Vec<::craft_ui::contract::ColumnSpec>", "The columns."),
    (
        "sort",
        "impl ::core::convert::Into<::core::option::Option<::craft_ui::contract::ColumnSort>>",
        "The order the app sorted the rows in, drawn on the column's header.",
    ),
    ("activate", "::craft_ui::contract::ActivateOn", "Which gesture activates a row."),
    ("striped", "bool", "Odd rows are tinted: the item reads `ItemCx::row.striped`."),
    ("allow_empty", "bool", "A press on the empty space below the rows empties the selection."),
    ("lead_on_open", "::craft_ui::contract::LeadOnOpen", "Where the lead lands when rows first show."),
    ("lead_follows_hover", "bool", "The lead, and the selection with it, moves to the row under the pointer when the pointer moves."),
    (
        "rows_stamp",
        "u64",
        "The rows are the set `stamp` names (a filtered set, for example); the lead and selection start over when the stamp changes.",
    ),
    ("focus_on_press", "bool", "A primary press moves key focus to the collection; off, it picks and activates and focus stays where it is."),
    ("empty", "impl ::core::convert::Into<::craft_ui::runtime::env::Label>", "The text of the empty state."),
    ("space", "::craft_ui::contract::SpaceAction", "What Space does on the lead row."),
    ("header", "bool", "The header row over the columns is shown."),
    ("digit_keys", "bool", "Digit keys arrive as digit intents instead of type-ahead."),
    ("marquee", "bool", "A drag on the background selects the rows it covers."),
];

/// The `ContainerProps` builders a `#[container]` field forwards: the ones a collection or container takes from its
/// caller under the builder's own name. A new `ContainerProps` builder of that kind is added here. Builders whose name
/// a component writes itself (`label`, `header`, `footer`, `scroll`) stay off the list.
const CONTAINER: &[Forwarded] = &[
    ("sizing", "::craft_ui::contract::Sizing", "How the component takes its size: to its content or to the room it has."),
    ("command_scope", "::craft_ui::runtime::commands::CommandScope", "Commands of `scope` reach the component while focus is in it."),
    ("surface", "::craft_ui::style::SurfaceKind", "The surface painted behind the body."),
    ("live", "::craft_ui::contract::Politeness", "The body is a live region announced with this politeness."),
    ("end", "::craft_ui::contract::EndRegion", "The region at the line's end edge, outside the body scroll."),
    ("position", "::craft_ui::contract::SetPosition", "The component's place in its set, for assistive technology."),
];

fn forwarders(field: &Field, attr: &Attribute, owner: &str, list: &[Forwarded]) -> syn::Result<Vec<TokenStream>> {
    let Some(field_name) = &field.ident else {
        return Err(syn::Error::new_spanned(field, "`#[collection]` and `#[container]` need a named field"));
    };
    attr.meta.require_path_only()?;
    if !is_named(&field.ty, owner) {
        return Err(syn::Error::new_spanned(
            &field.ty,
            format!("`#[{}]` is for a field of type `{owner}`", attr.path().get_ident().map_or_else(String::new, ToString::to_string)),
        ));
    }
    list.iter()
        .map(|(name, ty, doc)| {
            let name = Ident::new(name, field_name.span());
            let ty: Type = syn::parse_str(ty)?;
            Ok(quote! {
                #[doc = #doc]
                #[must_use]
                pub fn #name(mut self, value: #ty) -> Self {
                    self.#field_name = self.#field_name.#name(value);
                    self
                }
            })
        })
        .collect()
}

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let mut setters = Vec::new();
    for field in named_fields(&input, "Props")? {
        if let Some(attr) = field.attrs.iter().find(|attr| attr.path().is_ident("prop")) {
            setters.push(setter(&field, attr)?);
        }
        for attr in &field.attrs {
            if attr.path().is_ident("collection") {
                setters.extend(forwarders(&field, attr, "CollectionProps", COLLECTION)?);
            } else if attr.path().is_ident("container") {
                setters.extend(forwarders(&field, attr, "ContainerProps", CONTAINER)?);
            }
        }
    }
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #ident #type_generics #where_clause {
            #(#setters)*
        }
    })
}
