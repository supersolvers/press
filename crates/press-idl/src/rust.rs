// SPDX-License-Identifier: GPL-3.0-or-later
//! Generator for the Rust export layer: the `extern "C"` functions that make
//! up the ABI. Each one validates its arguments and forwards to a hand-written
//! function of the same name in `crate::api`, so the ABI cannot drift from the
//! interface definition without a compile error.

use crate::{camel, shout, snake, Idl, Ty};
use std::fmt::Write;

fn ffi_type(ty: &Ty) -> &'static str {
    match ty {
        Ty::Bool => "bool",
        Ty::I32 | Ty::Enum(_) => "i32",
        Ty::U32 => "u32",
        Ty::Str | Ty::Bytes | Ty::Callback => unreachable!(),
        Ty::Handle(_) => "u64",
    }
}

/// The type the hand-written `api` function sees.
fn api_type(ty: &Ty) -> String {
    match ty {
        Ty::Bool => "bool".into(),
        Ty::I32 => "i32".into(),
        Ty::U32 => "u32".into(),
        Ty::Str => "&str".into(),
        Ty::Bytes => "&[u8]".into(),
        Ty::Enum(n) => n.clone(),
        Ty::Handle(_) => "Handle".into(),
        Ty::Callback => "Work".into(),
    }
}

pub fn generate(idl: &Idl) -> String {
    let p = &idl.library.prefix;
    let mut o = String::new();
    writeln!(o, "// Generated from idl/press.toml by press-idl. Do not edit.").unwrap();
    writeln!(o, "#[allow(unused_imports)]\nuse crate::support::{{self, DestroyFn, Handle, Value}};").unwrap();
    writeln!(o, "use std::ffi::c_void;\n").unwrap();
    writeln!(o, "pub const ABI_VERSION: u32 = {};\n", idl.library.abi_version).unwrap();

    for e in &idl.enums {
        if e.flags {
            writeln!(o, "/// {}", e.doc).unwrap();
            for v in &e.values {
                writeln!(o, "/// {}\npub const {}_{}: u32 = {};", v.doc, shout(&e.name), shout(&v.name), v.value).unwrap();
            }
            writeln!(o).unwrap();
            continue;
        }
        writeln!(o, "/// {}", e.doc).unwrap();
        writeln!(o, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(i32)]\npub enum {} {{", e.name).unwrap();
        for v in &e.values {
            writeln!(o, "    /// {}\n    {} = {},", v.doc, v.name, v.value).unwrap();
        }
        writeln!(o, "}}\n").unwrap();
        writeln!(o, "impl {} {{", e.name).unwrap();
        writeln!(o, "    pub fn from_abi(value: i32) -> Result<Self, Status> {{\n        match value {{").unwrap();
        for v in &e.values {
            writeln!(o, "            {} => Ok({}::{}),", v.value, e.name, v.name).unwrap();
        }
        writeln!(o, "            _ => Err(Status::InvalidArgument),\n        }}\n    }}\n}}\n").unwrap();
    }

    for c in &idl.classes {
        for f in idl.functions(c) {
            let mut params = Vec::new();
            let mut prelude = String::new();
            let mut call = Vec::new();
            if f.receiver.is_some() {
                params.push("this: u64".to_string());
                call.push("Handle(this)".to_string());
            }
            for (name, ty) in &f.args {
                match ty {
                    Ty::Str => {
                        params.push(format!("{name}: *const u8"));
                        params.push(format!("{name}_len: usize"));
                        writeln!(prelude, "        let {name} = support::str_arg({name}, {name}_len)?;").unwrap();
                        call.push(name.clone());
                    }
                    Ty::Bytes => {
                        params.push(format!("{name}: *const u8"));
                        params.push(format!("{name}_len: usize"));
                        writeln!(prelude, "        let {name} = support::bytes_arg({name}, {name}_len)?;").unwrap();
                        call.push(name.clone());
                    }
                    Ty::Callback => {
                        params.push(format!("{name}: Option<support::CallbackFn>"));
                        params.push(format!("{name}_data: *mut c_void"));
                        params.push(format!("{name}_destroy: Option<DestroyFn>"));
                        writeln!(prelude, "        let {name} = support::Work::new({name}, {name}_data, {name}_destroy)?;").unwrap();
                        call.push(name.clone());
                    }
                    Ty::Enum(e) => {
                        params.push(format!("{name}: i32"));
                        writeln!(prelude, "        let {name} = {e}::from_abi({name})?;").unwrap();
                        call.push(name.clone());
                    }
                    Ty::Handle(_) => {
                        params.push(format!("{name}: u64"));
                        call.push(format!("Handle({name})"));
                    }
                    _ => {
                        params.push(format!("{name}: {}", ffi_type(ty)));
                        call.push(name.clone());
                    }
                }
            }
            let api_call = format!("crate::api::{}({})", f.name, call.join(", "));
            let body = match &f.ret {
                None => format!("{prelude}        {api_call}"),
                Some(Ty::Str) => {
                    params.extend(["out: *mut u8".to_string(), "out_cap: usize".to_string(), "out_len: *mut usize".to_string()]);
                    format!("{prelude}        let value = {api_call}?;\n        support::str_out(&value, out, out_cap, out_len)")
                }
                Some(ret) => {
                    params.push(format!("out: *mut {}", ffi_type(ret)));
                    let conv = match ret {
                        Ty::Handle(_) => "value.0",
                        Ty::Enum(_) => "value as i32",
                        _ => "value",
                    };
                    format!(
                        "        if out.is_null() {{\n            return Err(Status::InvalidArgument);\n        }}\n{prelude}        let value = {api_call}?;\n        unsafe {{ out.write({conv}) }};\n        Ok(())"
                    )
                }
            };
            // The signature the api module must provide, as documentation.
            let api_sig: Vec<String> = f
                .receiver
                .iter()
                .map(|_| "this: Handle".to_string())
                .chain(f.args.iter().map(|(n, t)| format!("{n}: {}", api_type(t))))
                .collect();
            let api_ret = match &f.ret {
                Some(Ty::Str) => "String".into(),
                Some(ret) => api_type(ret),
                None => "()".into(),
            };
            writeln!(o, "/// {}\n///\n/// Forwards to `api::{}({}) -> Result<{api_ret}, Status>`.", f.doc, f.name, api_sig.join(", ")).unwrap();
            writeln!(o, "#[no_mangle]\npub unsafe extern \"C\" fn {p}_{}({}) -> i32 {{", f.name, params.join(", ")).unwrap();
            writeln!(o, "    support::guard(|| {{\n{body}\n    }})\n}}\n").unwrap();
        }
    }

    // Events: typed callback aliases, connectors, and the dispatcher.
    let events = idl.events();
    for (id, c, e) in &events {
        let cs = snake(&c.name);
        let alias = format!("{}{}Fn", c.name, camel(&e.name));
        let mut ps = vec!["this: u64".to_string()];
        for a in &e.args {
            match idl.ty(&a.ty).unwrap() {
                Ty::Str | Ty::Bytes => ps.extend([format!("{}: *const u8", a.name), format!("{}_len: usize", a.name)]),
                ty => ps.push(format!("{}: {}", a.name, ffi_type(&ty))),
            }
        }
        ps.push("userdata: *mut c_void".into());
        writeln!(o, "pub const EVENT_{}_{}: u32 = {id};", shout(&c.name), shout(&e.name)).unwrap();
        writeln!(o, "pub type {alias} = unsafe extern \"C\" fn({});\n", ps.join(", ")).unwrap();
        writeln!(o, "/// {}", e.doc).unwrap();
        writeln!(
            o,
            "#[no_mangle]\npub unsafe extern \"C\" fn {p}_{cs}_on_{}(this: u64, callback: Option<{alias}>, userdata: *mut c_void, destroy: Option<DestroyFn>) -> i32 {{",
            e.name
        )
        .unwrap();
        writeln!(
            o,
            "    support::guard(|| support::connect(Handle(this), \"{}\", {id}, callback.map(|f| f as *const ()), userdata, destroy))\n}}\n",
            c.name
        )
        .unwrap();
    }

    writeln!(o, "/// The numeric id of an event emitted by the engine as \"Class.event\".").unwrap();
    writeln!(o, "pub(crate) fn event_id(name: &str) -> Option<u32> {{\n    match name {{").unwrap();
    for (id, c, e) in &events {
        writeln!(o, "        \"{}.{}\" => Some({id}),", c.name, e.name).unwrap();
    }
    writeln!(o, "        _ => None,\n    }}\n}}\n").unwrap();

    writeln!(o, "/// Call a connected handler with the arguments the engine emitted.").unwrap();
    writeln!(o, "///\n/// # Safety\n/// `callback` must be the pointer registered for `event`.").unwrap();
    writeln!(o, "#[allow(unused_variables)]").unwrap();
    writeln!(o, "pub(crate) unsafe fn invoke(event: u32, callback: *const (), this: u64, args: &[Value], userdata: *mut c_void) {{\n    match event {{").unwrap();
    for (id, c, e) in &events {
        let alias = format!("{}{}Fn", c.name, camel(&e.name));
        let mut call = vec!["this".to_string()];
        let mut strings = String::new();
        for (i, a) in e.args.iter().enumerate() {
            call.push(match idl.ty(&a.ty).unwrap() {
                Ty::Str => {
                    // a copy with a NUL on the end, alive for the call
                    writeln!(strings, "            let s{i} = support::c_text(args[{i}].as_str());").unwrap();
                    format!("s{i}.as_ptr(), s{i}.len() - 1")
                }
                Ty::Bytes => format!("args[{i}].as_bytes().as_ptr(), args[{i}].as_bytes().len()"),
                Ty::Bool => format!("args[{i}].as_bool()"),
                Ty::U32 => format!("args[{i}].as_i32() as u32"),
                _ => format!("args[{i}].as_i32()"),
            });
        }
        call.push("userdata".into());
        writeln!(o, "        {id} => {{\n            let f: {alias} = std::mem::transmute(callback);\n{strings}            f({});\n        }}", call.join(", ")).unwrap();
    }
    writeln!(o, "        _ => {{}}\n    }}\n}}").unwrap();
    o
}
