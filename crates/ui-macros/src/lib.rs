//! Macros of craft-ui: `#[derive(Component, Props, ComponentStyle, TokenGroup, CommandSet, EnvValue)]`,
//! `words!`, `#[story]`, `#[craft_ui::testing::trial]` and `register_contract!`.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, ReturnType, parse_macro_input};

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
