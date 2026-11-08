//! `#[derive(Pattern)]`: the marker impl and the registration.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Ident, Type};

/// What a `#[derive(Pattern)]` declares: the instantiations a `#[pattern(probe(Type<..>, ..))]` lists, and the
/// `PatternProbe` constructor of its kind (`#[pattern(tool)]`, `#[pattern(dialog)]`; `of` without either).
struct Options {
    types: Vec<Type>,
    probe: Ident,
}

fn options(attrs: &[Attribute]) -> syn::Result<Options> {
    let mut types = Vec::new();
    let mut probe = Ident::new("of", proc_macro2::Span::call_site());
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("pattern")) {
        attr.parse_nested_meta(|meta| {
            for kind in ["tool", "dialog"] {
                if meta.path.is_ident(kind) {
                    probe = Ident::new(kind, proc_macro2::Span::call_site());
                    return Ok(());
                }
            }
            if !meta.path.is_ident("probe") {
                return Err(meta.error("unknown `pattern` option; expected `probe(..)`, `tool` or `dialog`"));
            }
            let content;
            syn::parenthesized!(content in meta.input);
            let listed = syn::punctuated::Punctuated::<Type, syn::Token![,]>::parse_terminated(&content)?;
            if listed.is_empty() {
                return Err(meta.error("`probe` lists the instantiations to register"));
            }
            types.extend(listed);
            Ok(())
        })?;
    }
    Ok(Options { types, probe })
}

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    if !matches!(input.data, syn::Data::Struct(_)) {
        return Err(syn::Error::new_spanned(&input.ident, "`#[derive(Pattern)]` needs a struct"));
    }
    let ident = &input.ident;
    let Options { types, probe } = options(&input.attrs)?;
    let types = crate::component::instantiations(ident, &input.generics, types, "pattern")?;
    let purpose = crate::component::purpose_of(&input.attrs);
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let registration = crate::component::pattern_registrations(&types, &purpose, &probe);
    Ok(quote! {
        impl #impl_generics ::craft_ui::contract::Pattern for #ident #type_generics #where_clause {}
        #registration
    })
}
