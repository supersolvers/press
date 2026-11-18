// SPDX-License-Identifier: GPL-3.0-or-later
//! C header generator.

use crate::{shout, snake, Idl, Ty};
use std::fmt::Write;

fn ctype(idl: &Idl, ty: &Ty) -> String {
    let p = &idl.library.prefix;
    match ty {
        Ty::Bool => "bool".into(),
        Ty::I32 => "int32_t".into(),
        Ty::U32 => "uint32_t".into(),
        Ty::Str => "const char *".into(),
        Ty::Bytes => "const uint8_t *".into(),
        Ty::Callback => format!("{p}_callback_fn"),
        Ty::Enum(n) | Ty::Handle(n) => format!("{p}_{}_t", snake(n)),
    }
}

fn params(idl: &Idl, args: &[(String, Ty)]) -> Vec<String> {
    let mut out = Vec::new();
    for (name, ty) in args {
        match ty {
            Ty::Str => {
                out.push(format!("const char *{name}"));
                out.push(format!("size_t {name}_len"));
            }
            Ty::Bytes => {
                out.push(format!("const uint8_t *{name}"));
                out.push(format!("size_t {name}_len"));
            }
            Ty::Callback => {
                let p = &idl.library.prefix;
                out.push(format!("{p}_callback_fn {name}"));
                out.push(format!("void *{name}_data"));
                out.push(format!("{p}_destroy_fn {name}_destroy"));
            }
            _ => out.push(format!("{} {name}", ctype(idl, ty))),
        }
    }
    out
}

fn comment(out: &mut String, doc: &str) {
    if !doc.is_empty() {
        writeln!(out, "/** {doc} */").unwrap();
    }
}

pub fn generate(idl: &Idl) -> String {
    let p = &idl.library.prefix;
    let up = p.to_uppercase();
    let mut o = String::new();
    writeln!(o, "/* Generated from idl/press.toml by press-idl. Do not edit. */").unwrap();
    writeln!(o, "/* SPDX-License-Identifier: GPL-3.0-or-later */").unwrap();
    writeln!(o, "/**\n * {}\n *", idl.library.doc).unwrap();
    writeln!(o, " * Every function returns a {p}_status_t; results come back through the last").unwrap();
    writeln!(o, " * argument. Objects are opaque 64-bit handles; 0 is never a valid handle.").unwrap();
    writeln!(o, " * Strings are UTF-8 with an explicit length and are only borrowed for the").unwrap();
    writeln!(o, " * call. All calls must come from the thread that called {p}_app_init.\n *").unwrap();
    writeln!(o, " * A string passed to one of your handlers is likewise only valid until the").unwrap();
    writeln!(o, " * handler returns; copy it to keep it. It is NUL-terminated as a convenience,").unwrap();
    writeln!(o, " * though the length is what counts.\n *").unwrap();
    writeln!(o, " * A function that returns a string copies it into `out`, a buffer of").unwrap();
    writeln!(o, " * `out_cap` bytes that you supply, and stores the string's full length in").unwrap();
    writeln!(o, " * bytes in `*out_len`. The copy is NUL-terminated and is cut short, on a").unwrap();
    writeln!(o, " * character boundary, if the buffer is too small: the whole string arrived").unwrap();
    writeln!(o, " * when *out_len < out_cap. Pass NULL and 0 to ask for the length alone.\n */").unwrap();
    writeln!(o, "#ifndef {up}_H\n#define {up}_H\n").unwrap();
    writeln!(o, "#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <string.h>\n").unwrap();
    writeln!(o, "#ifdef __cplusplus\nextern \"C\" {{\n#endif\n").unwrap();
    writeln!(o, "#if defined(_WIN32)\n#define {up}_API __declspec(dllimport)\n#else\n#define {up}_API\n#endif\n").unwrap();
    writeln!(o, "#define {up}_ABI_VERSION {}\n", idl.library.abi_version).unwrap();
    writeln!(o, "/** Pass a NUL-terminated C string where a (pointer, length) pair is expected. */").unwrap();
    writeln!(o, "#define {up}_S(s) (s), strlen(s)\n").unwrap();

    for e in &idl.enums {
        let es = snake(&e.name);
        comment(&mut o, &e.doc);
        writeln!(o, "typedef {} {p}_{es}_t;", if e.flags { "uint32_t" } else { "int32_t" }).unwrap();
        writeln!(o, "enum {{").unwrap();
        for v in &e.values {
            let doc = if v.doc.is_empty() { String::new() } else { format!(" /**< {} */", v.doc) };
            writeln!(o, "    {up}_{}_{} = {},{doc}", shout(&e.name), shout(&v.name), v.value).unwrap();
        }
        writeln!(o, "}};\n").unwrap();
    }

    // A readable name for each status, usable without linking anything extra.
    if let Some(status) = idl.enums.iter().find(|e| e.name == "Status") {
        writeln!(o, "/** A short description of a status code. */").unwrap();
        writeln!(o, "static inline const char *{p}_status_message({p}_status_t status) {{").unwrap();
        writeln!(o, "    switch (status) {{").unwrap();
        for v in &status.values {
            writeln!(o, "    case {up}_STATUS_{}: return \"{}\";", shout(&v.name), v.doc).unwrap();
        }
        writeln!(o, "    default: return \"unknown status\";\n    }}\n}}\n").unwrap();
    }

    writeln!(o, "/** An opaque reference to a press object. */").unwrap();
    writeln!(o, "typedef uint64_t {p}_handle_t;").unwrap();
    for c in idl.classes.iter().filter(|c| !c.is_static) {
        writeln!(o, "typedef {p}_handle_t {p}_{}_t;", snake(&c.name)).unwrap();
    }
    writeln!(o, "\n/** Called when a callback is dropped, so its userdata can be released. */").unwrap();
    writeln!(o, "typedef void (*{p}_destroy_fn)(void *userdata);").unwrap();
    writeln!(o, "/** A piece of work handed to press to be called later. It is passed as three").unwrap();
    writeln!(o, " * arguments: the function, its userdata, and an optional destroy notifier").unwrap();
    writeln!(o, " * that runs once the function has been called or will never be. */").unwrap();
    writeln!(o, "typedef void (*{p}_callback_fn)(void *userdata);\n").unwrap();

    for c in &idl.classes {
        let cs = snake(&c.name);
        let base = c.base.as_ref().map(|b| format!(" (a {b})")).unwrap_or_default();
        writeln!(o, "/* ---- {}{base}: {} */\n", c.name, c.doc).unwrap();
        for f in idl.functions(c) {
            comment(&mut o, f.doc);
            let mut ps = Vec::new();
            if f.receiver.is_some() {
                ps.push(format!("{p}_{cs}_t self"));
            }
            ps.extend(params(idl, &f.args));
            match &f.ret {
                // see the header comment for the string-result convention
                Some(Ty::Str) => ps.extend(["char *out".to_string(), "size_t out_cap".to_string(), "size_t *out_len".to_string()]),
                Some(r) => ps.push(format!("{} *out", ctype(idl, r).trim_end())),
                None => {}
            }
            let ps = if ps.is_empty() { "void".to_string() } else { ps.join(", ") };
            writeln!(o, "{up}_API {p}_status_t {p}_{}({ps});\n", f.name).unwrap();
        }
        for e in &c.events {
            let fn_ty = format!("{p}_{cs}_{}_fn", e.name);
            let mut ps = vec![format!("{p}_{cs}_t self")];
            let args: Vec<(String, Ty)> = e.args.iter().map(|a| (a.name.clone(), idl.ty(&a.ty).unwrap())).collect();
            ps.extend(params(idl, &args));
            ps.push("void *userdata".into());
            comment(&mut o, &e.doc);
            writeln!(o, "typedef void (*{fn_ty})({});", ps.join(", ")).unwrap();
            writeln!(o, "/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */").unwrap();
            writeln!(
                o,
                "{up}_API {p}_status_t {p}_{cs}_on_{}({p}_{cs}_t self, {fn_ty} callback, void *userdata, {p}_destroy_fn destroy);\n",
                e.name
            )
            .unwrap();
        }
    }

    writeln!(o, "#ifdef __cplusplus\n}}\n#endif\n\n#endif /* {up}_H */").unwrap();
    o
}
