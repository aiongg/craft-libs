//! Macros of craft-ui: `#[derive(Component, Pattern, Props, ComponentStyle, TokenGroup, CommandSet, EnvValue)]`,
//! `words!`, `#[story]`, `#[craft_ui::testing::trial]` and `register_contract!`.

mod command_set;
mod component;
mod env;
mod pattern;
mod props;
mod style;
mod token_group;
mod util;
mod words;

#[cfg(test)]
mod tests;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, ItemFn, ReturnType, Token, Type, parse_macro_input};

fn expanded(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Writes `CommandSet` for an enum of unit variants and registers the set, which the command board loads at
/// install. `#[command(prefix = "..", scope = "..")]` on the enum gives the id prefix (`prefix.variant_name`)
/// and the scope (the application scope without one); each variant takes `label`, `icon`, `kind`, `modal`,
/// `default`, `path` and `alias`.
#[proc_macro_derive(CommandSet, attributes(command))]
pub fn derive_command_set(input: TokenStream) -> TokenStream {
    expanded(command_set::derive(input.into()))
}

/// Writes `Meta`, the `<Type>StyleKey` type, `HasProps` for a container or collection, and the type's
/// registration and probe.
///
/// Attributes: `#[component]`, `#[component(container)]`, `#[component(collection)]`, `widget`, `overlay`,
/// `content_painter`, `canvas_control`, `exempt(case = .., reason = ..)`, `probe(Type<..>, ..)` on the type;
/// `#[common]`, `#[container]` and `#[style_key]` on fields.
#[proc_macro_derive(Component, attributes(component, common, container, style_key))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    expanded(component::derive(input.into()))
}

/// Writes `Pattern` for a struct that only assembles components, and registers it so the components index and the
/// suite list it. `#[pattern(probe(Type<..>, ..))]` lists the instantiations of a type with type parameters.
#[proc_macro_derive(Pattern, attributes(pattern))]
pub fn derive_pattern(input: TokenStream) -> TokenStream {
    expanded(pattern::derive(input.into()))
}

/// Registers an `impl Component` written without the derive: `register_contract!(Type)`, with instantiations
/// for a generic type (`register_contract!(Field<'static, Plain>, Field<'static, Number>)`) and options after
/// a semicolon (`register_contract!(Menu; container, overlay)`; the shape is a leaf unless `container` or
/// `collection` is given).
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
    let allow = util::in_craft_ui().then(|| quote!(#[allow(unsafe_code)]));
    quote! {
        #func

        #allow
        #[::craft_ui::testing::__private::linkme::distributed_slice(::craft_ui::testing::TRIALS)]
        #[linkme(crate = ::craft_ui::testing::__private::linkme)]
        #[doc(hidden)]
        static #registration: ::craft_ui::testing::Trial = ::craft_ui::testing::Trial::new(module_path!(), #name, #ident);
    }
    .into()
}

/// The arguments of `#[story(Type, covers = [..], drive = ..)]`.
struct StoryArgs {
    ty: Type,
    covers: Vec<Ident>,
    drive: Option<Expr>,
}

impl Parse for StoryArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ty: Type = input.parse()?;
        let (mut covers, mut drive) = (Vec::new(), None);
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "covers" {
                let list;
                syn::bracketed!(list in input);
                covers = Punctuated::<Ident, Token![,]>::parse_terminated(&list)?.into_iter().collect();
            } else if key == "drive" {
                drive = Some(input.parse::<Expr>()?);
            } else {
                return Err(syn::Error::new(key.span(), "`story` takes `covers = [..]` and `drive = ..`"));
            }
        }
        Ok(StoryArgs { ty, covers, drive })
    }
}

/// Registers a function as a story of a component type: `#[story(Slider, covers = [Default, Mixed])]` on
/// `fn(&mut Ui, &mut StoryData) -> StoryOutput`. `drive = Drive::Step` names a drive other than the automatic one.
/// The type's registration and the story meet by the type's registered name.
#[proc_macro_attribute]
pub fn story(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as StoryArgs);
    let func = parse_macro_input!(item as ItemFn);
    let ident = &func.sig.ident;
    let name = ident.to_string();
    let registration = format_ident!("__CRAFT_STORY_{}", name.to_uppercase());
    let ty = &args.ty;
    let covers = &args.covers;
    let drive = args.drive.as_ref().map(|drive| quote!(.drive(#drive)));
    let allow = util::in_craft_ui().then(|| quote!(#[allow(unsafe_code)]));
    quote! {
        #func

        #allow
        #[::craft_ui::testing::__private::linkme::distributed_slice(::craft_ui::testing::STORIES)]
        #[linkme(crate = ::craft_ui::testing::__private::linkme)]
        #[doc(hidden)]
        static #registration: ::craft_ui::testing::StoryRegistration = ::craft_ui::testing::StoryRegistration::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_CRATE_NAME"),
            ::std::any::type_name::<#ty>,
            ::craft_ui::testing::Story::new(#name, #ident).covers(&[#(::craft_ui::testing::Required::#covers),*]) #drive,
        );
    }
    .into()
}
