#!/usr/bin/env python3
"""Render specimen PNGs: ./.venv/bin/python specimen.py [text-file]  ->  build/specimen-*.png"""
import glob
import os
import sys

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "src"))
from build import FAMILIES

DEFAULT = [
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "abcdefghijklmnopqrstuvwxyz",
    "0123456789 .,:;!?'\"-_+=<>/\\|()[]{}*^`~#$%&@",
    "The quick brown fox jumps over the lazy dog.",
    "File  Edit  View  Special     Hamburgefonstiv",
]


def render(path, lines, out):
    sizes = [(64, lines), (32, lines[3:]), (None, lines)]
    native = next((int(f.em) for f in FAMILIES if f"{f.name}-{f.style}" in path), 12)
    rows = []
    for size, ls in sizes:
        size = size or native
        f = ImageFont.truetype(path, size, layout_engine=ImageFont.Layout.RAQM)
        for l in ls:
            rows.append((f, l, size))
    w = 2200
    h = sum(int(s * 1.5) + 6 for _, _, s in rows) + 40
    im = Image.new("L", (w, h), 255)
    d = ImageDraw.Draw(im)
    y = 20
    for f, l, s in rows:
        d.text((20, y), l, font=f, fill=0)
        y += int(s * 1.5) + 6
    im.save(out)
    print(out)


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    lines = DEFAULT
    if len(sys.argv) > 1:
        lines = open(sys.argv[1], encoding="utf-8").read().splitlines()
    for p in sorted(glob.glob(os.path.join(here, "build", "*.ttf"))):
        render(p, lines, p[:-4].replace("build/", "build/specimen-") + ".png")
