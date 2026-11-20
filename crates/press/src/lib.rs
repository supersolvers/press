// SPDX-License-Identifier: GPL-3.0-or-later
//! libpress: the flat C ABI over the press engine.
//!
//! * `abi` is generated from idl/press.toml and holds every exported symbol.
//! * `api` is the hand-written implementation each exported function forwards to.
//! * `support` holds the per-thread runtime, argument checking and callbacks.
//! * `backend` puts windows on a display.
//! * `style` loads a look for the interface from a library of its own.

#![allow(clippy::missing_safety_doc)]

pub mod abi {
    include!(concat!(env!("OUT_DIR"), "/abi.rs"));
}
pub use abi::Status;

mod api;
mod backend;
pub mod style;
mod support;
pub mod theme;
