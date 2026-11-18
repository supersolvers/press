#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Copying, pasting and dragging something that is neither text nor a file.

    python3 examples/python/content.py PICTURE.png

The window can copy the picture to the clipboard and paste one from it,
its one row can be dragged out carrying the picture, and a picture dragged
from elsewhere can be dropped on it. The toolkit only carries the bytes:
what they mean is between the programs at either end.

What is copied or dragged goes as two kinds at once, the picture and its
name as text, so that a program that knows nothing of pictures can still
take the name.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "bindings", "python"))
import press  # noqa: E402

KIND = "image/png"
TEXT = "text/plain;charset=utf-8"
with open(sys.argv[1], "rb") as file:
    picture = file.read()

press.App.init()
window = press.Window("Content", 280, 150)
column = press.Stack(press.Orientation.VERTICAL)
column.set_padding(12)
column.set_spacing(8)

status = press.Label("Nothing pasted or dropped yet.")
copy = press.Button("Copy Picture")
paste = press.Button("Paste Picture")
rows = press.ListBox()
rows.add(os.path.basename(sys.argv[1]))
rows.set_icon(0, press.Icon.DOCUMENT)
rows.set_drag_data(0, KIND, picture)  # the row carries the picture when dragged out
caption = os.path.basename(sys.argv[1]).encode()
rows.add_drag_data(0, TEXT, caption)  # and its name, for whatever takes only text


def say(how, kind, data):
    same = " (the same picture)" if data == picture else ""
    status.set_text(f"{how} {len(data)} bytes of {kind}{same}" if data else f"Nothing of kind {kind} to paste.")
    print(how.lower(), kind, len(data), flush=True)


def copy_picture():
    press.App.copy_data(KIND, picture)
    press.App.copy_also(TEXT, caption)


copy.on_clicked(copy_picture)
paste.on_clicked(lambda: window.paste_data(KIND))
window.on_data_pasted(lambda kind, data: say("Pasted", kind, data))
window.accept_drops(KIND)
window.on_data_dropped(lambda kind, data, x, y: say("Dropped", kind, data))

buttons = press.Stack(press.Orientation.HORIZONTAL)
buttons.set_spacing(10)
buttons.add(copy)
buttons.add(paste)
for widget in (buttons, rows, status):
    column.add(widget)
window.set_content(column)
window.show()
press.App.run()
