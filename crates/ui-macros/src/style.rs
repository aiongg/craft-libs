//! `#[derive(ComponentStyle)]`: the trait from field attributes.

use crate::util::{named_fields, wrapped};
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Expr, Field, Ident, Token};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Surface,
    Focus,
    Metrics,
}

fn part_of(field: &Field) -> syn::Result<Option<Part>> {
    let mut part = None;
    for attr in field.attrs.iter().filter(|attr| attr.path().is_ident("style")) {
        attr.parse_nested_meta(|meta| {
            let found = if meta.path.is_ident("surface") {
                Part::Surface
            } else if meta.path.is_ident("focus") {
                Part::Focus
            } else if meta.path.is_ident("metrics") {
                Part::Metrics
            } else {
                return Err(meta.error("a field is `#[style(surface)]`, `#[style(focus)]` or `#[style(metrics)]`"));
            };
            part = Some(found);
            Ok(())
        })?;
    }
    Ok(part)
}

fn variants(attrs: &[Attribute]) -> syn::Result<Option<Vec<Expr>>> {
    let mut found = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("style")) {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("variants") {
                return Err(meta.error("the type takes `#[style(variants = [..])]`"));
            }
            let list: syn::ExprArray = meta.value()?.parse()?;
            found = Some(list.elems.into_iter().collect());
            Ok(())
        })?;
    }
    Ok(found)
}

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let fields = named_fields(&input, "ComponentStyle")?;
    let (mut surface, mut focus, mut metrics): (Option<&Field>, Option<&Field>, Option<&Field>) = (None, None, None);
    for field in &fields {
        let slot = match part_of(field)? {
            Some(Part::Surface) => &mut surface,
            Some(Part::Focus) => &mut focus,
            Some(Part::Metrics) => &mut metrics,
            None => continue,
        };
        if slot.replace(field).is_some() {
            return Err(syn::Error::new_spanned(field, "at most one field carries each `#[style(..)]` part"));
        }
    }
    let ident = &input.ident;
    let metrics = metrics.ok_or_else(|| syn::Error::new_spanned(ident, "a style has one field marked `#[style(metrics)]`, of type `Metrics`"))?;
    let accessor = |field: Option<&Field>, name: &str, ty: TokenStream| -> Option<TokenStream> {
        let field = field?;
        let name = Ident::new(name, ident.span());
        let member = field.ident.as_ref()?;
        let body =
            if wrapped(&field.ty, "Option").is_some() { quote!(self.#member.as_ref()) } else { quote!(::core::option::Option::Some(&self.#member)) };
        Some(quote! {
            fn #name(&self) -> ::core::option::Option<&#ty> {
                #body
            }
        })
    };
    let surface = accessor(surface, "surface", quote!(::craft_ui::style::SurfaceLook));
    let focus = accessor(focus, "focus", quote!(::craft_ui::style::FocusLook));
    let metrics_field = &metrics.ident;
    let variants = variants(&input.attrs)?.map(|list| {
        let list = Punctuated::<Expr, Token![,]>::from_iter(list);
        quote!(const VARIANTS: &'static [::craft_ui::style::Variant] = &[#list];)
    });
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics ::craft_ui::style::ComponentStyle for #ident #type_generics #where_clause {
            #variants
            #surface
            #focus

            fn metrics(&self) -> ::craft_ui::style::Metrics {
                self.#metrics_field
            }
        }
    })
}
