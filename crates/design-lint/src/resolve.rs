//! The `layer/import` and `layer/internal` rules.
//!
//! [`Index`] holds each crate's module tree with the names every module defines, re-exports (`pub use`)
//! and globs. A path is resolved through that index to the module that defines the item, and the layer
//! map judges that module, so a re-export in a low layer does not hide an item from a high one.
//! Visibility is not checked: rustc already does that.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use proc_macro2::LineColumn;
use syn::spanned::Spanned as _;
use syn::visit::{self, Visit};
use syn::{Attribute, Item, ItemUse, UseTree};

use crate::Error;
use crate::layers::{LayerMaps, finding, relative};
use crate::report::Finding;
use crate::rules::is_test_attr;

/// A path that names an item in a higher layer than the module that uses it.
pub const IMPORT: &str = "layer/import";
/// A path from layers above an internal module's range into that module.
pub const INTERNAL: &str = "layer/internal";

/// Longest chain of re-exports and globs followed before a path counts as unresolvable.
const MAX_DEPTH: u8 = 16;

#[derive(Debug)]
enum Name {
    Defined,
    Module,
    Use(Vec<String>),
}

pub(crate) struct Module {
    pub(crate) path: Vec<String>,
    pub(crate) file: PathBuf,
    items: Vec<Item>,
    names: HashMap<String, Name>,
    globs: Vec<Vec<String>>,
}

pub(crate) struct CrateTree {
    pub(crate) name: String,
    pub(crate) modules: BTreeMap<Vec<String>, Module>,
}

/// The module that defines the item a path names, or the module itself when the path names a module.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Resolved {
    krate: String,
    module: Vec<String>,
    is_module: bool,
}

impl Resolved {
    fn shown(&self) -> String {
        std::iter::once(self.krate.as_str()).chain(self.module.iter().map(String::as_str)).collect::<Vec<_>>().join("::")
    }
}

/// The module trees of the crates that have a layer map, keyed by Rust identifier.
#[derive(Default)]
pub(crate) struct Index {
    pub(crate) trees: BTreeMap<String, CrateTree>,
}

fn io_error(path: &Path, message: String) -> Error {
    Error::Io { path: path.to_owned(), source: std::io::Error::new(std::io::ErrorKind::InvalidData, message) }
}

fn is_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Fn(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        Item::Const(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        _ => &[],
    }
}

impl Index {
    /// Parses the module tree of each `(package name, lib.rs path)`.
    pub(crate) fn build(crates: &[(String, PathBuf)]) -> Result<Self, Error> {
        let mut index = Self::default();
        for (name, root_file) in crates {
            let tree = CrateTree::load(name, root_file)?;
            index.trees.insert(name.replace('-', "_"), tree);
        }
        Ok(index)
    }

    fn module(&self, krate: &str, path: &[String]) -> Option<&Module> {
        self.trees.get(krate)?.modules.get(path)
    }

    fn resolve(&self, krate: &str, from: &[String], segs: &[String], depth: u8) -> Option<Resolved> {
        if depth > MAX_DEPTH || segs.is_empty() {
            return None;
        }
        let (krate, module, rest) = self.start(krate, from, segs)?;
        self.walk(&krate, &module, rest, depth)
    }

    /// The crate and module a path's first segment leads to, and the segments left.
    fn start<'s>(&self, krate: &str, from: &[String], segs: &'s [String]) -> Option<(String, Vec<String>, &'s [String])> {
        let here = |rest| Some((krate.to_owned(), from.to_vec(), rest));
        match segs[0].as_str() {
            "crate" => Some((krate.to_owned(), Vec::new(), &segs[1..])),
            "self" => here(&segs[1..]),
            "super" => {
                let mut module = from.to_vec();
                let mut rest = segs;
                while rest.first().is_some_and(|s| s == "super") {
                    module.pop()?;
                    rest = &rest[1..];
                }
                Some((krate.to_owned(), module, rest))
            }
            first => {
                let module = self.module(krate, from)?;
                if module.names.contains_key(first) {
                    here(segs)
                } else if self.trees.contains_key(first) {
                    Some((first.to_owned(), Vec::new(), &segs[1..]))
                } else if !module.globs.is_empty() {
                    here(segs)
                } else {
                    None
                }
            }
        }
    }

    fn walk(&self, krate: &str, module_path: &[String], segs: &[String], depth: u8) -> Option<Resolved> {
        let resolved = |module: &[String], is_module| Some(Resolved { krate: krate.to_owned(), module: module.to_vec(), is_module });
        let Some((first, rest)) = segs.split_first() else {
            return resolved(module_path, true);
        };
        let module = self.module(krate, module_path)?;
        match module.names.get(first) {
            Some(Name::Module) => {
                let mut child = module_path.to_vec();
                child.push(first.clone());
                self.walk(krate, &child, rest, depth + 1)
            }
            Some(Name::Defined) => resolved(module_path, false),
            Some(Name::Use(target)) => {
                let target = self.resolve(krate, module_path, target, depth + 1)?;
                if rest.is_empty() || !target.is_module { Some(target) } else { self.walk(&target.krate, &target.module, rest, depth + 1) }
            }
            None => module.globs.iter().find_map(|glob| {
                let target = self.resolve(krate, module_path, glob, depth + 1)?;
                if target.is_module { self.walk(&target.krate, &target.module, segs, depth + 1) } else { Some(target) }
            }),
        }
    }

    /// Every item and module a glob import of `target` brings in, resolved to where it is defined.
    fn expand(&self, target: &Resolved, depth: u8) -> Vec<Resolved> {
        let Some(module) = self.module(&target.krate, &target.module).filter(|_| target.is_module && depth <= MAX_DEPTH) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (name, kind) in &module.names {
            let resolved = match kind {
                Name::Defined => Some(Resolved { krate: target.krate.clone(), module: target.module.clone(), is_module: false }),
                Name::Module => {
                    let mut child = target.module.clone();
                    child.push(name.clone());
                    Some(Resolved { krate: target.krate.clone(), module: child, is_module: true })
                }
                Name::Use(path) => self.resolve(&target.krate, &target.module, path, depth + 1),
            };
            out.extend(resolved);
        }
        for glob in &module.globs {
            if let Some(inner) = self.resolve(&target.krate, &target.module, glob, depth + 1) {
                out.extend(self.expand(&inner, depth + 1));
                out.push(inner);
            }
        }
        out
    }
}

impl CrateTree {
    fn load(name: &str, root_file: &Path) -> Result<Self, Error> {
        let mut tree = Self { name: name.to_owned(), modules: BTreeMap::new() };
        let items = parse_items(root_file)?;
        let dir = root_file.parent().unwrap_or(Path::new("")).to_owned();
        tree.add(Vec::new(), root_file, items, &dir)?;
        Ok(tree)
    }

    /// Adds the module at `path` whose items are `items`, and its children; `child_dir` is where `mod x;` files live.
    fn add(&mut self, path: Vec<String>, file: &Path, items: Vec<Item>, child_dir: &Path) -> Result<(), Error> {
        let items: Vec<Item> = items.into_iter().filter(|i| !is_test(item_attrs(i))).collect();
        let mut names = HashMap::new();
        let mut globs = Vec::new();
        let mut children = Vec::new();
        for item in &items {
            match item {
                Item::Use(u) => collect_use(u, &mut names, &mut globs),
                Item::Mod(m) => {
                    names.insert(m.ident.to_string(), Name::Module);
                    children.push(m);
                }
                other => {
                    if let Some(ident) = defined_name(other) {
                        names.insert(ident, Name::Defined);
                    }
                }
            }
        }
        for m in children {
            let mut child_path = path.clone();
            child_path.push(m.ident.to_string());
            let dir = child_dir.join(m.ident.to_string());
            match &m.content {
                Some((_, inner)) => self.add(child_path, file, inner.clone(), &dir)?,
                None => {
                    let (child_file, grand_dir) = module_file(child_dir, &m.ident.to_string(), file)?;
                    let inner = parse_items(&child_file)?;
                    self.add(child_path, &child_file, inner, &grand_dir)?;
                }
            }
        }
        self.modules.insert(path.clone(), Module { path, file: file.to_owned(), items, names, globs });
        Ok(())
    }
}

fn parse_items(file: &Path) -> Result<Vec<Item>, Error> {
    let src = std::fs::read_to_string(file).map_err(|source| Error::Io { path: file.to_owned(), source })?;
    syn::parse_file(&src).map(|f| f.items).map_err(|e| io_error(file, format!("does not parse: {e}")))
}

/// `dir/name.rs` or `dir/name/mod.rs`, and the directory its own `mod x;` files live in.
fn module_file(dir: &Path, name: &str, parent: &Path) -> Result<(PathBuf, PathBuf), Error> {
    let flat = dir.join(format!("{name}.rs"));
    let nested = dir.join(name).join("mod.rs");
    if flat.is_file() {
        Ok((flat, dir.join(name)))
    } else if nested.is_file() {
        Ok((nested, dir.join(name)))
    } else {
        Err(io_error(parent, format!("`mod {name};` has no file {} or {}", flat.display(), nested.display())))
    }
}

fn defined_name(item: &Item) -> Option<String> {
    let ident = match item {
        Item::Fn(i) => &i.sig.ident,
        Item::Struct(i) => &i.ident,
        Item::Enum(i) => &i.ident,
        Item::Union(i) => &i.ident,
        Item::Trait(i) => &i.ident,
        Item::TraitAlias(i) => &i.ident,
        Item::Type(i) => &i.ident,
        Item::Const(i) => &i.ident,
        Item::Static(i) => &i.ident,
        _ => return None,
    };
    Some(ident.to_string())
}

/// A leaf of a `use` tree: a path with the name it binds, or a glob.
enum Leaf {
    Path(Vec<String>, Option<String>),
    Glob(Vec<String>),
}

fn leaves(tree: &UseTree, prefix: &mut Vec<String>, out: &mut Vec<Leaf>) {
    match tree {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            leaves(&p.tree, prefix, out);
            prefix.pop();
        }
        UseTree::Name(n) => out.push(leaf(prefix, &n.ident.to_string(), None)),
        UseTree::Rename(r) => out.push(leaf(prefix, &r.ident.to_string(), Some(r.rename.to_string()))),
        UseTree::Glob(_) => out.push(Leaf::Glob(prefix.clone())),
        UseTree::Group(g) => g.items.iter().for_each(|t| leaves(t, prefix, out)),
    }
}

/// `use a::b::{self}` binds `b` to the path `a::b`.
fn leaf(prefix: &[String], ident: &str, alias: Option<String>) -> Leaf {
    let mut segs = prefix.to_vec();
    if ident != "self" || segs.is_empty() {
        segs.push(ident.to_owned());
    }
    Leaf::Path(segs, alias)
}

fn collect_use(item: &ItemUse, names: &mut HashMap<String, Name>, globs: &mut Vec<Vec<String>>) {
    let mut out = Vec::new();
    leaves(&item.tree, &mut Vec::new(), &mut out);
    for l in out {
        match l {
            Leaf::Glob(segs) => globs.push(segs),
            Leaf::Path(segs, alias) => {
                let bound = alias.or_else(|| segs.last().cloned()).unwrap_or_default();
                if bound != "_" {
                    names.insert(bound, Name::Use(segs));
                }
            }
        }
    }
}

struct Judge<'a> {
    index: &'a Index,
    maps: &'a LayerMaps,
    krate: &'a str,
    module: &'a Module,
    layer: u8,
    shown: PathBuf,
    out: Vec<Finding>,
    seen: HashSet<(usize, usize, &'static str, String)>,
}

impl Judge<'_> {
    fn judge_path(&mut self, segs: &[String], at: LineColumn) {
        if let Some(r) = self.index.resolve(self.krate, &self.module.path, segs, 0) {
            self.judge(&r, &segs.join("::"), at);
        }
    }

    fn judge_glob(&mut self, segs: &[String], at: LineColumn) {
        let Some(target) = self.index.resolve(self.krate, &self.module.path, segs, 0) else { return };
        let spelled = format!("{}::*", segs.join("::"));
        for r in self.index.expand(&target, 0) {
            self.judge(&r, &spelled, at);
        }
        self.judge(&target, &spelled, at);
    }

    fn judge(&mut self, r: &Resolved, spelled: &str, at: LineColumn) {
        let Some(info) = self.maps.get_by_ident(&r.krate).and_then(|c| c.layer_info(&r.module)) else { return };
        let (rule, message) = if info.layer > self.layer {
            (IMPORT, format!("`{spelled}` is defined in `{}` (layer {}); layer {} may not import a higher layer", r.shown(), info.layer, self.layer))
        } else if let Some((lo, hi)) = info.internal.filter(|&(_, hi)| self.layer > hi) {
            (INTERNAL, format!("`{spelled}` is defined in `{}`, internal to layers {lo} to {hi}; layer {} may not use it", r.shown(), self.layer))
        } else {
            return;
        };
        if self.seen.insert((at.line, at.column, rule, r.shown())) {
            let mut f = Finding::new(rule, at.line, at.column + 1, at.line, message);
            f.path.clone_from(&self.shown);
            f.area = self.krate.to_owned();
            self.out.push(f);
        }
    }
}

impl<'ast> Visit<'ast> for Judge<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if !is_test(item_attrs(item)) {
            visit::visit_item(self, item);
        }
    }

    /// Nested modules are judged on their own, with their own layer.
    fn visit_item_mod(&mut self, _: &'ast syn::ItemMod) {}

    fn visit_attribute(&mut self, _: &'ast Attribute) {}

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let at = item.span().start();
        let mut out = Vec::new();
        leaves(&item.tree, &mut Vec::new(), &mut out);
        for l in out {
            match l {
                Leaf::Path(segs, _) => self.judge_path(&segs, at),
                Leaf::Glob(segs) => self.judge_glob(&segs, at),
            }
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if path.segments.len() >= 2 {
            let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            self.judge_path(&segs, path.span().start());
        }
        visit::visit_path(self, path);
    }
}

/// `layer/import` and `layer/internal` findings for every crate in `index` that has a layer map.
pub(crate) fn check_imports(index: &Index, maps: &LayerMaps, root: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    for (ident, tree) in &index.trees {
        let Some(layers) = maps.get(&tree.name) else { continue };
        for module in tree.modules.values() {
            let Some(info) = layers.layer_info(&module.path) else { continue };
            let mut judge = Judge {
                index,
                maps,
                krate: ident,
                module,
                layer: info.layer,
                shown: relative(root, &module.file),
                out: Vec::new(),
                seen: HashSet::new(),
            };
            for item in &module.items {
                judge.visit_item(item);
            }
            out.append(&mut judge.out);
        }
    }
    out
}

/// `layer/unmapped` findings: modules no map entry covers in a crate with several layers.
pub(crate) fn check_coverage(index: &Index, maps: &LayerMaps, root: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    for tree in index.trees.values() {
        let Some(layers) = maps.get(&tree.name) else { continue };
        for module in tree.modules.values().filter(|m| layers.layer_of(&m.path).is_none()) {
            let shown = if module.path.is_empty() { "crate".to_owned() } else { module.path.join("::") };
            let message = format!("module `{shown}` of `{}` is in no layer-map entry", tree.name);
            out.push(finding(crate::layers::UNMAPPED, relative(root, &module.file), message));
        }
    }
    out
}
