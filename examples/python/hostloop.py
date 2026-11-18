#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""press inside asyncio.

    python3 examples/python/hostloop.py

asyncio owns the event loop. press is given a turn whenever its descriptor
is readable (`add_reader`), and an ordinary coroutine updates a label once a
second in between.
"""
import asyncio
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "bindings", "python"))
import press  # noqa: E402


async def main():
    press.App.init()
    window = press.Window("asyncio", 220, 90)
    column = press.Stack(press.Orientation.VERTICAL)
    column.set_padding(16)
    column.set_spacing(12)
    label = press.Label("0 seconds")
    reset = press.Button("Reset")
    column.add(label)
    column.add(reset)
    window.set_content(column)
    window.show()

    seconds = 0

    def on_reset():
        nonlocal seconds
        seconds = 0
        label.set_text("0 seconds")

    reset.on_clicked(on_reset)

    loop = asyncio.get_running_loop()
    finished = loop.create_future()

    def turn():
        if not press.App.dispatch() and not finished.done():
            finished.set_result(None)

    async def count():
        nonlocal seconds
        while True:
            await asyncio.sleep(1)
            seconds += 1
            label.set_text(f"{seconds} second{'' if seconds == 1 else 's'}")

    press.App.start()
    loop.add_reader(press.App.fd(), turn)
    counter = asyncio.ensure_future(count())
    await finished
    counter.cancel()
    loop.remove_reader(press.App.fd())
    press.App.stop()
    press.App.shutdown()


asyncio.run(main())
