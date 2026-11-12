//! Prelude checks that need no GPU.

use crate::{COLOR, MATH, Module, TRANSFER, assemble};

/// A WGSL edit that doesn't parse or validate. CI has no GPU, so the GPU target skips there and
/// only this check sees it.
#[test]
fn modules_validate() {
    let all: [&'static Module; 3] = [&MATH, &COLOR, &TRANSFER];
    for m in all {
        let src = assemble(&[m], &[]);
        let module = naga::front::wgsl::parse_str(&src).unwrap_or_else(|e| panic!("{}: {}", m.name, e.emit_to_string(&src)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{}: {e:?}", m.name));
    }
}

/// A module reached through several paths is included once, after what it depends on: WGSL
/// rejects a function declared twice.
#[test]
fn assembly_includes_each_module_once_after_its_dependencies() {
    let src = assemble(&[&TRANSFER, &COLOR, &MATH], &["// entry"]);
    let at = |name: &str| src.find(&format!("// ---- {name}\n")).unwrap_or_else(|| panic!("{name} missing"));
    assert_eq!(src.matches("// ---- craft-wgsl/math\n").count(), 1);
    assert!(at("craft-wgsl/math") < at("craft-wgsl/color") && at("craft-wgsl/color") < at("craft-wgsl/transfer"));
    assert!(src.ends_with("// entry\n"));
}
