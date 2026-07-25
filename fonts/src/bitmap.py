"""Native-size 1-bit bitmaps: derived from the outlines, optionally overridden
by hand-drawn pixels, and embedded in the font as an EBLC/EBDT strike."""
import os
import struct

import numpy as np

import compose
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables.DefaultTable import DefaultTable
from PIL import Image, ImageDraw, ImageFont

SS = 8          # supersampling factor
THRESH = 0.42   # ink coverage needed to switch a pixel on
PAD = 6         # px kept left and right of the advance box


def load_overrides(path, rows):
    """Text file of hand-drawn glyphs. Each block is

        glyph x top=7
        #...#
        .#.#.          '#' = ink, anything else = blank; a blank line ends the block.

    `top` is the cell row of the first line (row 0 is the top of the cell; the
    baseline row is printed by `pixels.py --dump`). Column 0 is the glyph origin.
    """
    out = {}
    if not os.path.exists(path):
        return out
    lines = [l.rstrip("\n") for l in open(path, encoding="utf-8")] + [""]
    i = 0
    while i < len(lines):
        if lines[i].startswith("glyph "):
            ch = lines[i][6]
            top = int(lines[i].split("top=")[1]) if "top=" in lines[i] else 0
            j = i + 1
            while lines[j].strip() and not lines[j].startswith(("glyph ", "#!")):
                j += 1
            block = lines[i + 1:j]
            a = np.zeros((rows, max(len(l) for l in block) + 2 * PAD), dtype=np.uint8)
            for r, l in enumerate(block):
                for c, v in enumerate(l):
                    a[top + r, PAD + c] = v == "#"
            out[ch] = a
            i = j
        else:
            i += 1
    return out


class Strike:
    def __init__(self, path, fam, override_file=None, extra=None, build_order=None):
        self.fam = fam
        self.path = path
        self.tt = TTFont(path)
        self.cmap = self.tt.getBestCmap()
        self.hmtx = self.tt["hmtx"]
        self.font = ImageFont.truetype(path, int(fam.em * SS))
        self.asc = int(fam.line_asc) + 4          # rows above the baseline kept per glyph (stacked accents)
        self.desc = int(fam.line_desc) + 1
        self.overrides = load_overrides(override_file, self.asc + self.desc) if override_file else {}
        self.cache = {}
        self.extra = extra or {}          # imported bitmaps (CJK): build glyph name -> BDF glyph
        self.build_order = build_order

    def glyph(self, ch):
        """-> (advance_px, array[rows, PAD + advance + PAD]); row 0 is self.asc px above the baseline."""
        if ch in self.cache:
            return self.cache[ch]
        name = self.cmap.get(ord(ch), ".notdef")
        adv = int(round(self.hmtx[name][0] / self.fam.px))
        g = self.extra.get("uni%04X" % ord(ch))
        if g is not None:
            _, w, h, xo, yo, rows = g
            a = np.zeros((self.asc + self.desc, adv + 2 * PAD), dtype=np.uint8)
            nb = ((w + 7) // 8) * 8
            for r, v in enumerate(rows):
                for x in range(w):
                    if v >> (nb - 1 - x) & 1:
                        a[self.asc - (yo + h) + r, PAD + xo + x] = 1
        elif ch in self.overrides:
            a = self.overrides[ch]
        elif self.fam.compose and (made := compose.compose(self, ch, PAD)) is not None:
            a = made
        else:
            w = adv + 2 * PAD
            big = Image.new("L", (w * SS, (self.asc + self.desc) * SS), 0)
            ImageDraw.Draw(big).text((PAD * SS, self.asc * SS), ch, font=self.font, fill=255, anchor="ls")
            cov = np.asarray(big, dtype=float).reshape(self.asc + self.desc, SS, w, SS).mean(axis=(1, 3)) / 255
            a = (cov >= THRESH).astype(np.uint8)
        self.cache[ch] = (adv, a)
        return adv, a

    def dump(self, ch):
        adv, a = self.glyph(ch)
        rows = np.nonzero(a.any(axis=1))[0]
        top, bot = (rows[0], rows[-1] + 1) if len(rows) else (0, 0)
        art = "\n".join("".join("#" if v else "." for v in row[PAD:PAD + adv]) for row in a[top:bot])
        return "glyph %s top=%d\n%s\n" % (ch, top, art)

    def draw_text(self, im, x, baseline, text):
        px = im.load()
        for ch in text:
            adv, a = self.glyph(ch)
            for r, c in zip(*np.nonzero(a)):
                X, Y = x + c - PAD, baseline - self.asc + r
                if 0 <= X < im.width and 0 <= Y < im.height:
                    px[X, Y] = 0
            x += adv
        return x

    # ---------------------------------------------------------------- embed
    def embed(self):
        """Write the strike into the font file as EBLC/EBDT tables."""
        tt = TTFont(self.path)
        order = tt.getGlyphOrder()
        by_name = {}
        for cp, name in sorted(self.cmap.items()):
            by_name.setdefault(name, chr(cp))
        ebdt = bytearray(struct.pack(">L", 0x00020000))
        offsets = []
        wmax = 0
        for i, name in enumerate(order):
            offsets.append(len(ebdt) - 4)
            g = self.extra.get(self.build_order[i]) if self.build_order else None
            if g is not None:                  # imported bitmap: store its rows untouched
                adv, w, h, xo, yo, rows = g
                if w and h and any(rows):
                    nbytes = (w + 7) // 8
                    wmax = max(wmax, adv)
                    ebdt += struct.pack(">BBbbB", h, w, xo, yo + h, adv)
                    ebdt += b"".join(v.to_bytes(nbytes, "big") for v in rows)
                continue
            ch = by_name.get(name)
            if ch is None:
                continue
            adv, a = self.glyph(ch)
            ys, xs = np.nonzero(a)
            if not len(ys):
                continue                       # blank glyph: renderer falls back to the outline
            r0, r1, c0, c1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
            box = a[r0:r1, c0:c1]
            wmax = max(wmax, adv)
            ebdt += struct.pack(">BBbbB", r1 - r0, c1 - c0, c0 - PAD, self.asc - r0, adv)
            ebdt += np.packbits(box, axis=1).tobytes()        # byte-aligned rows
        offsets.append(len(ebdt) - 4)
        n = len(order)
        sub = struct.pack(">HHL", 1, 1, 4) + b"".join(struct.pack(">L", o) for o in offsets)
        array = struct.pack(">HHL", 0, n - 1, 8)
        la, ld = int(self.fam.line_asc), int(self.fam.line_desc)
        line = struct.pack(">bbBbbbbbbbbb", la, -ld, wmax, 1, 0, 0, 0, 0, la, -ld, 0, 0)
        ppem = int(self.fam.em)
        size = struct.pack(">LLLL", 8 + 48, len(array) + len(sub), 1, 0) + line + line + \
            struct.pack(">HHBBBb", 0, n - 1, ppem, ppem, 1, 1)
        eblc = struct.pack(">LL", 0x00020000, 1) + size + array + sub
        for tag, data in (("EBLC", eblc), ("EBDT", bytes(ebdt))):
            t = DefaultTable(tag)       # raw bytes; skip fontTools' own EBLC/EBDT compiler
            t.data = data
            tt[tag] = t
        tt.save(self.path)
        return sum(1 for a, b in zip(offsets, offsets[1:]) if b > a)
