#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""The same dialog as examples/c/hello.c, through the generated Python binding.

    python3 examples/python/hello.py [--snapshot FILE.png]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "bindings", "python"))
import press  # noqa: E402

press.App.init()
assert press.App.abi_version() == press.ABI_VERSION

window = press.Window("Hello from Python", 300, 198)
column = press.Stack(press.Orientation.VERTICAL)
column.set_padding(16)
column.set_spacing(12)

name = press.TextField("")
name.set_placeholder("Type a name")
name.set_min_width(268)
check = press.Checkbox("Say it loudly")
status = press.Label("Press Return to say hello.")
ok = press.Button("OK")
ok.set_is_default(True)
ok.set_enabled(False)
cancel = press.Button("Cancel")
cancel.set_is_cancel(True)

buttons = press.Stack(press.Orientation.HORIZONTAL)
buttons.set_spacing(12)
buttons.set_align(press.Align.CENTER)
buttons.add(cancel)
buttons.add(ok)

for widget in (press.Label("What is your name?"), name, check, status, buttons):
    column.add(widget)
window.set_content(column)

shout = False


def on_ok():
    status.set_text(f"{GREETINGS[greeting]}, {name.get_text()}{'!!!' if shout else '.'}")
    print("ok:", repr(name.get_text()))


def set_shout(value):
    """The checkbox and the Options menu item show the same setting."""
    global shout
    shout = value
    check.set_checked(value)
    loud.set_checked(value)
    print("loud:", "on" if value else "off")


def item(menu, label, key, handler):
    entry = press.MenuItem(label)
    entry.set_shortcut(key)
    entry.on_activated(handler)
    menu.add(entry)
    return entry


bar = press.MenuBar()
file, edit, options = press.Menu("File"), press.Menu("Edit"), press.Menu("Options")
item(file, "Quit", "Q", press.App.quit)
item(edit, "Cut", "X", name.cut)
item(edit, "Copy", "C", name.copy)
item(edit, "Paste", "V", name.paste)
edit.add_separator()
item(edit, "Select All", "A", name.select_all)
loud = item(options, "Say It Loudly", "L", lambda: set_shout(not shout))
# a submenu: an item that opens another menu beside it
GREETINGS = ("Hello", "Good day")
greeting = 0
greeting_menu = press.Menu("")


def choose_greeting(index):
    global greeting
    greeting = index
    for i, entry in enumerate(greeting_items):
        entry.set_checked(i == index)


greeting_items = [item(greeting_menu, word, "", lambda i=i: choose_greeting(i)) for i, word in enumerate(GREETINGS)]
greeting_items[0].set_checked(True)
greeting_item = press.MenuItem("Greeting")
greeting_item.set_submenu(greeting_menu)
options.add(greeting_item)
for menu in (file, edit, options):
    bar.add(menu)
window.set_menu_bar(bar)


ok.on_clicked(on_ok)
cancel.on_clicked(press.App.quit)
name.on_changed(lambda text: ok.set_enabled(bool(text)))
check.on_toggled(set_shout)

# whatever the style, the window is as large as what is in it needs
window.fit()

if len(sys.argv) == 3 and sys.argv[1] == "--snapshot":
    window.type_text("Zoë")  # the first text field has the keyboard
    window.press_key(press.Key.TAB, 0)
    window.type_text(" ")  # Space presses the focused checkbox
    window.press_key(press.Key.TAB, press.Modifier.SHIFT)  # back to the field, text selected
    window.press_key(press.Key.END, 0)
    window.type_text(" Ångström")
    window.press_key(press.Key.RETURN, 0)  # the default button
    window.press_key(press.Key.MENU, 0)  # pull down File...
    window.press_key(press.Key.LEFT, 0)  # ...and wrap round to Options
    window.snapshot(sys.argv[2], 2)
    try:
        press.Label.set_text(ok, "x")  # a Button is not a Label
    except press.PressError as error:
        print("wrong type:", error)
else:
    window.show()
    press.App.run()
press.App.shutdown()
