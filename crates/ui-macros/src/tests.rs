//! The derives' error paths, on the expansion functions: a trybuild case builds a project per case, which the
//! one-integration-binary layout of craft-ui's tests (spec 22.2) does not fit.

use proc_macro2::TokenStream;
use quote::quote;

fn error(result: syn::Result<TokenStream>) -> String {
    match result {
        Ok(tokens) => panic!("expected an error, got {tokens}"),
        Err(error) => error.to_string(),
    }
}

fn component(input: TokenStream) -> String {
    error(crate::component::derive(input))
}

#[test]
fn a_generic_component_without_probe_is_an_error() {
    let message = component(quote! {
        struct Field<C: 'static> { #[common] common: Common, codec: C }
    });
    assert!(message.contains("probe"), "{message}");
}

#[test]
fn a_lifetime_only_component_needs_no_probe() {
    let tokens = crate::component::derive(quote! {
        struct Field<'a> { #[common] common: Common, text: &'a str }
    });
    assert!(tokens.is_ok_and(|tokens| tokens.to_string().contains("Field < 'static >")));
}

#[test]
fn probe_errors_name_the_rule() {
    let on_plain = component(quote! {
        #[component(probe(Plain))]
        struct Plain { #[common] common: Common }
    });
    assert!(on_plain.contains("type parameters"), "{on_plain}");
    let elsewhere = component(quote! {
        #[component(probe(Other<u8>))]
        struct Field<C: 'static> { #[common] common: Common, codec: C }
    });
    assert!(elsewhere.contains("instantiations of `Field`"), "{elsewhere}");
    let borrowed = component(quote! {
        #[component(probe(Field<'a, u8>))]
        struct Field<'a, C: 'static> { #[common] common: Common, codec: &'a C }
    });
    assert!(borrowed.contains("'static"), "{borrowed}");
}

#[test]
fn field_markers_are_checked_against_the_shape() {
    let missing = component(quote!(
        struct Leaf {
            size: u8,
        }
    ));
    assert!(missing.contains("#[common]"), "{missing}");
    let body_without_props = component(quote! {
        #[component(container)]
        struct Body { #[common] common: Common }
    });
    assert!(body_without_props.contains("#[container]"), "{body_without_props}");
    let leaf_with_props = component(quote! {
        struct Leaf { #[common] common: Common, #[container] props: ContainerProps }
    });
    assert!(leaf_with_props.contains("#[container]"), "{leaf_with_props}");
    let canvas_body = component(quote! {
        #[component(container, canvas_control)]
        struct Body { #[common] common: Common, #[container] props: ContainerProps }
    });
    assert!(canvas_body.contains("canvas_control"), "{canvas_body}");
}

#[test]
fn a_prop_without_a_doc_comment_is_an_error() {
    let message = error(crate::props::derive(quote! {
        struct Button {
            #[prop] loading: bool,
        }
    }));
    assert!(message.contains("doc comment"), "{message}");
}

#[test]
fn a_flag_prop_must_be_a_bool() {
    let message = error(crate::props::derive(quote! {
        struct Button {
            /// Count.
            #[prop(flag)] count: u8,
        }
    }));
    assert!(message.contains("bool"), "{message}");
}

#[test]
fn a_group_path_needs_a_crate_prefix_and_its_fields_are_colours_or_shapes() {
    let unprefixed = error(crate::token_group::derive(quote! {
        #[group(path = "clips")]
        struct Clips { audio: Rgba }
    }));
    assert!(unprefixed.contains("crate prefix"), "{unprefixed}");
    let missing = error(crate::token_group::derive(quote!(
        struct Clips {
            audio: Rgba,
        }
    )));
    assert!(missing.contains("path"), "{missing}");
    let wrong_field = error(crate::token_group::derive(quote! {
        #[group(path = "craft_timeline.clips")]
        struct Clips { height: f32 }
    }));
    assert!(wrong_field.contains("colours"), "{wrong_field}");
}

#[test]
fn a_style_needs_a_metrics_field() {
    let message = error(crate::style::derive(quote! {
        struct Look { #[style(surface)] surface: SurfaceLook }
    }));
    assert!(message.contains("metrics"), "{message}");
}

#[test]
fn register_contract_needs_a_type_and_known_options() {
    assert!(error(crate::component::register(quote!())).contains("component types"));
    assert!(error(crate::component::register(quote!(Chip; sticky))).contains("overlay"));
}

#[test]
fn option_errors_name_the_rule() {
    let widget = component(quote! {
        #[component(container, widget)]
        struct Body { #[common] common: Common, #[container] props: ContainerProps }
    });
    assert!(widget.contains("leaf"), "{widget}");
    let partial = component(quote! {
        #[component(exempt(case = "sizes"))]
        struct Leaf { #[common] common: Common }
    });
    assert!(partial.contains("both"), "{partial}");
    let both = component(quote! {
        #[component(container, collection)]
        struct Leaf { #[common] common: Common, #[container] props: ContainerProps }
    });
    assert!(both.contains("not both"), "{both}");
    let unknown = component(quote! {
        #[component(sticky)]
        struct Leaf { #[common] common: Common }
    });
    assert!(unknown.contains("unknown `component` option"), "{unknown}");
}

#[test]
fn a_field_carries_one_style_part() {
    let message = error(crate::style::derive(quote! {
        struct Look { #[style(surface)] #[style(focus)] both: SurfaceLook, #[style(metrics)] metrics: Metrics }
    }));
    assert!(message.contains("one"), "{message}");
}

#[test]
fn type_parameters_of_a_component_are_static_and_sized() {
    let bare = component(quote!(
        struct Field<C> {
            #[common]
            common: Common,
            c: C,
        }
    ));
    assert!(bare.contains("C: 'static"), "{bare}");
    let unsized_ = component(quote!(
        struct Field<C: 'static + ?Sized> {
            #[common]
            common: Common,
            c: Box<C>,
        }
    ));
    assert!(unsized_.contains("Sized"), "{unsized_}");
    let clause = crate::component::derive(quote! {
        #[component(probe(Field<u8>))]
        struct Field<C> where C: 'static { #[common] common: Common, c: C }
    });
    assert!(clause.is_ok(), "a where-clause bound counts");
    let associated = component(quote! {
        #[component(probe(Field<u8>))]
        struct Field<C: 'static + Codec> { #[common] common: Common, #[style_key] kind: C::Kind }
    });
    assert!(associated.contains("associated type"), "{associated}");
}

#[test]
fn style_key_fields_carry_docs() {
    let tokens = crate::component::derive(quote! {
        pub struct Gauge {
            #[common] common: Common,
            /// How full.
            #[style_key] fill: u8,
            #[style_key] tone: u8,
        }
    });
    let text = tokens.map(|tokens| tokens.to_string()).unwrap_or_default();
    assert!(text.contains("How full.") && text.contains("The `tone` input of the style key."), "{text}");
}

#[test]
fn a_widget_impl_keeps_the_types_bounds_and_requires_a_response_output() {
    let tokens = crate::component::derive(quote! {
        #[component(widget)]
        struct Gauge<'a> { #[common] common: Common, text: &'a str }
    });
    let text = tokens.map(|tokens| tokens.to_string()).unwrap_or_default();
    assert!(text.contains("Output = :: craft_ui :: __private :: egui :: Response"), "{text}");
}

#[test]
fn command_set_derive_needs_a_prefix_and_labels() {
    assert!(error(crate::command_set::derive(quote!(enum Set { #[command(label = "A")] A }))).contains("prefix"));
    assert!(error(crate::command_set::derive(quote!(#[command(prefix = "p")] enum Set { A }))).contains("label"));
    assert!(error(crate::command_set::derive(quote!(#[command(prefix = "p")] enum Set { #[command(label = "A", kind = "x")] A }))).contains("kind"));
    assert!(error(crate::command_set::derive(quote!(#[command(prefix = "p")] enum Set { #[command(label = "A")] A(u8) }))).contains("unit"));
}

fn words(input: TokenStream) -> String {
    error(crate::words::expand_words(input))
}

#[test]
fn words_rows_are_checked() {
    let id = words(quote! { struct W { save: "Save" = "Save"; } });
    assert!(id.contains("module.camelCase"), "{id}");
    let unit = words(quote! { struct W { size: unit "w.size" = "px"; } });
    assert!(unit.contains("{n}"), "{unit}");
    let count = words(quote! { struct W { items: count "w.items" = "{n} item" | "items"; } });
    assert!(count.contains("{n}"), "{count}");
    let braces = words(quote! { struct W { saved: template "w.saved" = "Saved {name"; } });
    assert!(braces.contains("unmatched"), "{braces}");
}
