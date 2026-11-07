//! `contract/unsafe-allow`: no `allow(unsafe_code)` in source. craft-ui keeps `unsafe_code` at `deny`, and the derives
//! write the one scoped allow its registrations need; source text never contains it.

use syn::visit::{self, Visit};
use syn::{Attribute, Meta};

use crate::report::Finding;

use super::Cx;

pub const ID: &str = "contract/unsafe-allow";

struct Allows {
    spans: Vec<proc_macro2::Span>,
}

impl Visit<'_> for Allows {
    fn visit_attribute(&mut self, attr: &Attribute) {
        if matches!(attr.path().get_ident().map(|i| i.to_string()).as_deref(), Some("allow" | "expect")) && names_unsafe_code(&attr.meta) {
            self.spans.push(attr.pound_token.spans[0]);
        }
        visit::visit_attribute(self, attr);
    }
}

fn names_unsafe_code(meta: &Meta) -> bool {
    let Meta::List(list) = meta else { return false };
    list.tokens.clone().into_iter().any(|token| matches!(token, proc_macro2::TokenTree::Ident(ident) if ident == "unsafe_code"))
}

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    let mut allows = Allows { spans: Vec::new() };
    allows.visit_file(cx.file);
    for span in allows.spans {
        cx.flag_span(out, ID, ID, span, "`allow(unsafe_code)`; the derives write the registrations' scoped allow".to_owned());
    }
}
