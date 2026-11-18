// SPDX-License-Identifier: GPL-3.0-or-later
//! Python binding generator (ctypes, no compiled extension).

use crate::{snake, Idl, Ty};
use std::fmt::Write;

fn ctypes(ty: &Ty) -> &'static str {
    match ty {
        Ty::Bool => "ctypes.c_bool",
        Ty::I32 | Ty::Enum(_) => "ctypes.c_int32",
        Ty::U32 => "ctypes.c_uint32",
        Ty::Str | Ty::Bytes | Ty::Callback => unreachable!(),
        Ty::Handle(_) => "ctypes.c_uint64",
    }
}

pub fn generate(idl: &Idl) -> String {
    let p = &idl.library.prefix;
    let mut o = String::new();
    writeln!(o, "# Generated from idl/press.toml by press-idl. Do not edit.").unwrap();
    writeln!(o, "# SPDX-License-Identifier: GPL-3.0-or-later").unwrap();
    writeln!(o, "\"\"\"{}\"\"\"", idl.library.doc).unwrap();
    o.push_str(
        r#"import ctypes
import enum
import itertools
import os
import threading

ABI_VERSION = @ABI@


def _load():
    here = os.path.dirname(os.path.abspath(__file__))
    built = [os.path.join(here, "..", "..", "target", profile, "libpress.so") for profile in ("release", "debug")]
    # of the builds in this checkout, the one built last is the one meant
    built = sorted((p for p in built if os.path.exists(p)), key=os.path.getmtime, reverse=True)
    candidates = [os.environ.get("PRESS_LIB")] + built + ["libpress.so"]
    for path in candidates:
        if path and (os.path.sep not in path or os.path.exists(path)):
            try:
                return ctypes.CDLL(path)
            except OSError:
                continue
    raise OSError("libpress.so not found; build it with 'cargo build' or set PRESS_LIB")


_lib = _load()
_DESTROY = ctypes.CFUNCTYPE(None, ctypes.c_void_p)
_CALLBACK = ctypes.CFUNCTYPE(None, ctypes.c_void_p)
_trampolines = {}
_posted = {}
_finished = []
_posted_lock = threading.Lock()
_post_ids = itertools.count(1)


def _work(fn):
    """Wrap a callable as (function, userdata, destroy) for the library to call later."""
    key = next(_post_ids)

    def run(_userdata):
        fn()

    def done(_userdata):
        # This runs inside one of the trampolines being let go, so they are
        # parked rather than freed here; the next call frees them.
        with _posted_lock:
            _finished.append(_posted.pop(key, None))

    entry = (_CALLBACK(run), _DESTROY(done))
    with _posted_lock:
        _finished.clear()
        _posted[key] = entry
    return entry[0], None, entry[1]


class PressError(Exception):
    def __init__(self, status):
        self.status = status
        super().__init__(status.name)


def _check(code):
    if code != 0:
        raise PressError(Status(code))


def _s(text):
    data = text.encode("utf-8")
    return data, len(data)


def _b(data):
    data = bytes(data)
    return data, len(data)


def _h(obj):
    return obj.handle if obj is not None else 0


class _Object:
    """Wraps a handle. Destroying the wrapper does not destroy the object."""

    def __init__(self, handle):
        self.handle = handle

    def _connect(self, name, connector, cfunc_type, fn, adapt):
        callback = cfunc_type(adapt(fn)) if fn is not None else cfunc_type()
        _check(connector(self.handle, callback, None, _DESTROY()))
        # The library holds a pointer into this trampoline, so it must outlive
        # the wrapper object, which the program is free to drop.
        _trampolines[(self.handle, name)] = callback


"#
        .replace("@ABI@", &idl.library.abi_version.to_string())
        .as_str(),
    );

    for e in &idl.enums {
        let kind = if e.flags { "IntFlag" } else { "IntEnum" };
        writeln!(o, "class {}(enum.{kind}):\n    \"\"\"{}\"\"\"", e.name, e.doc).unwrap();
        for v in &e.values {
            writeln!(o, "    {} = {}", crate::shout(&v.name), v.value).unwrap();
        }
        o.push_str("\n\n");
    }

    for c in &idl.classes {
        let cs = snake(&c.name);
        let base = if c.is_static { String::new() } else { format!("({})", c.base.as_deref().unwrap_or("_Object")) };
        writeln!(o, "class {}{base}:\n    \"\"\"{}\"\"\"", c.name, c.doc).unwrap();
        let mut wrote = false;
        for f in idl.functions(c) {
            wrote = true;
            let method = f.name.strip_prefix(&format!("{cs}_")).unwrap().to_string();
            let is_ctor = method == "new" && f.receiver.is_none() && !c.is_static;
            let mut params = Vec::new();
            let mut call = Vec::new();
            let mut argtypes = Vec::new();
            if f.receiver.is_some() {
                call.push("self.handle".to_string());
                argtypes.push("ctypes.c_uint64".to_string());
            }
            for (name, ty) in &f.args {
                params.push(name.clone());
                match ty {
                    Ty::Str => {
                        call.push(format!("*_s({name})"));
                        argtypes.push("ctypes.c_char_p".into());
                        argtypes.push("ctypes.c_size_t".into());
                    }
                    Ty::Bytes => {
                        call.push(format!("*_b({name})"));
                        argtypes.push("ctypes.c_char_p".into());
                        argtypes.push("ctypes.c_size_t".into());
                    }
                    Ty::Callback => {
                        call.push(format!("*_work({name})"));
                        argtypes.push("_CALLBACK, ctypes.c_void_p, _DESTROY".into());
                    }
                    Ty::Handle(_) => {
                        call.push(format!("_h({name})"));
                        argtypes.push(ctypes(ty).into());
                    }
                    Ty::Enum(_) => {
                        call.push(format!("int({name})"));
                        argtypes.push(ctypes(ty).into());
                    }
                    _ => {
                        call.push(name.clone());
                        argtypes.push(ctypes(ty).into());
                    }
                }
            }
            match &f.ret {
                Some(Ty::Str) => argtypes.push("ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)".into()),
                Some(r) => argtypes.push(format!("ctypes.POINTER({})", ctypes(r))),
                None => {}
            }
            let cfn = format!("_lib.{p}_{}", f.name);
            let sig = if is_ctor {
                format!("    def __init__(self, {}):", params.join(", ")).replace("(self, )", "(self)")
            } else if c.is_static {
                format!("    @staticmethod\n    def {method}({}):", params.join(", "))
            } else {
                format!("    def {method}(self, {}):", params.join(", ")).replace("(self, )", "(self)")
            };
            writeln!(o, "{sig}\n        \"\"\"{}\"\"\"", f.doc).unwrap();
            writeln!(o, "        {cfn}.argtypes = [{}]", argtypes.join(", ")).unwrap();
            match &f.ret {
                None => writeln!(o, "        _check({cfn}({}))", call.join(", ")).unwrap(),
                Some(Ty::Str) => {
                    // ask for the length, then fetch into a buffer that fits
                    let head = call.iter().map(|c| format!("{c}, ")).collect::<String>();
                    writeln!(o, "        size = ctypes.c_size_t()").unwrap();
                    writeln!(o, "        _check({cfn}({head}None, 0, ctypes.byref(size)))").unwrap();
                    writeln!(o, "        buffer = ctypes.create_string_buffer(size.value + 1)").unwrap();
                    writeln!(o, "        _check({cfn}({head}buffer, len(buffer), ctypes.byref(size)))").unwrap();
                    writeln!(o, "        return buffer.raw[:min(size.value, len(buffer) - 1)].decode(\"utf-8\")").unwrap();
                }
                Some(r) => {
                    writeln!(o, "        out = {}()", ctypes(r)).unwrap();
                    call.push("ctypes.byref(out)".into());
                    writeln!(o, "        _check({cfn}({}))", call.join(", ")).unwrap();
                    if is_ctor {
                        // not super(): a base class may have a constructor of its own
                        writeln!(o, "        _Object.__init__(self, out.value)").unwrap();
                    } else if let Ty::Enum(e) = r {
                        writeln!(o, "        return {e}(out.value)").unwrap();
                    } else {
                        writeln!(o, "        return out.value").unwrap();
                    }
                }
            }
            o.push('\n');
        }
        for e in &c.events {
            wrote = true;
            let mut cargs = vec!["ctypes.c_uint64".to_string()];
            let mut names = Vec::new();
            // what the trampoline receives, and what it hands the handler
            let (mut raw, mut given) = (Vec::new(), Vec::new());
            for a in &e.args {
                names.push(a.name.clone());
                match idl.ty(&a.ty).unwrap() {
                    Ty::Str => {
                        cargs.push("ctypes.POINTER(ctypes.c_char), ctypes.c_size_t".into());
                        raw.extend([a.name.clone(), format!("{}_len", a.name)]);
                        given.push(format!("ctypes.string_at({0}, {0}_len).decode(\"utf-8\")", a.name));
                    }
                    Ty::Bytes => {
                        cargs.push("ctypes.POINTER(ctypes.c_char), ctypes.c_size_t".into());
                        raw.extend([a.name.clone(), format!("{}_len", a.name)]);
                        given.push(format!("ctypes.string_at({0}, {0}_len)", a.name));
                    }
                    ty => {
                        cargs.push(ctypes(&ty).into());
                        raw.push(a.name.clone());
                        given.push(a.name.clone());
                    }
                }
            }
            cargs.push("ctypes.c_void_p".into());
            let lambda_args = ["_this".to_string()].into_iter().chain(raw).chain(["_userdata".to_string()]).collect::<Vec<_>>().join(", ");
            writeln!(o, "    def on_{}(self, fn):", e.name).unwrap();
            writeln!(o, "        \"\"\"{} The handler receives ({}); pass None to disconnect.\"\"\"", e.doc, names.join(", ")).unwrap();
            writeln!(o, "        cfunc = ctypes.CFUNCTYPE(None, {})", cargs.join(", ")).unwrap();
            writeln!(o, "        connector = _lib.{p}_{cs}_on_{}", e.name).unwrap();
            writeln!(o, "        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]").unwrap();
            writeln!(o, "        self._connect(\"{}\", connector, cfunc, fn, lambda f: lambda {lambda_args}: f({}))\n", e.name, given.join(", ")).unwrap();
        }
        if !wrote {
            o.push_str("    pass\n\n");
        }
        o.push('\n');
    }
    while o.ends_with("\n\n") {
        o.pop();
    }
    o
}
