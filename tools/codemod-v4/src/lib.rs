//! codemod-v4: the rename rules of contract-rt.md section 15.11 (rules 1-24) as a syn-based source rewriter.
//!
//! The rules are found on the syntax tree and applied as byte-span replacements, so formatting and comments outside
//! the rewritten tokens stay as written. A file is rewritten in rounds: each round applies the outermost of any
//! overlapping edits, and the next round finds what was left inside. Every rule is idempotent, so the rounds end.
//! Sites the do-not-touch list names (and rule sites whose receiver the tool cannot judge) come back as [`Skip`]s.

mod tables;

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use proc_macro2::{LineColumn, TokenStream, TokenTree};
use quote::ToTokens as _;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprField, ExprMethodCall, ExprReference, FnArg, ItemStruct, ItemUse, Local, Member, Pat, Path, Type, UseTree};

use tables::{Item, Kind, Service};

/// What differs between crates.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The crate is `craft-ui-contract`: rule 1-4 targets are `crate::contract::X`, the other targets are left.
    pub inside_contract: bool,
}

/// A site the tool left alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Skip {
    pub class: &'static str,
    pub line: usize,
    pub detail: String,
}

/// The result of one file.
#[derive(Debug, Default)]
pub struct Output {
    pub text: String,
    /// Applied edits per rule.
    pub counts: BTreeMap<&'static str, usize>,
    pub skips: Vec<Skip>,
}

struct Edit {
    start: usize,
    end: usize,
    text: String,
    rules: Vec<&'static str>,
}

/// Rewrites one source file.
pub fn transform(src: &str, opts: &Options) -> Result<Output, syn::Error> {
    let mut out = Output { text: src.to_owned(), ..Output::default() };
    for round in 0..24 {
        let file = syn::parse_file(&out.text)?;
        let mut pass = Pass::new(&out.text, opts);
        pass.run(&file);
        if round == 0 {
            out.skips = std::mem::take(&mut pass.skips);
            out.skips.sort_by(|a, b| (a.class, a.line, &a.detail).cmp(&(b.class, b.line, &b.detail)));
            out.skips.dedup();
        }
        let mut edits = pass.edits;
        if edits.is_empty() {
            break;
        }
        edits.sort_by_key(|e| (e.start, Reverse(e.end)));
        let mut kept: Vec<Edit> = Vec::new();
        let mut limit = 0;
        for e in edits {
            if e.start >= limit {
                limit = e.end;
                kept.push(e);
            }
        }
        for e in kept.iter().rev() {
            out.text.replace_range(e.start..e.end, &e.text);
        }
        for e in &kept {
            for rule in &e.rules {
                *out.counts.entry(rule).or_default() += 1;
            }
        }
    }
    Ok(out)
}

struct Leaf {
    path: Vec<String>,
    alias: Option<String>,
    glob: bool,
}

struct UseInfo {
    start: usize,
    end: usize,
    /// Attributes and visibility, as written before `use`.
    head: String,
    indent: String,
    leaves: Vec<Leaf>,
}

struct Pass<'a> {
    src: &'a str,
    starts: Vec<usize>,
    opts: &'a Options,
    prep: bool,
    edits: Vec<Edit>,
    skips: Vec<Skip>,
    uses: Vec<UseInfo>,
    /// Ranges of `Fixture` identifiers in `use` items below a `testing` module, and whether an alias follows.
    fixture_leaves: Vec<(usize, usize, bool)>,
    blocked: BTreeSet<&'static str>,
    tracked: HashSet<String>,
    renames: HashMap<String, &'static str>,
    egui_names: HashSet<String>,
    no_touch: Vec<(usize, usize)>,
    consumed: HashSet<usize>,
    chains_seen: HashSet<usize>,
}

fn flatten(tree: &UseTree, prefix: &mut Vec<String>, out: &mut Vec<Leaf>) {
    match tree {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            flatten(&p.tree, prefix, out);
            prefix.pop();
        }
        UseTree::Name(n) => {
            let mut path = prefix.clone();
            if n.ident != "self" {
                path.push(n.ident.to_string());
            }
            out.push(Leaf { path, alias: None, glob: false });
        }
        UseTree::Rename(r) => {
            let mut path = prefix.clone();
            if r.ident != "self" {
                path.push(r.ident.to_string());
            }
            out.push(Leaf { path, alias: Some(r.rename.to_string()), glob: false });
        }
        UseTree::Glob(_) => out.push(Leaf { path: prefix.clone(), alias: None, glob: true }),
        UseTree::Group(g) => {
            for item in &g.items {
                flatten(item, prefix, out);
            }
        }
    }
}

fn path_names(p: &Path) -> Vec<String> {
    p.segments.iter().map(|s| s.ident.to_string()).collect()
}

fn strip_refs(e: &Expr) -> &Expr {
    match e {
        Expr::Reference(r) => strip_refs(&r.expr),
        _ => e,
    }
}

fn strip_all(e: &Expr) -> &Expr {
    match e {
        Expr::Reference(r) => strip_all(&r.expr),
        Expr::Paren(p) => strip_all(&p.expr),
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => strip_all(&u.expr),
        _ => e,
    }
}

fn single_ident(e: &Expr) -> Option<String> {
    match strip_all(e) {
        Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => Some(p.path.segments[0].ident.to_string()),
        _ => None,
    }
}

fn field_named<'e>(e: &'e Expr, name: &str) -> Option<&'e ExprField> {
    match e {
        Expr::Field(f) if matches!(&f.member, Member::Named(id) if id == name) => Some(f),
        _ => None,
    }
}

fn is_ui(e: &Expr) -> bool {
    let e = strip_all(e);
    single_ident(e).is_some_and(|s| s == "ui" || s.ends_with("_ui") || s.starts_with("ui_")) || field_named(e, "ui").is_some()
}

fn is_cx_base(e: &Expr) -> bool {
    let e = strip_all(e);
    single_ident(e).is_some_and(|s| s == "cx" || s.ends_with("_cx")) || field_named(e, "cx").is_some()
}

/// `cx.ctx` where `cx` is a build context.
fn cx_ctx_field(e: &Expr) -> Option<&ExprField> {
    field_named(e, "ctx").filter(|f| is_cx_base(&f.base))
}

/// `ui.ctx()`: the context of a `ui`; returns the `ui` expression.
fn ui_ctx_call(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::MethodCall(m) if m.method == "ctx" && m.args.is_empty() && is_ui(&m.receiver) => Some(&m.receiver),
        _ => None,
    }
}

fn is_ctx_expr(e: &Expr) -> bool {
    let e = strip_all(e);
    ui_ctx_call(e).is_some() || single_ident(e).is_some_and(|s| s == "ctx" || s.ends_with("_ctx")) || field_named(e, "ctx").is_some()
}

fn needs_parens(e: &Expr) -> bool {
    !matches!(
        e,
        Expr::Path(_) | Expr::Field(_) | Expr::MethodCall(_) | Expr::Call(_) | Expr::Index(_) | Expr::Paren(_) | Expr::Macro(_) | Expr::Lit(_)
    )
}

fn pat_ident(p: &Pat) -> Option<String> {
    match p {
        Pat::Ident(i) => Some(i.ident.to_string()),
        Pat::Type(t) => pat_ident(&t.pat),
        _ => None,
    }
}

/// The call an initialiser chain starts with (`Fixture::new(f).with_x()` gives `Fixture::new(f)`).
fn chain_call(e: &Expr) -> Option<&ExprCall> {
    match e {
        Expr::MethodCall(m) => chain_call(&m.receiver),
        Expr::Try(t) => chain_call(&t.expr),
        Expr::Paren(p) => chain_call(&p.expr),
        Expr::Reference(r) => chain_call(&r.expr),
        Expr::Call(c) => Some(c),
        _ => None,
    }
}

/// The events a `Fixture::frame` argument lists: `vec![..]` or `Vec::new()`. `None` for any other argument.
fn frame_events(e: &Expr) -> Option<Vec<Expr>> {
    match strip_all(e) {
        Expr::Macro(em) if em.mac.path.is_ident("vec") => {
            let parsed = em.mac.parse_body_with(syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated).ok()?;
            Some(parsed.into_iter().collect())
        }
        Expr::Call(c) if c.args.is_empty() && call_path(c) == ["Vec", "new"] => Some(Vec::new()),
        _ => None,
    }
}

fn call_path(c: &ExprCall) -> Vec<String> {
    match &*c.func {
        Expr::Path(p) => path_names(&p.path),
        _ => Vec::new(),
    }
}

fn tokens_contain(ts: TokenStream, hit: &dyn Fn(&str) -> bool) -> Option<String> {
    for tt in ts {
        match tt {
            TokenTree::Ident(i) if hit(&i.to_string()) => return Some(i.to_string()),
            TokenTree::Group(g) => {
                if let Some(found) = tokens_contain(g.stream(), hit) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

struct UiRefFinder(bool);

impl<'ast> Visit<'ast> for UiRefFinder {
    fn visit_type_reference(&mut self, r: &'ast syn::TypeReference) {
        if let Type::Path(p) = &*r.elem
            && p.path.segments.last().is_some_and(|s| s.ident == "Ui")
        {
            self.0 = true;
        }
        visit::visit_type_reference(self, r);
    }
}

impl<'a> Pass<'a> {
    fn new(src: &'a str, opts: &'a Options) -> Self {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        Self {
            src,
            starts,
            opts,
            prep: true,
            edits: Vec::new(),
            skips: Vec::new(),
            uses: Vec::new(),
            fixture_leaves: Vec::new(),
            blocked: BTreeSet::new(),
            tracked: HashSet::new(),
            renames: HashMap::new(),
            egui_names: HashSet::new(),
            no_touch: Vec::new(),
            consumed: HashSet::new(),
            chains_seen: HashSet::new(),
        }
    }

    fn run(&mut self, file: &syn::File) {
        self.visit_file(file);
        self.prep = false;
        self.emit_uses();
        self.emit_fixture_leaves();
        self.visit_file(file);
    }

    fn off(&self, lc: LineColumn) -> usize {
        let Some(&line_start) = self.starts.get(lc.line.wrapping_sub(1)) else { return self.src.len() };
        let line_end = self.src[line_start..].find('\n').map_or(self.src.len(), |i| line_start + i);
        let line = &self.src[line_start..line_end];
        line_start + line.char_indices().nth(lc.column).map_or(line.len(), |(i, _)| i)
    }

    fn range<T: quote::ToTokens>(&self, t: &T) -> (usize, usize) {
        let tokens: Vec<TokenTree> = t.to_token_stream().into_iter().collect();
        match (tokens.first(), tokens.last()) {
            (Some(first), Some(last)) => (self.off(first.span().start()), self.off(last.span().end())),
            _ => (0, 0),
        }
    }

    fn ident_range(&self, id: &proc_macro2::Ident) -> (usize, usize) {
        (self.off(id.span().start()), self.off(id.span().end()))
    }

    fn line_of(&self, offset: usize) -> usize {
        self.starts.partition_point(|&s| s <= offset)
    }

    fn text(&self, r: (usize, usize)) -> &'a str {
        &self.src[r.0..r.1]
    }

    fn edit(&mut self, start: usize, end: usize, text: String, rule: &'static str) {
        self.edit_rules(start, end, text, vec![rule]);
    }

    fn edit_rules(&mut self, start: usize, end: usize, text: String, rules: Vec<&'static str>) {
        if self.src[start..end] != *text {
            self.edits.push(Edit { start, end, text, rules });
        }
    }

    fn skip(&mut self, class: &'static str, at: usize, detail: impl Into<String>) {
        let line = self.line_of(at);
        self.skips.push(Skip { class, line, detail: detail.into() });
    }

    fn in_no_touch(&self, r: (usize, usize)) -> bool {
        self.no_touch.iter().any(|&(s, e)| r.0 >= s && r.1 <= e)
    }

    /// The module a rule's items are imported from.
    fn module_of(&self, item: &Item) -> Option<String> {
        if self.opts.inside_contract {
            return (item.number <= 4).then(|| "crate::contract".to_owned());
        }
        Some(if item.module.is_empty() { "craft_ui".to_owned() } else { format!("craft_ui::{}", item.module) })
    }

    fn emit_uses(&mut self) {
        let uses = std::mem::take(&mut self.uses);
        for u in &uses {
            let mut rest: Vec<&Leaf> = Vec::new();
            let mut groups: Vec<(String, Vec<String>)> = Vec::new();
            let mut rules: Vec<&'static str> = Vec::new();
            for leaf in &u.leaves {
                if leaf.glob {
                    self.skip("egui-glob", u.start, leaf.path.join("::") + "::*");
                    rest.push(leaf);
                    continue;
                }
                let mapped = (leaf.path.len() == 2)
                    .then(|| tables::item_named(&leaf.path[1]))
                    .flatten()
                    .filter(|it| !self.blocked.contains(it.egui))
                    .and_then(|it| self.module_of(it).map(|m| (it, m)));
                if let Some((it, module)) = mapped {
                    let text = match &leaf.alias {
                        Some(a) => format!("{} as {a}", it.new),
                        None => it.new.to_owned(),
                    };
                    if leaf.alias.is_none() && it.new != it.egui {
                        self.renames.insert(it.egui.to_owned(), it.new);
                    }
                    rules.push(it.rule);
                    match groups.iter_mut().find(|(m, _)| *m == module) {
                        Some((_, items)) => items.push(text),
                        None => groups.push((module, vec![text])),
                    }
                } else {
                    if leaf.path.len() == 2 && tables::item_named(&leaf.path[1]).is_some() {
                        if self.opts.inside_contract {
                            self.skip("contract-internal-target", u.start, leaf.path.join("::"));
                        }
                    } else if let Some(name) = leaf.path.last().filter(|_| leaf.path.len() > 1) {
                        let class = tables::skip_class(name).unwrap_or("egui-unmapped");
                        self.skip(class, u.start, leaf.path.join("::"));
                    }
                    if leaf.path.len() > 1 {
                        let local = leaf.alias.clone().unwrap_or_else(|| leaf.path[leaf.path.len() - 1].clone());
                        self.egui_names.insert(local);
                    }
                    rest.push(leaf);
                }
            }
            if rules.is_empty() {
                continue;
            }
            let mut stmts: Vec<String> = Vec::new();
            if !rest.is_empty() {
                stmts.push(format!("{}use {};", u.head, rest_tree(&rest)));
            }
            for (module, items) in &groups {
                let tree = if items.len() == 1 { format!("{module}::{}", items[0]) } else { format!("{module}::{{{}}}", items.join(", ")) };
                stmts.push(format!("{}use {tree};", u.head));
            }
            let sep = format!("\n{}", u.indent);
            self.edit_rules(u.start, u.end, stmts.join(&sep), rules);
        }
    }

    fn emit_fixture_leaves(&mut self) {
        for (start, end, aliased) in std::mem::take(&mut self.fixture_leaves) {
            if !aliased {
                self.renames.insert("Fixture".to_owned(), "Harness");
            }
            self.edit(start, end, "Harness".to_owned(), "23 Fixture");
        }
    }

    fn fixture_walk(&mut self, tree: &UseTree, prefix_last: &str) {
        match tree {
            UseTree::Path(p) => self.fixture_walk(&p.tree, &p.ident.to_string()),
            UseTree::Name(n) if n.ident == "Fixture" && prefix_last == "testing" => {
                let r = self.ident_range(&n.ident);
                self.fixture_leaves.push((r.0, r.1, false));
            }
            UseTree::Rename(n) if n.ident == "Fixture" && prefix_last == "testing" => {
                let r = self.ident_range(&n.ident);
                self.fixture_leaves.push((r.0, r.1, true));
            }
            UseTree::Group(g) => {
                for item in &g.items {
                    self.fixture_walk(item, prefix_last);
                }
            }
            _ => {}
        }
    }

    fn prep_path(&mut self, p: &Path) {
        let segs = path_names(p);
        let ty = if segs.len() >= 2 && segs[0] == "egui" && segs[1] == "ViewportCommand" {
            Some(1)
        } else if segs.first().is_some_and(|s| s == "ViewportCommand") {
            Some(0)
        } else {
            None
        };
        if let Some(variant) = ty.and_then(|i| segs.get(i + 1))
            && tables::variant_named(variant).is_none()
        {
            self.blocked.insert("ViewportCommand");
            let at = self.range(p).0;
            self.skip("viewport-command-other", at, segs.join("::"));
        }
    }

    fn prep_method(&mut self, mc: &ExprMethodCall) {
        let mut names = vec![mc.method.to_string()];
        let mut e: &Expr = &mc.receiver;
        loop {
            match e {
                Expr::MethodCall(m) => {
                    names.push(m.method.to_string());
                    e = &m.receiver;
                }
                Expr::Call(c) => {
                    let path = call_path(c);
                    if path.len() >= 2 && path[path.len() - 2] == "UiBuilder" && path[path.len() - 1] == "new" {
                        let root = self.range(c).0;
                        if self.chains_seen.insert(root) {
                            if names.iter().any(|n| n == "layout") {
                                self.blocked.insert("UiBuilder");
                                self.skip("uibuilder-layout", root, "UiBuilder chain with .layout(..)");
                            } else if names.iter().any(|n| n != "max_rect" && n != "id_salt") {
                                self.blocked.insert("UiBuilder");
                                self.skip("uibuilder-chain-other", root, names.iter().rev().cloned().collect::<Vec<_>>().join("."));
                            }
                        }
                    }
                    break;
                }
                _ => break,
            }
        }
    }

    fn edit_path(&mut self, p: &Path) {
        let segs: Vec<(String, (usize, usize))> = p.segments.iter().map(|s| (s.ident.to_string(), self.ident_range(&s.ident))).collect();
        let whole = self.range(p);
        if segs.is_empty() || self.in_no_touch(whole) {
            return;
        }
        let first = segs[0].0.as_str();
        if first == "egui" && segs.len() >= 2 {
            let name = segs[1].0.as_str();
            if let Some(item) = tables::item_named(name) {
                if self.blocked.contains(item.egui) {
                    return;
                }
                let Some(module) = self.module_of(item) else {
                    self.skip("contract-internal-target", whole.0, segs.iter().map(|s| s.0.as_str()).collect::<Vec<_>>().join("::"));
                    return;
                };
                let target = format!("{module}::{}", item.new);
                self.edit(segs[0].1.0, segs[1].1.1, target, item.rule);
                if name == "ViewportCommand"
                    && let Some((variant, r)) = segs.get(2)
                    && let Some(new) = tables::variant_named(variant)
                {
                    self.edit(r.0, r.1, new.to_owned(), item.rule);
                }
            } else if let Some(class) = tables::skip_class(name) {
                self.skip(class, whole.0, segs.iter().map(|s| s.0.as_str()).collect::<Vec<_>>().join("::"));
            } else {
                self.skip("egui-unmapped", whole.0, segs.iter().map(|s| s.0.as_str()).collect::<Vec<_>>().join("::"));
            }
            return;
        }
        if first == "epaint" {
            self.skip("raw-egui-paint", whole.0, segs.iter().map(|s| s.0.as_str()).collect::<Vec<_>>().join("::"));
            return;
        }
        if let Some(&new) = self.renames.get(first) {
            let item = tables::item_named(first);
            self.edit(segs[0].1.0, segs[0].1.1, new.to_owned(), item.map_or("23 Fixture", |i| i.rule));
            if first == "ViewportCommand"
                && let Some((variant, r)) = segs.get(1)
                && let Some(new) = tables::variant_named(variant)
            {
                self.edit(r.0, r.1, new.to_owned(), "10 ViewportCommand");
            }
        } else if self.egui_names.contains(first)
            && let Some(class) = tables::skip_class(first)
        {
            self.skip(class, whole.0, first.to_owned());
        }
        for i in 1..segs.len() {
            if segs[i].0 == "Fixture" && segs[i - 1].0 == "testing" {
                self.edit(segs[i].1.0, segs[i].1.1, "Harness".to_owned(), "23 Fixture");
            }
        }
    }

    fn edit_method(&mut self, mc: &ExprMethodCall) {
        let name = mc.method.to_string();
        let nargs = mc.args.len();
        let recv_ui = is_ui(&mc.receiver);
        let m = self.ident_range(&mc.method);
        if let Some(&(_, new, n, rule)) = tables::UI_METHODS.iter().find(|u| u.0 == name)
            && nargs == n
        {
            if recv_ui {
                self.edit(m.0, m.1, new.to_owned(), rule);
            } else {
                self.skip("ui-method-other-receiver", m.0, format!(".{name}(..)"));
            }
        } else if name == "viewport_id" && nargs == 0 {
            if is_ctx_expr(&mc.receiver) {
                self.edit(m.0, m.1, "window".to_owned(), "16 window");
            } else {
                self.skip("ui-method-other-receiver", m.0, ".viewport_id()");
            }
        }
        if recv_ui && tables::ui_widget_method(&name) {
            self.skip("egui-layout-widgets", m.0, format!(".{name}(..)"));
        }
        if name == "input" && (recv_ui || is_ctx_expr(&mc.receiver)) {
            self.skip("ctx-input-read", m.0, ".input(..)");
        }
        if let Some(class) = tables::method_class(&name) {
            self.skip(class, m.0, format!(".{name}(..)"));
        }
        if name == "clone"
            && nargs == 0
            && let Some(f) = cx_ctx_field(&mc.receiver)
        {
            let r = self.range(f);
            let base = self.text(self.range(&f.base)).to_owned();
            self.consumed.insert(r.0);
            self.edit(r.0, r.1, format!("{base}.context()"), "22 cx.ctx");
        }
        if let Some(recv) = single_ident(&mc.receiver)
            && self.tracked.contains(&recv)
        {
            if name == "frame" {
                let events = if nargs == 1 { frame_events(&mc.args[0]) } else { None };
                let end = self.range(mc).1;
                match events.as_deref() {
                    Some([]) => self.edit(m.0, end, "step()".to_owned(), "24 Fixture::frame"),
                    Some([event]) => {
                        let text = self.text(self.range(event)).to_owned();
                        self.edit(m.0, end, format!("send({text})"), "24 Fixture::frame");
                    }
                    _ => self.skip("fixture-frame-args", m.0, ".frame(..)"),
                }
            } else if name == "access" {
                self.skip("fixture-access", m.0, ".access(kind, t)");
            } else if tables::fixture_without_twin(&name) {
                self.skip("fixture-no-twin", m.0, format!(".{name}(..)"));
            }
        }
    }

    fn edit_call(&mut self, c: &ExprCall) {
        let path = call_path(c);
        let n = path.len();
        if n < 2 {
            return;
        }
        let (module, name) = (path[n - 2].as_str(), path[n - 1].as_str());
        let args: Vec<&Expr> = c.args.iter().collect();
        let whole = self.range(c);
        if module == "StyleCx" && name == "of_env" {
            if args.len() == 2 {
                let (a, b) = (self.range(args[0]).0, self.range(args[1]).0);
                self.edit(a, b, String::new(), "20 StyleCx::of_env");
            }
        } else if module == "Fixture" && name == "new" && (self.renames.contains_key("Fixture") || (n >= 3 && path[n - 3] == "testing")) {
            if args.len() == 1 {
                if let Expr::Path(p) = &*c.func
                    && let Some(seg) = p.path.segments.last()
                {
                    let r = self.ident_range(&seg.ident);
                    self.edit(r.0, r.1, "new_ui".to_owned(), "23 Fixture::new");
                }
            } else {
                self.skip("fixture-new-args", whole.0, "Fixture::new(..)");
            }
        } else if let Some(svc) = tables::service_named(module, name) {
            self.service(svc, &args, whole);
        } else if tables::is_hand_written(module, name) {
            self.skip("service-hand-written", whole.0, format!("{module}::{name}"));
        }
    }

    fn service(&mut self, svc: &Service, args: &[&Expr], whole: (usize, usize)) {
        let label = format!("{}::{}", svc.module, svc.name);
        if args.len() != svc.args {
            self.skip("service-arity", whole.0, label);
            return;
        }
        let first = strip_refs(args[0]);
        let (recv, rule_kind): (String, Option<Kind>) = if let Some(ui) = ui_ctx_call(first) {
            (self.text(self.range(ui)).to_owned(), Some(Kind::Ui))
        } else if let Some(f) = cx_ctx_field(first) {
            self.consumed.insert(self.range(f).0);
            (self.text(self.range(&f.base)).to_owned(), Some(Kind::UiCx))
        } else {
            (String::new(), None)
        };
        let call = |recv: &str, rest: &str| format!("{recv}.{}({rest})", svc.new);
        let rest = if args.len() > 1 {
            if svc.secs {
                format!("std::time::Duration::from_secs_f64({})", self.text(self.range(args[1])))
            } else {
                let last = args[args.len() - 1];
                self.src[self.range(args[1]).0..self.range(last).1].to_owned()
            }
        } else {
            String::new()
        };
        let text = match (svc.kind, rule_kind) {
            (Kind::Ui | Kind::UiCx, Some(Kind::Ui)) => call(&recv, &rest),
            (Kind::UiCx, Some(Kind::UiCx)) => call(&recv, &rest),
            (Kind::Ctx, Some(Kind::Ui)) => call(&format!("{recv}.ctx()"), &rest),
            (Kind::Ctx, Some(Kind::UiCx)) => call(&format!("{recv}.context()"), &rest),
            (Kind::Ctx, None) => {
                let text = self.text(self.range(first));
                if needs_parens(first) { call(&format!("({text})"), &rest) } else { call(text, &rest) }
            }
            (Kind::Ui, Some(Kind::UiCx)) => {
                self.skip("cx-receiver-ui-only", whole.0, label);
                return;
            }
            _ => {
                self.skip("service-receiver-unknown", whole.0, label);
                return;
            }
        };
        self.edit(whole.0, whole.1, text, svc.rule);
    }
}

/// The text after `use` of the leaves that stay on egui.
fn rest_tree(rest: &[&Leaf]) -> String {
    let one = |l: &Leaf, root: bool| {
        let sub = l.path[1..].join("::");
        let mut s = match (root, sub.is_empty()) {
            (true, true) => "egui".to_owned(),
            (true, false) => format!("egui::{sub}"),
            (false, true) => "self".to_owned(),
            (false, false) => sub,
        };
        if l.glob {
            s = if s == "self" { "*".to_owned() } else { format!("{s}::*") };
        }
        if let Some(a) = &l.alias {
            s = format!("{s} as {a}");
        }
        s
    };
    if let [l] = rest { one(l, true) } else { format!("egui::{{{}}}", rest.iter().map(|l| one(l, false)).collect::<Vec<_>>().join(", ")) }
}

impl<'ast> Visit<'ast> for Pass<'_> {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        if !self.prep {
            return;
        }
        self.fixture_walk(&item.tree, "");
        let UseTree::Path(root) = &item.tree else { return };
        if root.ident != "egui" {
            return;
        }
        let (start, end) = self.range(item);
        let use_start = self.off(item.use_token.span.start());
        let line_start = self.starts[self.line_of(start) - 1];
        let indent: String = self.src[line_start..start].chars().take_while(|c| c.is_whitespace()).collect();
        let mut leaves = Vec::new();
        flatten(&item.tree, &mut Vec::new(), &mut leaves);
        self.uses.push(UseInfo { start, end, head: self.src[start..use_start].to_owned(), indent, leaves });
    }

    fn visit_path(&mut self, p: &'ast Path) {
        if self.prep {
            self.prep_path(p);
        } else {
            self.edit_path(p);
        }
        visit::visit_path(self, p);
    }

    fn visit_expr_method_call(&mut self, mc: &'ast ExprMethodCall) {
        if self.prep {
            self.prep_method(mc);
        } else {
            self.edit_method(mc);
        }
        visit::visit_expr_method_call(self, mc);
    }

    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        if !self.prep {
            self.edit_call(c);
        }
        visit::visit_expr_call(self, c);
    }

    fn visit_expr_reference(&mut self, r: &'ast ExprReference) {
        if !self.prep
            && r.mutability.is_none()
            && let Some(f) = cx_ctx_field(&r.expr)
        {
            let whole = self.range(r);
            let field = self.range(f);
            let base = self.text(self.range(&f.base)).to_owned();
            self.consumed.insert(field.0);
            self.edit(whole.0, whole.1, format!("{base}.context()"), "22 cx.ctx");
        }
        visit::visit_expr_reference(self, r);
    }

    fn visit_expr_field(&mut self, f: &'ast ExprField) {
        if !self.prep && matches!(&f.member, Member::Named(id) if id == "ctx") && !self.consumed.contains(&self.range(f).0) {
            let at = self.range(f).0;
            if is_cx_base(&f.base) {
                self.skip("cx-ctx-other", at, "cx.ctx");
            } else {
                self.skip("ctx-field-other-receiver", at, "<expr>.ctx");
            }
        }
        visit::visit_expr_field(self, f);
    }

    fn visit_item_struct(&mut self, s: &'ast ItemStruct) {
        if !self.prep {
            for field in &s.fields {
                let mut finder = UiRefFinder(false);
                finder.visit_type(&field.ty);
                if finder.0 {
                    let r = self.range(&field.ty);
                    self.no_touch.push(r);
                    self.skip("ui-struct-field", r.0, format!("{}: {}", s.ident, self.text(r)));
                }
            }
        }
        visit::visit_item_struct(self, s);
    }

    fn visit_local(&mut self, l: &'ast Local) {
        if self.prep
            && let Some(name) = pat_ident(&l.pat)
            && let Some(init) = &l.init
            && let Some(call) = chain_call(&init.expr)
        {
            let p = call_path(call);
            let n = p.len();
            if n >= 2 && matches!(p[n - 2].as_str(), "Fixture" | "Harness") && matches!(p[n - 1].as_str(), "new" | "new_ui") {
                self.tracked.insert(name);
            }
        }
        visit::visit_local(self, l);
    }

    fn visit_fn_arg(&mut self, a: &'ast FnArg) {
        if self.prep
            && let FnArg::Typed(t) = a
            && let Some(name) = pat_ident(&t.pat)
            && tokens_contain(t.ty.to_token_stream(), &|s| s == "Fixture" || s == "Harness").is_some()
        {
            self.tracked.insert(name);
        }
        visit::visit_fn_arg(self, a);
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if !self.prep {
            let renames: Vec<String> = self.renames.keys().cloned().collect();
            let hit = |s: &str| {
                matches!(s, "egui" | "painter" | "painter_at" | "is_sizing_pass" | "allocate_space" | "viewport_id" | "Fixture")
                    || renames.iter().any(|r| r == s)
            };
            if let Some(found) = tokens_contain(m.tokens.clone(), &hit) {
                let at = self.range(m).0;
                self.skip("macro-tokens", at, format!("{}! mentions `{found}`", m.path.to_token_stream()));
            }
        }
        visit::visit_macro(self, m);
    }
}
