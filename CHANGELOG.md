# Changes

What changed in each version of PRESS, the newest first. The version is the
one in `Cargo.toml`. Two other numbers say what a program or a style can
rely on, and each has its own history:

- the interface a program is written against (`PRESS_ABI_VERSION` in
  `include/press.h`, and the number `libpress.so.1` ends in), now 1;
- the interface a style is written against (`PRESS_STYLE_ABI` in
  `include/press_style.h`), now 11.

## 0.1.0, not yet released

The first standalone version of PRESS, which became a separate project
on 18 November 2026.

### The engine and its interface

- A flat C interface over the engine, generated from `idl/press.toml` with
  bindings for Swift and Python, and Press, a framework for Swift written
  on top of it.
- Windows under any Wayland compositor, and drawing to PNG files with no
  display at all.
- Buttons, checkboxes, radio buttons, sliders, steppers, combo boxes, text
  fields and text areas, lists, tables, icon views, tree views, tab views,
  scroll views, progress bars, pictures, group boxes, toolbars, status
  bars, menus, context menus and pop-up buttons.
- Rich text: words with looks of their own, read and written as RTF.
- Dividers between the panes of a stack.
- Drag and drop, the clipboard, input methods, accessibility, whole-number
  and fractional scale, and a program's own event loop.
- The library is named for its interface's version (`libpress.so.1`).

### Styles

- A program's whole window, frame and all, is drawn by a style: a library
  of its own, written in C against `include/press_style.h`.
- Fourteen looks, named for their years: Eighty-Four (built in),
  Eighty-Five, Eighty-Eight, Eighty-Nine, Ninety, Ninety-One, Ninety-Three,
  Ninety-Five, Ninety-Eight, Ninety-Nine, Two Thousand, Two Thousand and
  One, Two Thousand and Four and Two Thousand and Five.
- Themes: a look in other colours, sizes and type, from a text file. Six
  come with the library.
- A title bar may have a button that puts its window behind the others,
  through a Wayland protocol of PRESS's own, `press_depth_v1`.
- Two Thousand and One's window frame is measured from the period's
  desktop: a title bar 30 pixels high, shaded row for row as its title bars
  were (and paler behind), with its buttons six pixels down, and an edge
  four pixels deep, lighter inside than out.

### Installing

- `make install` installs the library, its headers, the styles, the themes
  and the licence. The fonts as files are installed only if they have been
  built; the library carries its own copy.
- A C program finds the library through pkg-config, as `press`.
