// SPDX-License-Identifier: GPL-3.0-or-later
// Generate the extern "C" layer from the interface definition at build time.
use std::path::Path;

fn main() {
    let idl_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../idl/press.toml");
    println!("cargo:rerun-if-changed={}", idl_path.display());
    let text = std::fs::read_to_string(&idl_path).expect("read idl/press.toml");
    let idl = press_idl::Idl::parse(&text).unwrap_or_else(|e| panic!("idl/press.toml: {e}"));
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("abi.rs");
    std::fs::write(out, press_idl::rust::generate(&idl)).unwrap();
}
