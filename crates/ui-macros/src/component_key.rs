//! `#[derive(ComponentKey)]`: the component key of the contract on craft-ui-rt (docs/contract-rt.md
//! section 6). Every field feeds the key through `MemoProps`, in order, so the key covers what the
//! hooks read from `self`. `#[key(skip)]` leaves out a field the hooks do not read (the id source,
//! a callback); `#[key(by = expr)]` feeds `expr` instead (it may name `self`). A type whose key
//! cannot be written carries `#[component(unkeyed)]` and runs at every build of its scope; the
//! other `#[component(..)]` arguments belong to `#[derive(Component)]`.

use crate::util::{fields_or_unit, ui_crate};
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{DeriveInput, Meta, Token};

enum Feed {
    Field,
    Skip,
    By(syn::Expr),
}

fn feed_of(field: &syn::Field) -> syn::Result<Feed> {
    let mut feed = Feed::Field;
    for attr in field.attrs.iter().filter(|attr| attr.path().is_ident("key")) {
        attr.parse_nested_meta(|meta| {
            if !matches!(feed, Feed::Field) {
                return Err(meta.error("a field feeds the key one way: `skip` or `by = ..`"));
            }
            if meta.path.is_ident("skip") {
                feed = Feed::Skip;
                Ok(())
            } else if meta.path.is_ident("by") {
                feed = Feed::By(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("expected `#[key(skip)]` or `#[key(by = expr)]`"))
            }
        })?;
    }
    Ok(feed)
}

fn unkeyed(input: &DeriveInput) -> syn::Result<bool> {
    for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("component")) {
        let args = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        if args.iter().any(|meta| matches!(meta, Meta::Path(path) if path.is_ident("unkeyed"))) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let k = ui_crate();
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    if unkeyed(&input)? {
        return Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #k::contract::rt::ComponentKey for #ident #type_generics #where_clause {
                const UNKEYED: bool = true;
                fn key(&self, _: &mut #k::contract::MemoKey) {}
            }
        });
    }
    let mut statements = Vec::new();
    for field in fields_or_unit(&input, "ComponentKey")? {
        let name = field.ident.as_ref().expect("named fields");
        statements.push(match feed_of(&field)? {
            Feed::Field => quote!(#k::contract::MemoProps::key(&self.#name, key);),
            Feed::Skip => quote!(),
            Feed::By(expr) => quote!(#k::contract::MemoProps::key(&(#expr), key);),
        });
    }
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #k::contract::rt::ComponentKey for #ident #type_generics #where_clause {
            fn key(&self, key: &mut #k::contract::MemoKey) {
                #(#statements)*
            }
        }
    })
}
