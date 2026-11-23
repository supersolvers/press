// SPDX-License-Identifier: GPL-3.0-or-later
//! Regenerate the checked-in C header and bindings from `idl/press.toml`
//! with `cargo run -p press-idl`. Add `-- --check` to check for stale
//! output without writing files.

use press_idl::Idl;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("idl/press.toml");
    let text = match std::fs::read_to_string(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", source.display());
            return ExitCode::FAILURE;
        }
    };
    let idl = match Idl::parse(&text) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("idl/press.toml: {e}");
            return ExitCode::FAILURE;
        }
    };
    let outputs = [
        ("include/press.h", press_idl::c::generate(&idl)),
        ("bindings/python/press.py", press_idl::python::generate(&idl)),
        ("bindings/swift/Sources/PressCore/PressCore.swift", press_idl::swift::generate(&idl)),
    ];
    let mut stale = false;
    for (rel, content) in outputs {
        let path = root.join(rel);
        if check {
            if std::fs::read_to_string(&path).ok().as_deref() != Some(content.as_str()) {
                eprintln!("{rel} is out of date. Regenerate it with cargo run -p press-idl.");
                stale = true;
            }
        } else {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
            println!("wrote {rel}");
        }
    }
    if stale { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
