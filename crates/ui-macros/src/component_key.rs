//! The component key `#[derive(Component)]` writes (docs/contract-rt.md section 6): the key that decides whether a
//! component on craft-ui-rt runs or keeps its outputs. Every field feeds it through `MemoProps`, in order, so the key
//! covers what the hooks read from `self`. `#[key(skip)]` leaves out a field the hooks do not read (a callback the
//! run only calls on input); `#[key(by = expr)]` feeds `expr` instead (it may name `self`). A type whose key cannot be
//! written carries `#[component(unkeyed)]` and runs at every build of its scope.

use crate::util::ui_crate;
use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;
use syn::{DeriveInput, Field, Generics, Type};

enum Feed {
    Field,
    Skip,
    By(syn::Expr),
}

fn feed_of(field: &Field) -> syn::Result<Feed> {
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

/// Whether `ty` names one of the type's own type parameters.
struct NamesParam<'a> {
    params: &'a [&'a syn::Ident],
    found: bool,
}

impl<'ast> Visit<'ast> for NamesParam<'_> {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if path.qself.is_none() && path.path.segments.first().is_some_and(|first| self.params.contains(&&first.ident)) {
            self.found = true;
        }
        syn::visit::visit_type_path(self, path);
    }
}

fn names_param(ty: &Type, generics: &Generics) -> bool {
    let params: Vec<&syn::Ident> = generics.type_params().map(|param| &param.ident).collect();
    let mut visit = NamesParam { params: &params, found: false };
    visit.visit_type(ty);
    visit.found
}

/// The crates whose `#[derive(Component)]` types get their `ComponentKey`: every field of every component type there
/// feeds the key or carries `#[key(..)]`. A crate joins once its field types implement `MemoProps` (the spike B3
/// report lists what each of ui-widgets, ui-panels, ui-patterns, ui-shell, layers and color-ui still needs); then the
/// list goes and every crate gets the key.
const KEYED_CRATES: &[&str] = &["craft-ui-contract", "craft-ui-test-kit"];

/// Whether the crate being compiled gets the `ComponentKey` impls (see [`KEYED_CRATES`]).
pub(crate) fn keyed_crate() -> bool {
    std::env::var("CARGO_PKG_NAME").is_ok_and(|name| KEYED_CRATES.contains(&name.as_str()))
}

/// The `ComponentKey` impl for a `#[derive(Component)]` type. A field whose type names a type parameter adds a
/// `MemoProps` bound for that type; every other field must be `MemoProps` as written, so a field the key cannot feed
/// is an error at the field.
pub(crate) fn key_impl(input: &DeriveInput, fields: &[Field], unkeyed: bool) -> syn::Result<TokenStream> {
    let k = ui_crate();
    let ident = &input.ident;
    if unkeyed {
        let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
        return Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #k::contract::ComponentKey for #ident #type_generics #where_clause {
                const UNKEYED: bool = true;
                fn key(&self, _: &mut #k::contract::MemoKey) {}
            }
        });
    }
    let mut generics = input.generics.clone();
    let mut statements = Vec::new();
    for field in fields {
        let name = field.ident.as_ref().expect("named fields");
        match feed_of(field)? {
            Feed::Field => {
                if names_param(&field.ty, &input.generics) {
                    let ty = &field.ty;
                    generics.make_where_clause().predicates.push(syn::parse_quote!(#ty: #k::contract::MemoProps));
                }
                let span = field.ty.clone();
                statements.push(quote_spanned_feed(&k, quote!(&self.#name), &span));
            }
            Feed::Skip => {}
            Feed::By(expr) => statements.push(quote!(#k::contract::MemoProps::key(&(#expr), key);)),
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #k::contract::ComponentKey for #ident #type_generics #where_clause {
            fn key(&self, key: &mut #k::contract::MemoKey) {
                #(#statements)*
            }
        }
    })
}

/// `MemoProps::key(field, key)` with the error span on the field's type, so a field the key cannot feed points there.
fn quote_spanned_feed(k: &TokenStream, field: TokenStream, ty: &Type) -> TokenStream {
    let span = syn::spanned::Spanned::span(ty);
    quote::quote_spanned!(span=> <#ty as #k::contract::MemoProps>::key(#field, key);)
}
