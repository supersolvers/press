// SPDX-License-Identifier: GPL-3.0-or-later
//! Swift binding generator. The output is the one source file of the `PressCore`
//! module in bindings/swift, written against the C header, which that package
//! imports as the module `CPress`.
//!
//! How the interface is spelt in Swift:
//!   * names are in camel case; `App.init` is `App.initialize`, since `init`
//!     is what Swift calls a constructor
//!   * every call that can fail throws a `PressError`
//!   * a readable property is a throwing computed property (`try field.text`)
//!     and a writable one has a setter method (`try field.setText("x")`),
//!     because a Swift property's setter cannot throw
//!   * i32 is `Int`, clamped on the way in; u32 is `UInt32`; bytes are
//!     `[UInt8]`; an enum of flags is an `OptionSet`
//!   * an event is connected by passing a closure to `onName`, or nil to
//!     disconnect

use crate::{snake, Class, Idl, Ty};
use std::fmt::Write;

/// Words that cannot be used as a name without backticks.
const KEYWORDS: &[&str] = &[
    "associatedtype", "class", "deinit", "enum", "extension", "fileprivate", "func", "import", "init", "inout",
    "internal", "let", "open", "operator", "private", "protocol", "public", "rethrows", "static", "struct",
    "subscript", "typealias", "var", "break", "case", "continue", "default", "defer", "do", "else", "fallthrough",
    "for", "guard", "if", "in", "repeat", "return", "switch", "where", "while", "as", "catch", "false", "is", "nil",
    "super", "self", "Self", "throw", "throws", "true", "try", "Any", "Type", "Protocol",
];

fn escape(name: String) -> String {
    if KEYWORDS.contains(&name.as_str()) {
        format!("`{name}`")
    } else {
        name
    }
}

/// "set_min_size" -> "setMinSize"
fn lower_camel(name: &str) -> String {
    let mut out = String::new();
    for (i, word) in name.split('_').filter(|w| !w.is_empty()).enumerate() {
        let mut chars = word.chars();
        match chars.next() {
            Some(first) if i > 0 => out.extend(first.to_uppercase()),
            Some(first) => out.push(first),
            None => {}
        }
        out.push_str(chars.as_str());
    }
    out
}

/// "NotInitialized" -> "notInitialized"
fn case_name(name: &str) -> String {
    let mut chars = name.chars();
    let first: String = chars.next().map(|c| c.to_lowercase().collect()).unwrap_or_default();
    escape(first + chars.as_str())
}

/// The name of a method. `init` would read as a constructor.
fn method_name(name: &str) -> String {
    match name {
        "init" => "initialize".into(),
        n => escape(lower_camel(n)),
    }
}

fn arg_name(name: &str) -> String {
    escape(lower_camel(name))
}

/// The type a program sees.
fn public_type(ty: &Ty) -> String {
    match ty {
        Ty::Bool => "Bool".into(),
        Ty::I32 => "Int".into(),
        Ty::U32 => "UInt32".into(),
        Ty::Str => "String".into(),
        Ty::Bytes => "[UInt8]".into(),
        Ty::Enum(n) => n.clone(),
        Ty::Handle(n) => format!("{n}?"),
        Ty::Callback => "@escaping @Sendable () -> Void".into(),
    }
}

/// The type of an out parameter in the C header, and what it starts as.
fn out_var(ty: &Ty) -> &'static str {
    match ty {
        Ty::Bool => "false",
        Ty::I32 | Ty::Enum(_) => "Int32(0)",
        Ty::U32 => "UInt32(0)",
        Ty::Handle(_) => "UInt64(0)",
        Ty::Str | Ty::Bytes | Ty::Callback => unreachable!(),
    }
}

fn doc(out: &mut String, indent: &str, text: &str) {
    if !text.is_empty() {
        writeln!(out, "{indent}/// {text}").unwrap();
    }
}

/// The parameter list of a function, and the arguments of the C call.
/// `label_first` is whether the first parameter is given by name.
fn params(args: &[(String, Ty)], label_first: bool) -> (String, Vec<String>) {
    let mut list = Vec::new();
    let mut call = Vec::new();
    for (i, (name, ty)) in args.iter().enumerate() {
        let n = arg_name(name);
        let label = if i == 0 && !label_first { "_ " } else { "" };
        list.push(format!("{label}{n}: {}", public_type(ty)));
        match ty {
            // Swift lends a String or an array to C for the length of the call
            Ty::Str => call.extend([n.clone(), format!("{n}.utf8.count")]),
            Ty::Bytes => call.extend([n.clone(), format!("{n}.count")]),
            Ty::I32 => call.push(format!("Int32(clamping: {n})")),
            Ty::Enum(_) => call.push(format!("{n}.rawValue")),
            Ty::Handle(_) => call.push(format!("{n}?.handle ?? 0")),
            Ty::Callback => call.extend(["_runWork".to_string(), "box.toOpaque()".to_string(), "_release".to_string()]),
            Ty::Bool | Ty::U32 => call.push(n),
        }
    }
    (list.join(", "), call)
}

/// The parameter list of a class's constructor, if it has one.
fn constructor_params(idl: &Idl, c: &Class) -> Option<String> {
    let k = c.constructor.as_ref()?;
    let args: Vec<(String, Ty)> = k.args.iter().map(|a| (a.name.clone(), idl.ty(&a.ty).unwrap())).collect();
    Some(params(&args, args.len() != 1).0)
}

/// Whether a class above this one has a constructor taking the same things,
/// which Swift then wants marked as overridden.
fn overrides_constructor(idl: &Idl, c: &Class, list: &str) -> bool {
    let mut base = c.base.as_deref();
    while let Some(name) = base {
        let b = idl.classes.iter().find(|b| b.name == name).unwrap();
        if constructor_params(idl, b).as_deref() == Some(list) {
            return true;
        }
        base = b.base.as_deref();
    }
    false
}

const SUPPORT: &str = r#"import CPress

/// The ABI version this binding was generated for. `App.abiVersion()` is that
/// of the library that was loaded.
public let pressABIVersion: UInt32 = @ABI@

/// What a call throws when it does not return `Status.ok`.
public struct PressError: Error, Equatable, CustomStringConvertible {
    /// The status as the library returned it.
    public let code: Int32

    public init(code: Int32) {
        self.code = code
    }

    public init(_ status: Status) {
        self.code = status.rawValue
    }

    /// The status, or nil if the library is newer than this binding and
    /// returned one it does not know.
    public var status: Status? {
        Status(rawValue: code)
    }

    public var description: String {
        status.map { "\($0): \($0.message)" } ?? "unknown status \(code)"
    }
}

/// Wraps a handle. Letting go of the wrapper does not destroy the object;
/// `destroy` does, and so does destroying whatever the object is inside.
/// Two wrappers of the same handle are equal.
open class PressObject: Hashable {
    public let handle: UInt64

    /// Wrap a handle that is already known, as a class of your choosing. The
    /// library checks the class at each call and refuses the wrong one.
    public init(handle: UInt64) {
        self.handle = handle
    }

    public static func == (a: PressObject, b: PressObject) -> Bool {
        a.handle == b.handle
    }

    public func hash(into hasher: inout Hasher) {
        hasher.combine(handle)
    }
}

@inline(__always)
private func _check(_ code: Int32) throws {
    if code != 0 {
        throw PressError(code: code)
    }
}

/// Fetch a string result: ask for its length, then for the string itself in
/// a buffer that fits.
private func _string(_ fetch: (UnsafeMutablePointer<CChar>?, Int, UnsafeMutablePointer<Int>?) -> Int32) throws -> String {
    var size = 0
    try _check(fetch(nil, 0, &size))
    var buffer = [CChar](repeating: 0, count: size + 1)
    let capacity = buffer.count
    try _check(buffer.withUnsafeMutableBufferPointer { fetch($0.baseAddress, capacity, &size) })
    return String(decoding: buffer.prefix(min(size, capacity - 1)).map { UInt8(bitPattern: $0) }, as: UTF8.self)
}

/// Copy a string lent to a handler.
private func _text(_ pointer: UnsafePointer<CChar>?, _ length: Int) -> String {
    guard let pointer, length > 0 else { return "" }
    return pointer.withMemoryRebound(to: UInt8.self, capacity: length) {
        String(decoding: UnsafeBufferPointer(start: $0, count: length), as: UTF8.self)
    }
}

/// Copy bytes lent to a handler.
private func _bytes(_ pointer: UnsafePointer<UInt8>?, _ length: Int) -> [UInt8] {
    guard let pointer, length > 0 else { return [] }
    return Array(UnsafeBufferPointer(start: pointer, count: length))
}

/// Keeps a closure alive while the library holds it as userdata.
private final class _Box<T> {
    let value: T

    init(_ value: T) {
        self.value = value
    }
}

/// The closure behind a userdata pointer. The caller holds its own reference
/// to it for the length of the call, so a handler may replace or disconnect
/// itself while it runs.
private func _unbox<T>(_ userdata: UnsafeMutableRawPointer?, as type: T.Type) -> T {
    Unmanaged<_Box<T>>.fromOpaque(userdata!).takeUnretainedValue().value
}

/// The destroy notifier of every closure handed to the library.
private func _release(_ userdata: UnsafeMutableRawPointer?) {
    if let userdata {
        Unmanaged<AnyObject>.fromOpaque(userdata).release()
    }
}

/// Runs a closure handed over as a piece of work.
private func _runWork(_ userdata: UnsafeMutableRawPointer?) {
    _unbox(userdata, as: (() -> Void).self)()
}

"#;

pub fn generate(idl: &Idl) -> String {
    let p = &idl.library.prefix;
    let mut o = String::new();
    writeln!(o, "// Generated from idl/press.toml by press-idl. Do not edit.").unwrap();
    writeln!(o, "// SPDX-License-Identifier: GPL-3.0-or-later").unwrap();
    writeln!(o, "//\n// {}\n", idl.library.doc).unwrap();
    o.push_str(&SUPPORT.replace("@ABI@", &idl.library.abi_version.to_string()));

    for e in &idl.enums {
        doc(&mut o, "", &e.doc);
        if e.flags {
            writeln!(o, "public struct {}: OptionSet, Hashable, Sendable {{", e.name).unwrap();
            writeln!(o, "    public let rawValue: UInt32\n").unwrap();
            writeln!(o, "    public init(rawValue: UInt32) {{\n        self.rawValue = rawValue\n    }}\n").unwrap();
            for v in &e.values {
                doc(&mut o, "    ", &v.doc);
                writeln!(o, "    public static let {} = {}(rawValue: {})", case_name(&v.name), e.name, v.value).unwrap();
            }
            writeln!(o, "}}\n").unwrap();
            continue;
        }
        writeln!(o, "public enum {}: Int32, CaseIterable, Sendable {{", e.name).unwrap();
        for v in &e.values {
            doc(&mut o, "    ", &v.doc);
            writeln!(o, "    case {} = {}", case_name(&v.name), v.value).unwrap();
        }
        if e.name == "Status" {
            writeln!(o, "\n    /// A short description of the status.").unwrap();
            writeln!(o, "    public var message: String {{\n        switch self {{").unwrap();
            for v in &e.values {
                writeln!(o, "        case .{}: return {:?}", case_name(&v.name), v.doc).unwrap();
            }
            writeln!(o, "        }}\n    }}").unwrap();
        }
        writeln!(o, "}}\n").unwrap();
    }

    for c in &idl.classes {
        let cs = snake(&c.name);
        doc(&mut o, "", &c.doc);
        if c.is_static {
            writeln!(o, "public enum {} {{", c.name).unwrap();
        } else {
            writeln!(o, "public class {}: {} {{", c.name, c.base.as_deref().unwrap_or("PressObject")).unwrap();
            // Said again in every class, since a class with a constructor of
            // its own does not inherit this one.
            writeln!(o, "    public override init(handle: UInt64) {{\n        super.init(handle: handle)\n    }}\n").unwrap();
        }
        let prefix = if c.is_static { "public static func" } else { "public func" };
        let functions = idl.functions(c);
        let method_names: Vec<String> = c.methods.iter().map(|m| method_name(&m.name)).collect();
        for f in &functions {
            let name = f.name.strip_prefix(&format!("{cs}_")).unwrap();
            let is_ctor = name == "new" && c.constructor.is_some();
            let getter = c.properties.iter().find(|pr| name.strip_prefix("get_") == Some(pr.name.as_str()) && f.args.is_empty());
            let setter = c.properties.iter().any(|pr| name.strip_prefix("set_") == Some(pr.name.as_str()));
            let (list, mut call) = params(&f.args, is_ctor && f.args.len() != 1);
            if f.receiver.is_some() {
                call.insert(0, "handle".into());
            }
            let cfn = format!("{p}_{}", f.name);
            let callbacks = f.args.iter().filter(|(_, t)| *t == Ty::Callback).count();
            assert!(callbacks <= 1, "{}: one callback at most", f.name);

            doc(&mut o, "    ", f.doc);
            // Where the body is indented to, and what closes it.
            let (pad, close) = if is_ctor {
                let over = if overrides_constructor(idl, c, &list) { "override " } else { "" };
                writeln!(o, "    public {over}init({list}) throws {{").unwrap();
                ("        ", "    }")
            } else if let Some(pr) = getter {
                let n = arg_name(&pr.name);
                assert!(!method_names.contains(&n), "{}: the property {n} has the name of a method", c.name);
                let keyword = if c.is_static { "public static var" } else { "public var" };
                writeln!(o, "    {keyword} {n}: {} {{\n        get throws {{", public_type(f.ret.as_ref().unwrap()).trim_end_matches('?')).unwrap();
                ("            ", "        }\n    }")
            } else {
                let n = method_name(name);
                assert!(setter || !n.starts_with('`'), "{}: {n} needs another name in Swift", f.name);
                let ret = f.ret.as_ref().map(|r| format!(" -> {}", public_type(r))).unwrap_or_default();
                writeln!(o, "    {prefix} {n}({list}) throws{ret} {{").unwrap();
                ("        ", "    }")
            };

            if callbacks == 1 {
                let (work, _) = f.args.iter().find(|(_, t)| *t == Ty::Callback).unwrap();
                assert!(f.ret.is_none(), "{}: a function that takes a callback returns nothing", f.name);
                // If the call is refused the library has taken nothing, and
                // the closure is ours to let go of.
                writeln!(o, "{pad}let box = Unmanaged.passRetained(_Box<() -> Void>({}))", arg_name(work)).unwrap();
                writeln!(o, "{pad}let status = {cfn}({})", call.join(", ")).unwrap();
                writeln!(o, "{pad}if status != 0 {{\n{pad}    box.release()\n{pad}}}").unwrap();
                writeln!(o, "{pad}try _check(status)").unwrap();
            } else {
                match &f.ret {
                    None => writeln!(o, "{pad}try _check({cfn}({}))", call.join(", ")).unwrap(),
                    Some(Ty::Str) => {
                        let head = call.iter().map(|a| format!("{a}, ")).collect::<String>();
                        writeln!(o, "{pad}return try _string {{ {cfn}({head}$0, $1, $2) }}").unwrap();
                    }
                    Some(r) => {
                        writeln!(o, "{pad}var out = {}", out_var(r)).unwrap();
                        call.push("&out".into());
                        writeln!(o, "{pad}try _check({cfn}({}))", call.join(", ")).unwrap();
                        if is_ctor {
                            writeln!(o, "{pad}super.init(handle: out)").unwrap();
                        } else {
                            match r {
                                Ty::I32 => writeln!(o, "{pad}return Int(out)").unwrap(),
                                Ty::Handle(n) => writeln!(o, "{pad}return out == 0 ? nil : {n}(handle: out)").unwrap(),
                                Ty::Enum(n) => {
                                    // a value this binding has no name for
                                    writeln!(o, "{pad}guard let value = {n}(rawValue: out) else {{\n{pad}    throw PressError(.{})\n{pad}}}", case_name("Internal")).unwrap();
                                    writeln!(o, "{pad}return value").unwrap();
                                }
                                _ => writeln!(o, "{pad}return out").unwrap(),
                            }
                        }
                    }
                }
            }
            writeln!(o, "{close}\n").unwrap();
        }

        for e in &c.events {
            // what the C function receives, and what it hands the handler
            let mut raw = vec!["_".to_string()];
            let (mut types, mut given, mut names) = (Vec::new(), Vec::new(), Vec::new());
            let mut prelude = String::new();
            for a in &e.args {
                let n = arg_name(&a.name);
                names.push(a.name.clone());
                let ty = idl.ty(&a.ty).unwrap();
                types.push(public_type(&ty));
                match ty {
                    Ty::Str => {
                        raw.extend([n.clone(), format!("{n}Length")]);
                        given.push(format!("_text({n}, {n}Length)"));
                    }
                    Ty::Bytes => {
                        raw.extend([n.clone(), format!("{n}Length")]);
                        given.push(format!("_bytes({n}, {n}Length)"));
                    }
                    Ty::I32 => {
                        raw.push(n.clone());
                        given.push(format!("Int({n})"));
                    }
                    Ty::Enum(en) => {
                        // a value this binding has no name for is not passed on
                        raw.push(n.clone());
                        writeln!(prelude, "            guard let {n} = {en}(rawValue: {n}) else {{ return }}").unwrap();
                        given.push(n);
                    }
                    Ty::Handle(cn) => {
                        raw.push(n.clone());
                        given.push(format!("{n} == 0 ? nil : {cn}(handle: {n})"));
                    }
                    Ty::Bool | Ty::U32 => {
                        raw.push(n.clone());
                        given.push(n);
                    }
                    Ty::Callback => unreachable!(),
                }
            }
            raw.push("userdata".into());
            let fn_ty = format!("({}) -> Void", types.join(", "));
            let cfn = format!("{p}_{cs}_on_{}", e.name);
            let receives = if names.is_empty() { String::new() } else { format!(" The handler receives ({}).", names.join(", ")) };
            writeln!(o, "    /// {}{receives} A new handler takes the place of the last; pass nil to disconnect.", e.doc).unwrap();
            writeln!(o, "    public func {}(_ handler: ({fn_ty})?) throws {{", escape(lower_camel(&format!("on_{}", e.name)))).unwrap();
            writeln!(o, "        guard let handler else {{\n            return try _check({cfn}(handle, nil, nil, nil))\n        }}").unwrap();
            writeln!(o, "        let box = Unmanaged.passRetained(_Box<{fn_ty}>(handler))").unwrap();
            writeln!(o, "        let status = {cfn}(handle, {{ {} in", raw.join(", ")).unwrap();
            o.push_str(&prelude);
            writeln!(o, "            _unbox(userdata, as: ({fn_ty}).self)({})", given.join(", ")).unwrap();
            writeln!(o, "        }}, box.toOpaque(), _release)").unwrap();
            writeln!(o, "        if status != 0 {{\n            box.release()\n        }}").unwrap();
            writeln!(o, "        try _check(status)\n    }}\n").unwrap();
        }
        // no blank line before the closing brace
        if o.ends_with("\n\n") {
            o.pop();
        }
        writeln!(o, "}}\n").unwrap();
    }
    while o.ends_with("\n\n") {
        o.pop();
    }
    o
}
