//! `#[derive(Props)]`: one setter per `#[prop]` field.

use crate::util::{is_named, named_fields, wrapped};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Field, GenericArgument, Ident, LitStr, Meta, PathArguments, Type, TypeParamBound};

#[derive(Default)]
struct Prop {
    into: bool,
    flag: bool,
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
        } else if meta.path.is_ident("name") {
            let name: LitStr = meta.value()?.parse()?;
            prop.name = Some(name.parse().map_err(|_| syn::Error::new(name.span(), "`name` is the setter's identifier"))?);
        } else if meta.path.is_ident("func") {
            let bound: LitStr = meta.value()?.parse()?;
            prop.func = Some(bound.parse().map_err(|_| syn::Error::new(bound.span(), "`func` is a closure bound such as \"Fn(&K) -> bool\""))?);
        } else if meta.path.is_ident("quantity") {
            read_quantity(&meta)?;
        } else {
            return Err(meta.error("unknown `prop` option; expected `into`, `flag`, `name = \"..\"`, `func = \"..\"` or `quantity = \"..\"`"));
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

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let mut setters = Vec::new();
    for field in named_fields(&input, "Props")? {
        if let Some(attr) = field.attrs.iter().find(|attr| attr.path().is_ident("prop")) {
            setters.push(setter(&field, attr)?);
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
