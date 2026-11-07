//! Macros of craft-ui: `#[derive(Component, Props, ComponentStyle, TokenGroup, CommandSet, EnvValue)]`,
//! `words!`, `#[story]`, `#[craft_ui::testing::trial]` and `register_contract!`.

mod component;
mod env;
mod props;
mod style;
mod token_group;
mod util;
mod words;

#[cfg(test)]
mod tests;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, ReturnType, parse_macro_input};

fn expanded(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Writes `Meta`, the `<Type>StyleKey` type, `HasProps` for a container or collection, and the type's
/// registration and probe.
///
/// Attributes: `#[component]`, `#[component(container)]`, `#[component(collection)]`, `widget`, `overlay`,
/// `content_painter`, `canvas_control`, `exempt(case = .., reason = ..)`, `probe(Type<..>, ..)` on the type;
/// `#[common]`, `#[container]` and `#[style_key]` on fields.
#[proc_macro_derive(Component, attributes(component, common, container, style_key, prop))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    expanded(component::derive(input.into()))
}

/// Registers an `impl Component` written without the derive: `register_contract!(Type)`, with instantiations
/// for a generic type (`register_contract!(Field<'static, Plain>, Field<'static, Number>)`) and options after
/// a semicolon (`register_contract!(Menu; overlay)`).
#[proc_macro]
pub fn register_contract(input: TokenStream) -> TokenStream {
    expanded(component::register(input.into()))
}

/// Writes one setter per `#[prop]` field: `#[prop]`, `#[prop(into)]`, `#[prop(flag)]`, `#[prop(name = "..")]`.
/// The field's doc comment is the setter's.
#[proc_macro_derive(Props, attributes(prop))]
pub fn derive_props(input: TokenStream) -> TokenStream {
    expanded(props::derive(input.into()))
}

/// Writes `ComponentStyle` from `#[style(surface)]`, `#[style(focus)]` and `#[style(metrics)]` fields and
/// `#[style(variants = [..])]` on the type.
#[proc_macro_derive(ComponentStyle, attributes(style))]
pub fn derive_component_style(input: TokenStream) -> TokenStream {
    expanded(style::derive(input.into()))
}

/// Writes `EnvValue`; `#[env(style)]` also writes `AFFECTS_STYLE`, the style hash and the `StyleValue` marker,
/// and `#[env(root = ..)]` gives the root value (`Default` otherwise).
#[proc_macro_derive(EnvValue, attributes(env))]
pub fn derive_env_value(input: TokenStream) -> TokenStream {
    expanded(env::derive(input.into()))
}

/// Writes `TokenGroup`'s `PATH` and theme overlay from `#[group(path = "..")]`; the author writes an inherent
/// `fn derive(tokens: &Tokens) -> Self`.
#[proc_macro_derive(TokenGroup, attributes(group))]
pub fn derive_token_group(input: TokenStream) -> TokenStream {
    expanded(token_group::derive(input.into()))
}

/// Writes a struct of built-in strings and its `WordSet` impl, one row per field. A row is
/// `field: "module.camelCase" = "English";`, or starts with `template`, `unit`, `count` (`"{n} item" | "{n} items"`),
/// `array`, `context` or `plain`. The macro checks ids, placeholders and that unit and count rows contain `{n}`.
#[proc_macro]
pub fn words(input: TokenStream) -> TokenStream {
    expanded(words::expand_words(input.into()))
}

/// Registers a function as a trial of craft-ui's integration binary.
///
/// The function takes no arguments and returns `()`; it fails by panicking. The registration lists it in
/// `craft_ui::testing::TRIALS`, so the runner finds it without a hand-kept list.
#[proc_macro_attribute]
pub fn trial(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "`trial` takes no arguments").to_compile_error().into();
    }
    let func = parse_macro_input!(item as ItemFn);
    let sig = &func.sig;
    let well_formed = sig.inputs.is_empty() && sig.asyncness.is_none() && sig.generics.params.is_empty() && matches!(sig.output, ReturnType::Default);
    if !well_formed {
        return syn::Error::new_spanned(sig, "a trial is `fn name()` with no arguments, generics or return value").to_compile_error().into();
    }
    let ident = &sig.ident;
    let name = ident.to_string();
    let registration = format_ident!("__CRAFT_TRIAL_{}", name.to_uppercase());
    quote! {
        #func

        #[::craft_ui::testing::__private::linkme::distributed_slice(::craft_ui::testing::TRIALS)]
        #[linkme(crate = ::craft_ui::testing::__private::linkme)]
        #[doc(hidden)]
        static #registration: ::craft_ui::testing::Trial = ::craft_ui::testing::Trial::new(module_path!(), #name, #ident);
    }
    .into()
}
