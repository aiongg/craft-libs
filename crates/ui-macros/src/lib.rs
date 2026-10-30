//! Derive macros for `craft_ui::story`: [`StoryArgs`] and [`StoryChoice`]. Use them through
//! `craft_ui::story`, which re-exports both.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, LitStr, Type, parse_macro_input, spanned::Spanned};

/// Derives `StoryArgs` for a struct with named fields. Each field is one arg, in field order.
///
/// - `bool`: a checkbox.
/// - `String`: a text field.
/// - `f64`, `f32` and the integer types: a number; `#[arg(range = MIN..=MAX)]` is required.
/// - any other type: a choice; the type derives `StoryChoice`.
///
/// `#[arg(name = "...")]` sets the arg's name where it is not the field name. Two fields with the
/// same arg name are a compile error.
///
/// The kind is read from the field's type name: only the single-segment names `bool`, `String`,
/// `f64`, `f32` and the integer types are recognised. A path (`std::string::String`) or an alias
/// is taken as a choice type. Integer fields read back the rounded value of the number arg.
#[proc_macro_derive(StoryArgs, attributes(arg))]
pub fn derive_story_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_args(&input).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Derives `StoryChoice` for an enum of unit variants. An option's name is the variant name in
/// lower case unless `#[choice(name = "...")]` sets it. Options keep the variant order.
#[proc_macro_derive(StoryChoice, attributes(choice))]
pub fn derive_story_choice(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_choice(&input).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// The value of `key = "..."` in the attributes called `attr`.
fn string_attr(attrs: &[syn::Attribute], attr: &str, key: &str) -> syn::Result<Option<String>> {
    let mut found = None;
    for a in attrs.iter().filter(|a| a.path().is_ident(attr)) {
        a.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                found = Some(meta.value()?.parse::<LitStr>()?.value());
                Ok(())
            } else {
                Err(meta.error(format!("unknown `{attr}` option; expected `{key}`")))
            }
        })?;
    }
    Ok(found)
}

/// `#[arg(name = "...", range = MIN..=MAX)]`.
struct ArgAttr {
    name: Option<String>,
    range: Option<(Expr, Expr)>,
}

fn arg_attr(attrs: &[syn::Attribute]) -> syn::Result<ArgAttr> {
    let mut out = ArgAttr { name: None, range: None };
    for a in attrs.iter().filter(|a| a.path().is_ident("arg")) {
        a.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                out.name = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("range") {
                let value: Expr = meta.value()?.parse()?;
                match value {
                    Expr::Range(r) if matches!(r.limits, syn::RangeLimits::Closed(_)) => match (r.start, r.end) {
                        (Some(start), Some(end)) => out.range = Some((*start, *end)),
                        _ => return Err(meta.error("`range` needs both ends: `MIN..=MAX`")),
                    },
                    _ => return Err(meta.error("`range` must be a closed range: `MIN..=MAX`")),
                }
            } else {
                return Err(meta.error("unknown `arg` option; expected `name` or `range`"));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

enum Kind {
    Bool,
    Text,
    Number(Type, bool),
    Choice(Type),
}

fn kind_of(ty: &Type) -> Kind {
    let ident = match ty {
        Type::Path(p) if p.qself.is_none() => p.path.get_ident().map(ToString::to_string),
        _ => None,
    };
    match ident.as_deref() {
        Some("bool") => Kind::Bool,
        Some("String") => Kind::Text,
        Some("f64" | "f32") => Kind::Number(ty.clone(), false),
        Some("u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize") => Kind::Number(ty.clone(), true),
        _ => Kind::Choice(ty.clone()),
    }
}

fn expand_args(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "`StoryArgs` derives on structs with named fields"));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(input.span(), "`StoryArgs` derives on structs with named fields"));
    };
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut adds = Vec::new();
    let mut reads = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for field in &fields.named {
        let Some(ident) = &field.ident else { continue };
        let attr = arg_attr(&field.attrs)?;
        let arg_name = attr.name.unwrap_or_else(|| ident.to_string());
        if !seen.insert(arg_name.clone()) {
            return Err(syn::Error::new(field.span(), format!("two args are named `{arg_name}`")));
        }
        let kind = kind_of(&field.ty);
        if attr.range.is_some() && !matches!(kind, Kind::Number(..)) {
            return Err(syn::Error::new(field.span(), "`range` applies to number args only"));
        }
        let (add, read) = match kind {
            Kind::Bool => (quote!(.with_bool(#arg_name, self.#ident)), quote!(args.bool(#arg_name))),
            Kind::Text => (quote!(.with_text(#arg_name, self.#ident.clone())), quote!(args.text(#arg_name).to_owned())),
            Kind::Number(ty, integer) => {
                let Some((min, max)) = attr.range else {
                    return Err(syn::Error::new(field.span(), "a number arg needs `#[arg(range = MIN..=MAX)]`"));
                };
                (
                    quote!(.with_number(#arg_name, self.#ident as f64, (#min) as f64, (#max) as f64)),
                    if integer { quote!(args.number(#arg_name).round() as #ty) } else { quote!(args.number(#arg_name) as #ty) },
                )
            }
            Kind::Choice(ty) => (
                quote!(.with_choice(#arg_name, <#ty as ::craft_ui::story::StoryChoice>::OPTIONS, ::craft_ui::story::StoryChoice::index(&self.#ident))),
                quote!(<#ty as ::craft_ui::story::StoryChoice>::from_index(args.choice_index(#arg_name))),
            ),
        };
        adds.push(add);
        reads.push(quote!(#ident: #read));
    }
    Ok(quote! {
        impl #impl_generics ::craft_ui::story::StoryArgs for #name #ty_generics #where_clause {
            fn to_args(&self) -> ::craft_ui::story::Args {
                ::craft_ui::story::Args::new() #(#adds)*
            }

            fn from_args(args: &::craft_ui::story::Args) -> Self {
                Self { #(#reads),* }
            }
        }
    })
}

fn expand_choice(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "`StoryChoice` derives on enums"));
    };
    if data.variants.is_empty() {
        return Err(syn::Error::new(input.span(), "`StoryChoice` needs at least one variant"));
    }
    let name = &input.ident;
    let mut names = Vec::new();
    let mut index_arms = Vec::new();
    let mut from_arms = Vec::new();
    let last = data.variants.len() - 1;
    for (i, variant) in data.variants.iter().enumerate() {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(variant.span(), "`StoryChoice` variants must be unit variants"));
        }
        let ident = &variant.ident;
        let option = string_attr(&variant.attrs, "choice", "name")?.unwrap_or_else(|| ident.to_string().to_lowercase());
        names.push(LitStr::new(&option, variant.span()));
        index_arms.push(quote!(Self::#ident => #i));
        // An index past the end selects the last option, as `Args::with_choice` clamps.
        from_arms.push(if i == last { quote!(_ => Self::#ident) } else { quote!(#i => Self::#ident) });
    }
    Ok(quote! {
        impl ::craft_ui::story::StoryChoice for #name {
            const OPTIONS: &'static [&'static str] = &[#(#names),*];

            fn index(&self) -> usize {
                match self { #(#index_arms),* }
            }

            fn from_index(index: usize) -> Self {
                match index { #(#from_arms),* }
            }
        }
    })
}
