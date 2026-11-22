// SPDX-License-Identifier: GPL-3.0-or-later
// Generate the extern "C" layer from the interface definition at build time.
use std::path::Path;

fn main() {
    let idl_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../idl/press.toml");
    println!("cargo:rerun-if-changed={}", idl_path.display());
    let text = std::fs::read_to_string(&idl_path).expect("read idl/press.toml");
    let idl = press_idl::Idl::parse(&text).unwrap_or_else(|e| panic!("idl/press.toml: {e}"));
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("abi.rs");
    std::fs::write(&out, press_idl::rust::generate(&idl)).unwrap();

    // The library is named for the interface's version, so a program made
    // for another version is not handed this one. A program linked against
    // the build here asks for it by that name, so the name is put beside
    // the library: target/PROFILE, the folder that holds OUT_DIR's `build`.
    let soname = format!("libpress.so.{}", idl.library.abi_version);
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{soname}");
    let build = out.ancestors().find(|p| p.file_name().is_some_and(|n| n == "build"));
    if let Some(profile) = build.and_then(|b| b.parent()) {
        let link = profile.join(&soname);
        if std::fs::read_link(&link).is_err() {
            let _ = std::os::unix::fs::symlink("libpress.so", &link);
        }
    }
}
