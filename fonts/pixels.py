#!/usr/bin/env python3
"""Preview the fonts as 1-bit pixels at their native size:
    ./.venv/bin/python pixels.py [text-file]
    ./.venv/bin/python pixels.py --dump "as"     print glyphs in the override format (see pixels/)"""
import glob, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "src"))
import numpy as np
from PIL import Image
import bitmap
from build import FAMILIES

LINES = ["ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz", "0123456789 .,:;!?'\"-_+=<>/\\|()[]{}*^`~#$%&@",
         "The quick brown fox jumps over the lazy dog.", "File  Edit  View  Special   Hamburgefonstiv AVATAR"]

if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    dump = sys.argv[2] if len(sys.argv) > 2 and sys.argv[1] == "--dump" else None
    lines = open(sys.argv[1], encoding="utf-8").read().splitlines() if len(sys.argv) > 1 and not dump else LINES
    Z = 5
    for fam in FAMILIES:
        path = os.path.join(here, "build", f"{fam.name}-{fam.style}.ttf")
        strike = bitmap.Strike(path, fam, os.path.join(here, "pixels", f"{fam.name}-{fam.style}.txt"))
        if dump:
            print(f"# {fam.name}-{fam.style}"); [print(strike.dump(ch)) for ch in dump]
            continue
        lh = int(fam.line_asc + fam.line_desc) + 2
        im = Image.new("L", (460, lh * len(lines) + 4), 255)
        for i, l in enumerate(lines):
            strike.draw_text(im, 3, 2 + i * lh + int(fam.line_asc), l)
        im = Image.fromarray(np.kron(np.asarray(im), np.ones((Z, Z), dtype=np.uint8)))
        out = os.path.join(here, "build", f"pixels-{fam.name}-{fam.style}.png")
        im.save(out); print(out)
