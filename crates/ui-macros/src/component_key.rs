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

/// What a field type needs of the type's own type parameters: the parameter itself, or an associated type of one
/// (`C::Value`, `<C as Codec>::Value`), which is then the type that must be `MemoProps`.
struct NamedParams<'a> {
    params: &'a [&'a syn::Ident],
    found: Vec<Type>,
}

impl NamedParams<'_> {
    fn add(&mut self, ty: Type) {
        let text = quote!(#ty).to_string();
        if !self.found.iter().any(|known| quote!(#known).to_string() == text) {
            self.found.push(ty);
        }
    }
}

impl<'ast> Visit<'ast> for NamedParams<'_> {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        let first_is_param = path.path.segments.first().is_some_and(|first| self.params.contains(&&first.ident));
        let qself_param = path.qself.as_ref().is_some_and(|q| {
            matches!(&*q.ty, Type::Path(inner) if inner.qself.is_none()
                && inner.path.segments.first().is_some_and(|first| self.params.contains(&&first.ident)))
        });
        if qself_param || (path.qself.is_none() && first_is_param) {
            self.add(Type::Path(path.clone()));
        } else {
            syn::visit::visit_type_path(self, path);
        }
    }
}

fn named_params(ty: &Type, generics: &Generics) -> Vec<Type> {
    let params: Vec<&syn::Ident> = generics.type_params().map(|param| &param.ident).collect();
    let mut visit = NamedParams { params: &params, found: Vec::new() };
    visit.visit_type(ty);
    visit.found
}

/// The crates whose `#[derive(Component)]` types get their `ComponentKey`: every field of every component type there
/// feeds the key or carries `#[key(..)]`. A crate joins once its field types implement `MemoProps`. `craft-layers` has no
/// component type. Still unkeyed: ui-canvas (needs
/// `MemoProps` for kurbo `Rect`, `Point`, `Vec2` and `BezPath` and for its own types) and the crates not yet checked above it; when the last joins, the list goes
/// and every crate gets the key.
const KEYED_CRATES: &[&str] =
    &["craft-ui-contract", "craft-ui-test-kit", "craft-ui-widgets", "craft-ui-panels", "craft-ui-patterns", "craft-ui-shell", "craft-color-ui"];

/// Whether the crate being compiled gets the `ComponentKey` impls (see [`KEYED_CRATES`]).
pub(crate) fn keyed_crate() -> bool {
    std::env::var("CARGO_PKG_NAME").is_ok_and(|name| KEYED_CRATES.contains(&name.as_str()))
}

/// The `ComponentKey` impl for a `#[derive(Component)]` type. A field whose type names a type parameter adds
/// a `MemoProps` bound for that parameter (or its associated type); every other field must be `MemoProps` as written, so a field the key cannot feed
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
    let mut bounded: Vec<String> = Vec::new();
    for field in fields {
        let name = field.ident.as_ref().expect("named fields");
        match feed_of(field)? {
            Feed::Field => {
                // A `&mut T` binding feeds by the value it points at (`T: MemoProps`); `MemoProps` itself rejects
                // `&mut`, so a memoized view cannot hold a write access its key would not see.
                let (ty, access) = match &field.ty {
                    Type::Reference(reference) if reference.mutability.is_some() => (&*reference.elem, quote!(&*self.#name)),
                    ty => (ty, quote!(&self.#name)),
                };
                for param in named_params(ty, &input.generics) {
                    let text = quote!(#param).to_string();
                    if !bounded.contains(&text) {
                        generics.make_where_clause().predicates.push(syn::parse_quote!(#param: #k::contract::MemoProps));
                        bounded.push(text);
                    }
                }
                statements.push(quote_spanned_feed(&k, access, ty));
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
