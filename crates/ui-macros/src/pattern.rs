//! `#[derive(Pattern)]`: the marker impl and the registration.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Type};

/// The instantiations a `#[pattern(probe(Type<..>, ..))]` lists.
fn probe_types(attrs: &[Attribute]) -> syn::Result<Vec<Type>> {
    let mut types = Vec::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("pattern")) {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("probe") {
                return Err(meta.error("unknown `pattern` option; expected `probe(..)`"));
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
    Ok(types)
}

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    if !matches!(input.data, syn::Data::Struct(_)) {
        return Err(syn::Error::new_spanned(&input.ident, "`#[derive(Pattern)]` needs a struct"));
    }
    let ident = &input.ident;
    let types = crate::component::instantiations(ident, &input.generics, probe_types(&input.attrs)?, "pattern")?;
    let purpose = crate::component::purpose_of(&input.attrs);
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let registration = crate::component::pattern_registrations(&types, &purpose);
    Ok(quote! {
        impl #impl_generics ::craft_ui::contract::Pattern for #ident #type_generics #where_clause {}
        #registration
    })
}
