# Changes

Releases are listed newest first. The PRESS version comes from
`Cargo.toml`. The program and style interfaces have their own versions.

- The program ABI is 1, as recorded by `PRESS_ABI_VERSION` in
  `include/press.h` and the library name `libpress.so.1`.
- The style ABI is 11, as recorded by `PRESS_STYLE_ABI` in
  `include/press_style.h`.

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
- Icon views laid out as icons, as tiles (a picture of 48 pixels with the
  name and two lines of details beside it) or as thumbnails (a picture of
  up to 96 pixels in a frame), `IconView.set_layout`; an item's large
  picture may be up to 96 pixels.
- The rich-text widget reads and writes RTF.
- Dividers between the panes of a stack.
- Drag and drop, the clipboard, input methods, accessibility, whole-number
  and fractional scale, and a program's own event loop.
- The library is named for its interface's version (`libpress.so.1`).

### Styles

- A program's whole window, frame and all, is drawn by a style: a library
  of its own, written in C against `include/press_style.h`.
- Fifteen looks, named for their years: Eighty-Four (built in),
  Eighty-Five, Eighty-Eight, Eighty-Nine, Ninety, Ninety-One, Ninety-Three,
  Ninety-Five, Ninety-Eight, Ninety-Nine, Two Thousand, Two Thousand and
  One, Two Thousand and Four, Two Thousand and Five and Two Thousand and
  Twenty-Seven. The last recalls no desktop there was: it is a future as
  2001 saw it, in lit gel.
- Themes override a style's colors, sizes, and fonts from a text file.
  Six come with the library.
- A title bar may have a button that puts its window behind the others,
  through a Wayland protocol of PRESS's own, `press_depth_v1`.
- Two Thousand and One's window frame is measured from the period's
  desktop: a title bar 30 pixels high, shaded row for row as its title bars
  were (and paler behind), with its buttons six pixels down, and an edge
  four pixels deep, lighter inside than out.
- Two Thousand and Four's window frame is laid out as the period's window
  manager laid out its frames: square, in a line of black; a line of light
  and a title bar of twenty rows shading from tan to brown, the title in
  white over a shadow, and a dark line under it; edges five pixels deep
  with a line of shade at the right and foot; square buttons of 17 pixels,
  darker than the bar in a rim of light, with signs two pixels thick.
  Behind others it is flat parchment with a grey title.
- Two Thousand and Four's widgets are shaped as the period's were,
  measured from it and drawn in the style's own colours: buttons flat and
  pale in a thin grey line, a little darker low down, the default one in a
  dark line with a soft glow three pixels deep (`default_ring` 3; buttons
  and pop-ups 30 pixels high); boxes to tick and rings to choose sunk a
  little; fields white in a grey line; scroll bars and sliders with grey
  troughs and knobs with a grip of dots (a slider's knob 31 pixels long and
  nothing marking the way gone); tabs behind greyer and two pixels lower;
  list headers as buttons side by side; menus flat, with no shadow and a
  single line between groups, their items 23 pixels high.

### Installing

- `make install` installs the library, its headers, the styles, the themes
  and the licence. The fonts as files are installed only if they have been
  built; the library carries its own copy.
- A C program finds the library through pkg-config, as `press`.
