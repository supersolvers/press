// SPDX-License-Identifier: GPL-3.0-or-later
//! The press engine. It owns the widget tree and knows how to lay it out,
//! draw it in black and white, and react to pointer and keyboard input. It has no
//! knowledge of windows systems or of the C ABI; both live in the `press`
//! crate and drive this one.

pub mod canvas;
pub mod cursor;
pub mod drives;
pub mod font;
pub mod pictures;
pub mod programs;
pub mod slots;
pub mod style;
pub mod text;
pub mod ui;

pub use cursor::{Cursor, CursorImage};
pub use font::{Face, Font};
pub use canvas::{Canvas, Rect, BLACK, TRANSPARENT, WHITE};
pub use slots::Handle;
pub use style::{Metrics, OneBit, Painter, Palette, Part, PartKind, Style};
pub use ui::{Icon, LineStyle, MenuEntry, AccessNode, AccessRole, AccessRun, AccessTree, Align, PopupInfo, Emitted, Error, Key, Mods, Orientation, PointerKind, Ui, Value, WindowInfo};
