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
fn component_derive_rejects_invalid_input() {
    {
        let message = component(quote! {
            struct Field<C: 'static> { #[common] common: Common, codec: C }
        });
        assert!(message.contains("probe"), "{message}");
    }
    {
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
    {
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
    {
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
    {
        let message = error(crate::style::derive(quote! {
            struct Look { #[style(surface)] #[style(focus)] both: SurfaceLook, #[style(metrics)] metrics: Metrics }
        }));
        assert!(message.contains("one"), "{message}");
    }
}

#[test]
fn an_unregistered_component_implements_meta_and_registers_nothing() {
    let tokens = crate::component::derive(quote! {
        #[component(unregistered)]
        struct Row { #[common] common: Common }
    })
    .expect("the derive accepts `unregistered`")
    .to_string();
    assert!(tokens.contains("Meta"), "{tokens}");
    assert!(!tokens.contains("__register_contract"), "{tokens}");
    let with_option = component(quote! {
        #[component(unregistered, overlay)]
        struct Row { #[common] common: Common }
    });
    assert!(with_option.contains("unregistered"), "{with_option}");
}

#[test]
fn a_lifetime_only_component_needs_no_probe() {
    let tokens = crate::component::derive(quote! {
        struct Field<'a> { #[common] common: Common, text: &'a str }
    });
    assert!(tokens.is_ok_and(|tokens| tokens.to_string().contains("Field < 'static >")));
}

#[test]
fn props_style_and_registration_reject_invalid_input() {
    {
        let message = error(crate::props::derive(quote! {
            struct Button {
                #[prop] loading: bool,
            }
        }));
        assert!(message.contains("doc comment"), "{message}");
    }
    {
        let message = error(crate::props::derive(quote! {
            struct Button {
                /// Count.
                #[prop(flag)] count: u8,
            }
        }));
        assert!(message.contains("bool"), "{message}");
    }
    {
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
    {
        let message = error(crate::style::derive(quote! {
            struct Look { #[style(surface)] surface: SurfaceLook }
        }));
        assert!(message.contains("metrics"), "{message}");
    }
    {
        assert!(error(crate::component::register(quote!())).contains("component types"));
        assert!(error(crate::component::register(quote!(Chip; sticky))).contains("overlay"));
    }
    {
        let message = error(crate::props::derive(quote! { struct Grid { #[collection] props: ContainerProps } }));
        assert!(message.contains("CollectionProps"), "{message}");
    }
}

#[test]
fn a_quantity_names_what_a_prop_measures_and_changes_no_setter() {
    let plain = crate::props::derive(quote! {
        struct Fade {
            /// Strength.
            #[prop] opacity: f32,
        }
    })
    .unwrap()
    .to_string();
    let declared = crate::props::derive(quote! {
        struct Fade {
            /// Strength.
            #[prop(quantity = "opacity")] opacity: f32,
        }
    })
    .unwrap()
    .to_string();
    assert_eq!(plain, declared);
    let message = error(crate::props::derive(quote! {
        struct Fade {
            /// Strength.
            #[prop(quantity = " ")] opacity: f32,
        }
    }));
    assert!(message.contains("quantity"), "{message}");
}

#[test]
fn type_parameters_of_a_component_are_static_and_sized() {
    component(quote!(
        struct Field<C> {
            #[common]
            common: Common,
            c: C,
        }
    ));
    component(quote!(
        struct Field<C: 'static + ?Sized> {
            #[common]
            common: Common,
            c: Box<C>,
        }
    ));
    let clause = crate::component::derive(quote! {
        #[component(probe(Field<u8>))]
        struct Field<C> where C: 'static { #[common] common: Common, c: C }
    });
    assert!(clause.is_ok(), "a where-clause bound counts");
    component(quote! {
        #[component(probe(Field<u8>))]
        struct Field<C: 'static + Codec> { #[common] common: Common, #[style_key] kind: C::Kind }
    });
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
    assert!(text.contains("Response"), "{text}");
}

#[test]
fn command_set_and_words_reject_invalid_input() {
    {
        assert!(
            error(crate::command_set::derive(quote!(
                enum Set {
                    #[command(label = "A")]
                    A,
                }
            )))
            .contains("prefix")
        );
        assert!(
            error(crate::command_set::derive(quote!(
                #[command(prefix = "p")]
                enum Set {
                    A,
                }
            )))
            .contains("label")
        );
        assert!(
            error(crate::command_set::derive(quote!(
                #[command(prefix = "p")]
                enum Set {
                    #[command(label = "A", kind = "x")]
                    A,
                }
            )))
            .contains("kind")
        );
        assert!(
            error(crate::command_set::derive(quote!(
                #[command(prefix = "p")]
                enum Set {
                    #[command(label = "A")]
                    A(u8),
                }
            )))
            .contains("unit")
        );
    }
    {
        for prefix in ["canvas.view", "kbd_trial", "Canvas", "", "9p"] {
            let message = error(crate::command_set::derive(quote!(
                #[command(prefix = #prefix)]
                enum Set {
                    #[command(label = "A")]
                    A,
                }
            )));
            assert!(message.contains("lower-camel"), "`{prefix}`: {message}");
        }
        let accepted = crate::command_set::derive(quote!(
            #[command(prefix = "canvasView")]
            enum Set {
                #[command(label = "A")]
                A,
            }
        ));
        assert!(accepted.is_ok());
    }
    {
        let id = words(quote! { struct W { save: "Save" = "Save"; } });
        assert!(id.contains("module.camelCase"), "{id}");
        let unit = words(quote! { struct W { size: unit "w.size" = "px"; } });
        assert!(unit.contains("{n}"), "{unit}");
        let count = words(quote! { struct W { items: count "w.items" = "{n} item" | "items"; } });
        assert!(count.contains("{n}"), "{count}");
        let braces = words(quote! { struct W { saved: template "w.saved" = "Saved {name"; } });
        assert!(braces.contains("unmatched"), "{braces}");
    }
}

fn words(input: TokenStream) -> String {
    error(crate::words::expand_words(input))
}

#[test]
fn text_and_icon_fields_are_listed_in_field_order() {
    let tokens = crate::style::derive(quote! {
        struct Look {
            #[style(metrics)] metrics: Metrics,
            #[style(text)] label: TextLook,
            #[style(text)] hint: Option<TextLook>,
            #[style(icon)] glyph: IconLook,
        }
    })
    .map(|tokens| tokens.to_string().split_whitespace().collect::<String>());
    let tokens = tokens.unwrap_or_default();
    let (label, hint) = (tokens.find("self.label"), tokens.find("self.hint"));
    assert!(label.is_some() && label < hint, "{tokens}");
    assert!(tokens.contains("fnicons"), "{tokens}");
}

fn expansion(input: TokenStream) -> String {
    crate::props::derive(input).map(|tokens| tokens.to_string().split_whitespace().collect::<String>()).unwrap_or_default()
}

#[test]
fn a_collection_field_forwards_the_collection_builders_and_a_container_field_the_container_ones() {
    let collection = expansion(quote! { struct Grid { #[collection] props: CollectionProps } });
    for setter in ["select", "sort", "rows_stamp", "header"] {
        assert!(collection.contains(&format!("fn{setter}(")), "{setter}: {collection}");
    }
    let container = expansion(quote! { struct Grid { #[container] props: ContainerProps } });
    for setter in ["sizing", "surface"] {
        assert!(container.contains(&format!("fn{setter}(")), "{setter}: {container}");
    }
    assert!(!container.contains("fnselect("), "{container}");
}

#[test]
fn registration_outside_craft_ui_is_gated_only_when_the_package_declares_screen_testing() {
    let compact = |tokens: TokenStream| tokens.to_string().split_whitespace().collect::<String>();
    let declared = "[package]\nname = \"app\"\n\n[features]\nscreen-testing = [\"craft-ui/testing\"]\n";
    assert_eq!(compact(crate::component::gate_for(false, declared)), "#[cfg(feature=\"screen-testing\")]");
    let undeclared = "[package]\nname = \"gallery\"\nscreen-testing = 1\n\n[dependencies]\nscreen-testing = \"1\"\n";
    assert_eq!(compact(crate::component::gate_for(false, undeclared)), "");
    assert_eq!(compact(crate::component::gate_for(false, "")), "");
    assert!(compact(crate::component::gate_for(true, declared)).contains("allow(unsafe_code)"));
}
