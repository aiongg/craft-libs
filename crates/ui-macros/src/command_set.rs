//! `#[derive(CommandSet)]` with `#[command(prefix = "..", scope = "..")]` on the enum and
//! `#[command(label = "..", icon = "..", kind = "..", modal = "..", default = "..", path = "..", alias = "..")]` on variants.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, LitStr, Variant};

#[derive(Default)]
struct SetAttrs {
    prefix: Option<LitStr>,
    scope: Option<LitStr>,
}

#[derive(Default)]
struct VariantAttrs {
    label: Option<LitStr>,
    icon: Option<LitStr>,
    kind: Option<LitStr>,
    modal: Option<LitStr>,
    defaults: Vec<LitStr>,
    paths: Vec<LitStr>,
    aliases: Vec<LitStr>,
}

fn command_attrs(attrs: &[Attribute], mut read: impl FnMut(&syn::meta::ParseNestedMeta<'_>) -> syn::Result<bool>) -> syn::Result<()> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("command")) {
        attr.parse_nested_meta(|meta| if read(&meta)? { Ok(()) } else { Err(meta.error("unknown `command` option")) })?;
    }
    Ok(())
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn kind_tokens(kind: &LitStr) -> syn::Result<TokenStream> {
    Ok(match kind.value().as_str() {
        "command" => quote!(::craft_ui::runtime::commands::CommandKind::Command),
        "toggle" => quote!(::craft_ui::runtime::commands::CommandKind::Toggle),
        "momentary" => quote!(::craft_ui::runtime::commands::CommandKind::Momentary),
        _ => return Err(syn::Error::new(kind.span(), "`kind` is `\"command\"`, `\"toggle\"` or `\"momentary\"`")),
    })
}

fn modal_tokens(modal: &LitStr) -> syn::Result<TokenStream> {
    Ok(match modal.value().as_str() {
        "blocked" => quote!(::craft_ui::runtime::commands::ModalRule::Blocked),
        "in_dialogs" => quote!(::craft_ui::runtime::commands::ModalRule::InDialogs),
        "always" => quote!(::craft_ui::runtime::commands::ModalRule::Always),
        _ => return Err(syn::Error::new(modal.span(), "`modal` is `\"blocked\"`, `\"in_dialogs\"` or `\"always\"`")),
    })
}

fn variant(variant: &Variant) -> syn::Result<VariantAttrs> {
    if !matches!(variant.fields, Fields::Unit) {
        return Err(syn::Error::new_spanned(variant, "a command is a unit variant"));
    }
    let mut attrs = VariantAttrs::default();
    command_attrs(&variant.attrs, |meta| {
        let value = |meta: &syn::meta::ParseNestedMeta<'_>| meta.value()?.parse::<LitStr>();
        match () {
            () if meta.path.is_ident("label") => attrs.label = Some(value(meta)?),
            () if meta.path.is_ident("icon") => attrs.icon = Some(value(meta)?),
            () if meta.path.is_ident("kind") => attrs.kind = Some(value(meta)?),
            () if meta.path.is_ident("modal") => attrs.modal = Some(value(meta)?),
            () if meta.path.is_ident("default") => attrs.defaults.push(value(meta)?),
            () if meta.path.is_ident("path") => attrs.paths.push(value(meta)?),
            () if meta.path.is_ident("alias") => attrs.aliases.push(value(meta)?),
            () => return Ok(false),
        }
        Ok(true)
    })?;
    Ok(attrs)
}

pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let ident = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(ident, "`#[derive(CommandSet)]` needs an enum of unit variants"));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&input.generics, "a command set has no generic parameters"));
    }
    let mut set = SetAttrs::default();
    command_attrs(&input.attrs, |meta| {
        if meta.path.is_ident("prefix") {
            set.prefix = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("scope") {
            set.scope = Some(meta.value()?.parse()?);
        } else {
            return Ok(false);
        }
        Ok(true)
    })?;
    let prefix = set.prefix.ok_or_else(|| syn::Error::new_spanned(ident, "a command set names its id prefix: `#[command(prefix = \"..\")]`"))?;
    let scope = match &set.scope {
        Some(scope) => quote!(::craft_ui::runtime::commands::CommandScope::new(#scope)),
        None => quote!(::craft_ui::runtime::commands::CommandScope::APP),
    };
    let (mut ids, mut arms, mut defs) = (Vec::new(), Vec::new(), Vec::new());
    for v in &data.variants {
        let attrs = variant(v)?;
        let name = &v.ident;
        let id = format!("{}.{}", prefix.value(), snake(&name.to_string()));
        if ids.contains(&id) {
            return Err(syn::Error::new_spanned(v, format!("two variants make the command id `{id}`")));
        }
        let label = attrs.label.ok_or_else(|| syn::Error::new_spanned(v, "a command has a label: `#[command(label = \"..\")]`"))?;
        let mut chain = quote!();
        if let Some(icon) = &attrs.icon {
            chain.extend(quote!(.with_icon(#icon)));
        }
        if let Some(kind) = &attrs.kind {
            let kind = kind_tokens(kind)?;
            chain.extend(quote!(.with_kind(#kind)));
        }
        if let Some(modal) = &attrs.modal {
            let modal = modal_tokens(modal)?;
            chain.extend(quote!(.with_modal(#modal)));
        }
        for path in &attrs.paths {
            let segments = path.value().split('/').map(str::to_owned).collect::<Vec<_>>();
            chain.extend(quote!(.with_path(::std::vec![#(::std::string::String::from(#segments)),*])));
        }
        for alias in &attrs.aliases {
            chain.extend(quote!(.with_alias(#alias)));
        }
        let mut defaults = Vec::new();
        for text in &attrs.defaults {
            defaults.push(quote! {
                def = def.with_default(::craft_ui::runtime::commands::Binding::parse(#text).map_err(|error| ::craft_ui::runtime::commands::CommandDefError::binding(#id, error))?);
            });
        }
        arms.push(quote!(#id => ::core::option::Option::Some(Self::#name)));
        defs.push(quote! {
            {
                let scope = Self::SCOPE.scope().ok_or_else(|| ::craft_ui::runtime::commands::CommandDefError::scope(#id))?;
                let mut def = ::craft_ui::runtime::commands::CommandDef::new(::craft_ui::runtime::commands::CommandRef::new(#id), #label, scope) #chain;
                #(#defaults)*
                out.push(def);
            }
        });
        ids.push(id);
    }
    let names: Vec<_> = data.variants.iter().map(|v| &v.ident).collect();
    // Inside craft-ui the registration's `link_section` static meets the crate's `deny(unsafe_code)`; see `component.rs`.
    let allow = crate::util::in_craft_ui().then(|| quote!(#[allow(unsafe_code)]));
    Ok(quote! {
        impl ::craft_ui::runtime::commands::CommandSet for #ident {
            const SCOPE: ::craft_ui::runtime::commands::CommandScope = #scope;

            fn from_ref(command: &::craft_ui::runtime::commands::CommandRef) -> ::core::option::Option<Self> {
                match command.id.as_str() {
                    #(#arms,)*
                    _ => ::core::option::Option::None,
                }
            }

            fn id(self) -> &'static str {
                match self {
                    #(Self::#names => #ids,)*
                }
            }

            fn defs() -> ::core::result::Result<::std::vec::Vec<::craft_ui::runtime::commands::CommandDef>, ::craft_ui::runtime::commands::CommandDefError> {
                let mut out = ::std::vec::Vec::new();
                #(#defs)*
                ::core::result::Result::Ok(out)
            }
        }

        impl ::core::convert::From<#ident> for ::craft_ui::runtime::commands::CommandRef {
            fn from(command: #ident) -> Self {
                ::craft_ui::runtime::commands::CommandRef::new(::craft_ui::runtime::commands::CommandSet::id(command))
            }
        }

        #allow
        const _: () = {
            ::craft_ui::__register_command_set!(#ident);
        };
    })
}
