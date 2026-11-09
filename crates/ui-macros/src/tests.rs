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
fn command_set_prefix_is_one_lower_camel_segment() {
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
    assert!(tokens.contains("out.push(&self.label);out.extend(self.hint.as_ref());"), "{tokens}");
    assert!(tokens.contains("fnicons"), "{tokens}");
}

fn expansion(input: TokenStream) -> String {
    crate::props::derive(input).map(|tokens| tokens.to_string().split_whitespace().collect::<String>()).unwrap_or_default()
}

#[test]
fn a_collection_field_forwards_the_collection_builders_and_a_container_field_the_container_ones() {
    let collection = expansion(quote! { struct Grid { #[collection] props: CollectionProps } });
    for setter in ["select", "sort", "rows_stamp", "header"] {
        assert!(collection.contains(&format!("pubfn{setter}(mutself,value:")), "{setter}: {collection}");
    }
    assert!(collection.contains("self.props=self.props.rows_stamp(value)"), "{collection}");
    let container = expansion(quote! { struct Grid { #[container] props: ContainerProps } });
    for setter in ["sizing", "surface"] {
        assert!(container.contains(&format!("pubfn{setter}(mutself,value:")), "{setter}: {container}");
    }
    assert!(!container.contains("pubfnselect("), "{container}");
}

#[test]
fn registration_is_gated_only_when_the_package_declares_screen_testing() {
    let compact = |tokens: TokenStream| tokens.to_string().split_whitespace().collect::<String>();
    let declared = "[package]\nname = \"app\"\n\n[features]\nscreen-testing = [\"craft-ui/testing\"]\n";
    assert_eq!(compact(crate::component::gate_for(declared)), "#[cfg(feature=\"screen-testing\")]");
    let undeclared = "[package]\nname = \"gallery\"\nscreen-testing = 1\n\n[dependencies]\nscreen-testing = \"1\"\n";
    assert_eq!(compact(crate::component::gate_for(undeclared)), "");
    assert_eq!(compact(crate::component::gate_for("")), "");
}
