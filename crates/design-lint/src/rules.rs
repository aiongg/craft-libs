//! The `syn`-based rules: literal sizes and named colour constants.

use std::collections::HashMap;
use std::ops::RangeInclusive;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::visit::{self, Visit};
use syn::{Attribute, BinOp, Expr, ExprCall, ExprMethodCall, ExprPath, ExprStruct, Lit, Meta, Token, UnOp};

use crate::report::Finding;

pub const VEC2: &str = "literal-size/vec2";
pub const ADD_SPACE: &str = "literal-size/add-space";
pub const CORNER_RADIUS: &str = "literal-size/corner-radius";
pub const STROKE: &str = "literal-size/stroke";
pub const MARGIN: &str = "literal-size/margin";
pub const FONT_ID: &str = "literal-size/font-id";
pub const RAW_COLOUR_CONST: &str = "raw-colour/const";
pub const PARSE: &str = "parse";

/// Every rule id the syntax rules emit.
pub const RULES: [&str; 8] = [VEC2, ADD_SPACE, CORNER_RADIUS, STROKE, MARGIN, FONT_ID, RAW_COLOUR_CONST, PARSE];

const NAMED_COLOURS: [&str; 21] = [
    "WHITE",
    "BLACK",
    "RED",
    "GREEN",
    "BLUE",
    "YELLOW",
    "GRAY",
    "DARK_GRAY",
    "LIGHT_GRAY",
    "LIGHT_RED",
    "DARK_RED",
    "ORANGE",
    "GOLD",
    "BROWN",
    "LIGHT_YELLOW",
    "LIGHT_GREEN",
    "DARK_GREEN",
    "LIGHT_BLUE",
    "DARK_BLUE",
    "PURPLE",
    "KHAKI",
];

/// The findings of one source file and the line ranges skipped as test code.
#[derive(Debug, Default)]
pub struct Analysis {
    /// Findings in source order; `path` and `area` are empty until the caller fills them.
    pub findings: Vec<Finding>,
    /// Line ranges (1-based, inclusive) of `#[cfg(test)]` and `#[test]` items that were not checked.
    pub skipped: Vec<RangeInclusive<usize>>,
}

/// Checks one file's source text. A file that does not parse yields one `parse` finding.
pub fn analyze_source(src: &str, include_tests: bool) -> Analysis {
    let file = match syn::parse_file(src) {
        Ok(file) => file,
        Err(e) => {
            let start = e.span().start();
            let finding = Finding::new(PARSE, start.line.max(1), start.column + 1, start.line.max(1), e.to_string());
            return Analysis { findings: vec![finding], skipped: Vec::new() };
        }
    };
    let mut linter = Linter { lines: src.lines().collect(), consts: HashMap::new(), out: Analysis::default(), include_tests };
    if linter.skips(&file.attrs, file.span()) {
        return linter.out;
    }
    linter.resolve_consts(&file);
    linter.visit_file(&file);
    linter.out.findings.sort_by_key(|f| (f.line, f.column));
    linter.out
}

/// A same-file `const` or `static` whose value is a numeric literal expression.
struct ConstValue {
    value: f64,
    line: usize,
}

struct Linter<'a> {
    lines: Vec<&'a str>,
    consts: HashMap<String, ConstValue>,
    out: Analysis,
    include_tests: bool,
}

/// Collects `const NAME: T = <expr>` and `static` items, in any scope.
#[derive(Default)]
struct ConstCollector {
    items: Vec<(String, usize, Expr)>,
}

impl<'ast> Visit<'ast> for ConstCollector {
    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        self.items.push((i.ident.to_string(), i.ident.span().start().line, (*i.expr).clone()));
        visit::visit_item_const(self, i);
    }
    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        self.items.push((i.ident.to_string(), i.ident.span().start().line, (*i.expr).clone()));
        visit::visit_item_static(self, i);
    }
    fn visit_impl_item_const(&mut self, i: &'ast syn::ImplItemConst) {
        self.items.push((i.ident.to_string(), i.ident.span().start().line, i.expr.clone()));
        visit::visit_impl_item_const(self, i);
    }
}

fn single_ident(e: &ExprPath) -> Option<String> {
    let path = &e.path;
    (e.qself.is_none() && path.leading_colon.is_none() && path.segments.len() == 1 && path.segments[0].arguments.is_none())
        .then(|| path.segments[0].ident.to_string())
}

/// The last two segments of a path, `("Vec2", "new")` for `egui::Vec2::new`.
fn last_two(path: &syn::Path) -> Option<(String, String)> {
    let n = path.segments.len();
    (n >= 2).then(|| (path.segments[n - 2].ident.to_string(), path.segments[n - 1].ident.to_string()))
}

fn last_ident(path: &syn::Path) -> Option<String> {
    path.segments.last().map(|s| s.ident.to_string())
}

fn cfg_is_test(meta: &Meta) -> bool {
    match meta {
        Meta::Path(p) => p.is_ident("test"),
        Meta::List(l) if l.path.is_ident("all") => {
            l.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).is_ok_and(|inner| inner.iter().any(cfg_is_test))
        }
        _ => false,
    }
}

fn is_test_attr(attr: &Attribute) -> bool {
    if attr.path().is_ident("test") {
        return true;
    }
    if !attr.path().is_ident("cfg") {
        return false;
    }
    match &attr.meta {
        Meta::List(l) => l.parse_args::<Meta>().is_ok_and(|m| cfg_is_test(&m)),
        _ => false,
    }
}

impl Linter<'_> {
    /// Whether an item with these attributes is test code to skip; records its lines when it is.
    fn skips(&mut self, attrs: &[Attribute], span: proc_macro2::Span) -> bool {
        if self.include_tests || !attrs.iter().any(is_test_attr) {
            return false;
        }
        self.out.skipped.push(span.start().line..=span.end().line);
        true
    }

    fn resolve_consts(&mut self, file: &syn::File) {
        let mut collector = ConstCollector::default();
        collector.visit_file(file);
        let mut pending = collector.items;
        loop {
            let before = pending.len();
            pending.retain(|(name, line, expr)| match self.eval(expr) {
                Some(value) => {
                    self.consts.entry(name.clone()).or_insert(ConstValue { value, line: *line });
                    false
                }
                None => true,
            });
            if pending.len() == before {
                break;
            }
        }
    }

    /// The value of an expression made of numeric literals, same-file consts, negation, casts, parentheses and arithmetic.
    fn eval(&self, e: &Expr) -> Option<f64> {
        match e {
            Expr::Lit(l) => match &l.lit {
                Lit::Float(f) => f.base10_parse().ok(),
                Lit::Int(i) => i.base10_parse().ok(),
                _ => None,
            },
            Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)) => self.eval(&u.expr).map(|v| -v),
            Expr::Paren(p) => self.eval(&p.expr),
            Expr::Group(g) => self.eval(&g.expr),
            Expr::Cast(c) => self.eval(&c.expr),
            Expr::Binary(b) => {
                let (l, r) = (self.eval(&b.left)?, self.eval(&b.right)?);
                match b.op {
                    BinOp::Add(_) => Some(l + r),
                    BinOp::Sub(_) => Some(l - r),
                    BinOp::Mul(_) => Some(l * r),
                    BinOp::Div(_) => Some(l / r),
                    BinOp::Rem(_) => Some(l % r),
                    _ => None,
                }
            }
            Expr::Path(p) => single_ident(p).and_then(|name| self.consts.get(&name)).map(|c| c.value),
            _ => None,
        }
    }

    /// The const a bare identifier argument names, when it is one.
    fn const_name(&self, e: &Expr) -> Option<(String, usize)> {
        match e {
            Expr::Paren(p) => self.const_name(&p.expr),
            Expr::Group(g) => self.const_name(&g.expr),
            Expr::Path(p) => {
                let name = single_ident(p)?;
                let line = self.consts.get(&name)?.line;
                Some((name, line))
            }
            _ => None,
        }
    }

    fn is_flagged_size(&self, e: &Expr) -> bool {
        self.eval(e).is_some_and(|v| v != 0.0)
    }

    /// Reports `what` when any of `args` is a non-zero literal size.
    fn size_rule<'e>(&mut self, rule: &'static str, what: proc_macro2::Span, args: impl IntoIterator<Item = &'e Expr>) {
        let flagged: Vec<&Expr> = args.into_iter().filter(|a| self.is_flagged_size(a)).collect();
        if flagged.is_empty() {
            return;
        }
        let snippet = self.snippet(what);
        let named: Vec<String> = flagged.iter().filter_map(|a| self.const_name(a)).map(|(n, l)| format!("`{n}` (const, line {l})")).collect();
        let uses = if named.is_empty() { "literal sizes".to_owned() } else { format!("{}, a literal size", named.join(", ")) };
        let message = format!("`{snippet}` uses {uses}; {}", hint(rule));
        self.report(rule, what, message);
    }

    fn report(&mut self, rule: &'static str, span: proc_macro2::Span, message: String) {
        let (start, end) = (span.start(), span.end());
        self.out.findings.push(Finding::new(rule, start.line, start.column + 1, end.line, message));
    }

    /// The source text of a span; the first line and an ellipsis when it spans several lines.
    fn snippet(&self, span: proc_macro2::Span) -> String {
        let (start, end) = (span.start(), span.end());
        let Some(line) = self.lines.get(start.line.wrapping_sub(1)) else { return String::new() };
        let chars: Vec<char> = line.chars().collect();
        if start.line == end.line {
            chars.get(start.column..end.column.min(chars.len())).map(|s| s.iter().collect()).unwrap_or_default()
        } else {
            let first: String = chars.get(start.column..).map(|s| s.iter().collect()).unwrap_or_default();
            format!("{} …", first.trim_end())
        }
    }

    fn check_call(&mut self, call: &ExprCall) {
        let Expr::Path(func) = &*call.func else { return };
        let span = call.span();
        let args = call.args.iter();
        if last_ident(&func.path).as_deref() == Some("vec2") {
            self.size_rule(VEC2, span, args);
            return;
        }
        let Some((ty, f)) = last_two(&func.path) else { return };
        match (ty.as_str(), f.as_str()) {
            ("Vec2", "new" | "splat") => self.size_rule(VEC2, span, args),
            ("CornerRadius", "same" | "from") => self.size_rule(CORNER_RADIUS, span, args),
            ("Stroke" | "PathStroke", "new") => self.size_rule(STROKE, span, args.take(1)),
            ("Margin", "same" | "symmetric") => self.size_rule(MARGIN, span, args),
            ("FontId", "proportional" | "monospace" | "new") => self.size_rule(FONT_ID, span, args.take(1)),
            _ => {}
        }
    }

    fn check_method(&mut self, call: &ExprMethodCall) {
        let name = call.method.to_string();
        let rule = match name.as_str() {
            "add_space" => ADD_SPACE,
            "corner_radius" => CORNER_RADIUS,
            "inner_margin" | "outer_margin" => MARGIN,
            _ => return,
        };
        let end = call.paren_token.span.close();
        let span = call.method.span().join(end).unwrap_or_else(|| call.method.span());
        self.size_rule(rule, span, call.args.iter());
    }

    fn check_struct(&mut self, s: &ExprStruct) {
        let Some(ty) = last_ident(&s.path) else { return };
        let (rule, fields): (&'static str, &[&str]) = match ty.as_str() {
            "Vec2" => (VEC2, &["x", "y"]),
            "CornerRadius" => (CORNER_RADIUS, &["nw", "ne", "sw", "se"]),
            "Stroke" | "PathStroke" => (STROKE, &["width"]),
            "Margin" => (MARGIN, &["left", "right", "top", "bottom"]),
            _ => return,
        };
        let values =
            s.fields.iter().filter(|f| matches!(&f.member, syn::Member::Named(n) if fields.contains(&n.to_string().as_str()))).map(|f| &f.expr);
        self.size_rule(rule, s.span(), values);
    }

    fn check_path(&mut self, p: &ExprPath) {
        let Some((ty, name)) = last_two(&p.path) else { return };
        if ty == "Color32" && NAMED_COLOURS.contains(&name.as_str()) {
            let snippet = self.snippet(p.span());
            let message = format!(
                "`{snippet}`; take the colour from a semantic token (`Tokens::of(ui).color.text.normal.to_egui()`) or mark `// design: allow(<reason>)`"
            );
            self.report(RAW_COLOUR_CONST, p.span(), message);
        }
    }
}

fn hint(rule: &str) -> &'static str {
    match rule {
        VEC2 => "use a size token (`Tokens::of(ui).size.control.md`, `space.gap.inline`) or mark `// design: allow(<reason>)`",
        ADD_SPACE => "use a spacing token (`Tokens::of(ui).space.gap.inline`) or mark `// design: allow(<reason>)`",
        CORNER_RADIUS => "use a radius token from `Tokens::of(ui)` or mark `// design: allow(<reason>)`",
        STROKE => "use a stroke width token from `Tokens::of(ui)` or mark `// design: allow(<reason>)`",
        MARGIN => "use a spacing token (`Tokens::of(ui).space`) or mark `// design: allow(<reason>)`",
        _ => "use a type-size token from `Tokens::of(ui)` or mark `// design: allow(<reason>)`",
    }
}

impl<'ast> Visit<'ast> for Linter<'_> {
    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_item_mod(self, i);
        }
    }
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_item_fn(self, i);
        }
    }
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_item_impl(self, i);
        }
    }
    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_impl_item_fn(self, i);
        }
    }
    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_trait_item_fn(self, i);
        }
    }
    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_item_const(self, i);
        }
    }
    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        if !self.skips(&i.attrs, i.span()) {
            visit::visit_item_static(self, i);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        self.check_call(call);
        visit::visit_expr_call(self, call);
    }
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        self.check_method(call);
        visit::visit_expr_method_call(self, call);
    }
    fn visit_expr_struct(&mut self, s: &'ast ExprStruct) {
        self.check_struct(s);
        visit::visit_expr_struct(self, s);
    }
    fn visit_expr_path(&mut self, p: &'ast ExprPath) {
        self.check_path(p);
        visit::visit_expr_path(self, p);
    }

    /// Macro arguments are checked when they parse as comma-separated expressions (or `expr; expr`).
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if let Ok(args) = m.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
            for arg in &args {
                self.visit_expr(arg);
            }
        } else if let Ok((a, b)) = m.parse_body_with(|input: syn::parse::ParseStream| {
            let a: Expr = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok((a, input.parse::<Expr>()?))
        }) {
            self.visit_expr(&a);
            self.visit_expr(&b);
        }
    }
}
