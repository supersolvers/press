// SPDX-License-Identifier: GPL-3.0-or-later
//! The press interface definition: its data model, validation, and the
//! generators that turn it into a C header, the Rust export layer and
//! language bindings.

use serde::Deserialize;
use std::collections::HashSet;

pub mod c;
pub mod python;
pub mod rust;
pub mod swift;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Idl {
    pub library: Library,
    #[serde(default, rename = "enum")]
    pub enums: Vec<Enum>,
    #[serde(default, rename = "class")]
    pub classes: Vec<Class>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub name: String,
    pub prefix: String,
    pub abi_version: u32,
    #[serde(default)]
    pub doc: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enum {
    pub name: String,
    #[serde(default)]
    pub doc: String,
    /// The values are bits to be combined in a u32, not alternatives.
    #[serde(default)]
    pub flags: bool,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnumValue {
    pub name: String,
    pub value: i32,
    #[serde(default)]
    pub doc: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Class {
    pub name: String,
    #[serde(default)]
    pub doc: String,
    pub base: Option<String>,
    /// No constructor; exists to carry functions shared by its subclasses.
    #[serde(default, rename = "abstract")]
    pub is_abstract: bool,
    /// A namespace of free functions; there are no instances.
    #[serde(default, rename = "static")]
    pub is_static: bool,
    pub constructor: Option<Constructor>,
    #[serde(default, rename = "method")]
    pub methods: Vec<Method>,
    #[serde(default, rename = "property")]
    pub properties: Vec<Property>,
    #[serde(default, rename = "event")]
    pub events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constructor {
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub args: Vec<Arg>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub name: String,
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub args: Vec<Arg>,
    pub returns: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub doc: String,
    /// "rw" (default), "r" or "w".
    #[serde(default = "rw")]
    pub access: String,
}

fn rw() -> String {
    "rw".into()
}

impl Property {
    pub fn readable(&self) -> bool {
        self.access.contains('r')
    }
    pub fn writable(&self) -> bool {
        self.access.contains('w')
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub name: String,
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub args: Vec<Arg>,
}

/// A resolved type name.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Bool,
    I32,
    U32,
    Str,
    /// Any bytes at all: a pointer and a length, lent for the call. Only
    /// allowed as an argument, of a method or of an event.
    Bytes,
    Enum(String),
    Handle(String),
    /// A function to call later, with its userdata and destroy notifier.
    /// Only allowed as a method argument.
    Callback,
}

impl Idl {
    pub fn parse(text: &str) -> Result<Idl, String> {
        let idl: Idl = toml::from_str(text).map_err(|e| e.to_string())?;
        idl.validate()?;
        Ok(idl)
    }

    pub fn ty(&self, name: &str) -> Result<Ty, String> {
        Ok(match name {
            "bool" => Ty::Bool,
            "i32" => Ty::I32,
            "u32" => Ty::U32,
            "str" => Ty::Str,
            "bytes" => Ty::Bytes,
            "callback" => Ty::Callback,
            n if self.enums.iter().any(|e| e.name == n && e.flags) => {
                return Err(format!("'{n}' is a set of flags; pass it as u32"));
            }
            n if self.enums.iter().any(|e| e.name == n) => Ty::Enum(n.into()),
            n if self.classes.iter().any(|c| c.name == n && !c.is_static) => Ty::Handle(n.into()),
            n => return Err(format!("unknown type '{n}'")),
        })
    }

    fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for name in self.enums.iter().map(|e| &e.name).chain(self.classes.iter().map(|c| &c.name)) {
            if !seen.insert(name) {
                return Err(format!("'{name}' is defined twice"));
            }
        }
        if !self.enums.iter().any(|e| e.name == "Status" && e.values.iter().any(|v| v.name == "Ok" && v.value == 0)) {
            return Err("an enum 'Status' with 'Ok' = 0 is required".into());
        }
        for c in &self.classes {
            let at = |what: &str| format!("{}.{}", c.name, what);
            if let Some(base) = &c.base {
                if !self.classes.iter().any(|b| &b.name == base) {
                    return Err(format!("{}: unknown base '{base}'", c.name));
                }
            }
            if c.is_static && (c.constructor.is_some() || !c.properties.is_empty() || !c.events.is_empty()) {
                return Err(format!("{}: a static class can only have methods", c.name));
            }
            if c.is_abstract && c.constructor.is_some() {
                return Err(format!("{}: an abstract class cannot have a constructor", c.name));
            }
            let mut names = HashSet::new();
            for a in c.constructor.iter().flat_map(|k| &k.args) {
                self.ty(&a.ty).map_err(|e| format!("{}: {e}", at("constructor")))?;
            }
            for m in &c.methods {
                if !names.insert(m.name.clone()) {
                    return Err(format!("{} is defined twice", at(&m.name)));
                }
                for a in &m.args {
                    self.ty(&a.ty).map_err(|e| format!("{}: {e}", at(&m.name)))?;
                }
                if let Some(r) = &m.returns {
                    let ret = self.ty(r).map_err(|e| format!("{}: {e}", at(&m.name)))?;
                    if matches!(ret, Ty::Callback | Ty::Bytes) {
                        return Err(format!("{}: a callback or bytes cannot be returned", at(&m.name)));
                    }
                }
            }
            for p in &c.properties {
                if matches!(self.ty(&p.ty).map_err(|e| format!("{}: {e}", at(&p.name)))?, Ty::Callback | Ty::Bytes) {
                    return Err(format!("{}: a callback or bytes cannot be a property", at(&p.name)));
                }
                if !matches!(p.access.as_str(), "rw" | "r" | "w") {
                    return Err(format!("{}: access must be rw, r or w", at(&p.name)));
                }
                for n in [format!("set_{}", p.name), format!("get_{}", p.name)] {
                    if !names.insert(n.clone()) {
                        return Err(format!("{} clashes with a method", at(&n)));
                    }
                }
            }
            for e in &c.events {
                if !names.insert(format!("on_{}", e.name)) {
                    return Err(format!("{} is defined twice", at(&e.name)));
                }
                for a in &e.args {
                    let ty = self.ty(&a.ty).map_err(|err| format!("{}: {err}", at(&e.name)))?;
                    if ty == Ty::Callback {
                        return Err(format!("{}: an event cannot carry a callback", at(&e.name)));
                    }
                }
            }
        }
        Ok(())
    }

    /// Every event with a stable numeric id (declaration order).
    pub fn events(&self) -> Vec<(u32, &Class, &Event)> {
        let mut id = 0;
        let mut out = Vec::new();
        for c in &self.classes {
            for e in &c.events {
                id += 1;
                out.push((id, c, e));
            }
        }
        out
    }
}

/// "MenuBar" -> "menu_bar"
pub fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// "NotInitialized" -> "NOT_INITIALIZED"
pub fn shout(name: &str) -> String {
    snake(name).to_uppercase()
}

/// "close_requested" -> "CloseRequested"
pub fn camel(name: &str) -> String {
    name.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect()
}

/// The flat functions a class contributes, in a generator-neutral form.
pub struct Function<'a> {
    /// Name without the library prefix, e.g. "button_set_label".
    pub name: String,
    pub doc: &'a str,
    /// Class of the receiver, None for constructors and static functions.
    pub receiver: Option<&'a str>,
    pub args: Vec<(String, Ty)>,
    pub ret: Option<Ty>,
}

impl Idl {
    /// Constructors, methods and property accessors (not event connectors).
    pub fn functions<'a>(&'a self, c: &'a Class) -> Vec<Function<'a>> {
        let cs = snake(&c.name);
        let recv = if c.is_static { None } else { Some(c.name.as_str()) };
        let args = |list: &[Arg]| list.iter().map(|a| (a.name.clone(), self.ty(&a.ty).unwrap())).collect::<Vec<_>>();
        let mut out = Vec::new();
        if let Some(k) = &c.constructor {
            out.push(Function {
                name: format!("{cs}_new"),
                doc: &k.doc,
                receiver: None,
                args: args(&k.args),
                ret: Some(Ty::Handle(c.name.clone())),
            });
        }
        for m in &c.methods {
            out.push(Function {
                name: format!("{cs}_{}", m.name),
                doc: &m.doc,
                receiver: recv,
                args: args(&m.args),
                ret: m.returns.as_ref().map(|r| self.ty(r).unwrap()),
            });
        }
        for p in &c.properties {
            let ty = self.ty(&p.ty).unwrap();
            if p.writable() {
                out.push(Function {
                    name: format!("{cs}_set_{}", p.name),
                    doc: &p.doc,
                    receiver: recv,
                    args: vec![("value".into(), ty.clone())],
                    ret: None,
                });
            }
            if p.readable() {
                out.push(Function {
                    name: format!("{cs}_get_{}", p.name),
                    doc: &p.doc,
                    receiver: recv,
                    args: vec![],
                    ret: Some(ty),
                });
            }
        }
        out
    }
}
