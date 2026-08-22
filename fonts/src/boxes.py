"""Box drawing (U+2500-U+257F) and block elements (U+2580-U+259F) for the fixed-width families.

These are drawn here in pixels, not as outlines: they have to fill the cell exactly, from the top
of the line to the bottom and from one side to the other, so that a frame drawn with them joins up
from one character and one line to the next. Each is made from its Unicode name ("LIGHT DOWN AND
RIGHT", "DOWN SINGLE AND HORIZONTAL DOUBLE", ...), so every one of the set is covered by the same
few rules.

The cell is the family's advance wide and its whole line high (line_asc + line_desc). A light line
is one pixel, a heavy one two, a double one two lines of one pixel with one between; vertical lines
sit at the middle column, horizontal ones at the middle row of the line.

The glyphs are given in the shape the CJK import uses (see cjk.py): (advance, width, height,
x offset, y offset of the bottom from the baseline, rows as integers with the leftmost pixel in
the top bit of a byte-padded row), so that the build embeds their pixels untouched.
"""
import unicodedata

from cjk import outline

FIRST, LAST = 0x2500, 0x25A0


def _cell(fam):
    return int(fam.mono), int(fam.line_asc + fam.line_desc)


def _arms(name):
    """The weight of each arm (up, right, down, left): 0 none, 1 light, 2 heavy, 3 double."""
    weights = {"LIGHT": 1, "SINGLE": 1, "HEAVY": 2, "DOUBLE": 3}
    dirs = {"UP": ("up",), "DOWN": ("down",), "LEFT": ("left",), "RIGHT": ("right",),
            "HORIZONTAL": ("left", "right"), "VERTICAL": ("up", "down")}
    arms = dict(up=0, right=0, down=0, left=0)
    parts = [p.split() for p in name.split(" AND ")]
    found = [([w for w in p if w in weights], [d for w in p if w in dirs for d in dirs[w]]) for p in parts]
    # a part that names no weight takes that of its neighbour: "LIGHT DOWN AND RIGHT"
    for i, (ws, ds) in enumerate(found):
        w = ws[0] if ws else next((f[0][0] for f in found[i + 1:] + found[:i][::-1] if f[0]), "LIGHT")
        for d in ds:
            arms[d] = weights[w]
    return arms


def _draw(name, W, H):
    px = [[0] * W for _ in range(H)]

    def fill(x0, y0, x1, y1):                     # inclusive
        for y in range(max(0, y0), min(H - 1, y1) + 1):
            for x in range(max(0, x0), min(W - 1, x1) + 1):
                px[y][x] = 1

    L = (W - 1) // 2                              # the middle column
    M = (H - 1) // 2                              # the middle row
    if name.startswith("BOX DRAWINGS "):
        name = name[len("BOX DRAWINGS "):]
        if "DIAGONAL" in name:
            for y in range(H):
                x = round((H - 1 - y) * (W - 1) / (H - 1))
                if "UPPER RIGHT TO LOWER LEFT" in name or "CROSS" in name:
                    px[y][x] = 1
                if "UPPER LEFT TO LOWER RIGHT" in name or "CROSS" in name:
                    px[y][W - 1 - x] = 1
            return px
        heavy = name.startswith("HEAVY")
        t = 2 if heavy else 1
        if "DASH" in name:
            n = {"DOUBLE DASH": 2, "TRIPLE DASH": 3, "QUADRUPLE DASH": 4}[next(k for k in ("DOUBLE DASH", "TRIPLE DASH", "QUADRUPLE DASH") if k in name)]
            length = W if "HORIZONTAL" in name else H
            step = length / n
            for i in range(n):
                a, b = round(i * step), round((i + 1) * step) - 2     # a gap of at least one pixel at each end
                if "HORIZONTAL" in name:
                    fill(a + (0 if i else 0), M, max(a, b), M + t - 1)
                else:
                    fill(L, a, L + t - 1, max(a, b))
            return px
        if "ARC" in name:
            # a light corner with its turn cut by a pixel
            down, right = "DOWN" in name, "RIGHT" in name
            ys = range(M + 1, H) if down else range(0, M)
            xs = range(L + 1, W) if right else range(0, L)
            for y in ys:
                px[y][L] = 1
            for x in xs:
                px[M][x] = 1
            px[M][L] = 0
            return px
        arms = _arms(name)
        up, right, down, left = arms["up"], arms["right"], arms["down"], arms["left"]
        D = 3
        for arm, w in arms.items():
            if not w:
                continue
            if w in (1, 2):
                a, b = (L, L + w - 1)                 # columns of a vertical stroke
                c, d = (M, M + w - 1)                 # rows of a horizontal one
                # a single line meeting a double one stops at it; otherwise it reaches the middle
                if arm == "up":
                    fill(a, 0, b, (M - 2) if (left == D or right == D) else d)
                elif arm == "down":
                    fill(a, (M + 2) if (left == D or right == D) else c, b, H - 1)
                elif arm == "left":
                    fill(0, c, (L - 2) if (up == D or down == D) else b, d)
                else:
                    fill((L + 2) if (up == D or down == D) else a, c, W - 1, d)
            else:
                # Two lines. Each stops where a double arm across its side turns into it, and
                # otherwise carries on to the far line of the other pair.
                if arm == "up":
                    fill(L - 1, 0, L - 1, (M - 1) if left == D else (M + 1 if down != D else H - 1))
                    fill(L + 1, 0, L + 1, (M - 1) if right == D else (M + 1 if down != D else H - 1))
                elif arm == "down":
                    fill(L - 1, (M + 1) if left == D else (M - 1 if up != D else 0), L - 1, H - 1)
                    fill(L + 1, (M + 1) if right == D else (M - 1 if up != D else 0), L + 1, H - 1)
                elif arm == "left":
                    fill(0, M - 1, (L - 1) if up == D else (L + 1 if right != D else W - 1), M - 1)
                    fill(0, M + 1, (L - 1) if down == D else (L + 1 if right != D else W - 1), M + 1)
                else:
                    fill((L + 1) if up == D else (L - 1 if left != D else 0), M - 1, W - 1, M - 1)
                    fill((L + 1) if down == D else (L - 1 if left != D else 0), M + 1, W - 1, M + 1)
        # a single or heavy vertical crossing a double horizontal (or the other way) carries on
        # between its two lines
        if (up in (1, 2) and down in (1, 2)) and (left == D or right == D):
            fill(L, M - 1, L + up - 1, M + 1)
        if (left in (1, 2) and right in (1, 2)) and (up == D or down == D):
            fill(L - 1, M, L + 1, M + left - 1)
        return px

    # block elements
    cp = unicodedata.lookup(name)
    o = ord(cp)
    half_w, half_h = W // 2, H // 2
    if o == 0x2580: fill(0, 0, W - 1, half_h - 1)                       # upper half
    elif 0x2581 <= o <= 0x2588:                                           # lower eighths to full
        n = o - 0x2580
        fill(0, H - round(H * n / 8), W - 1, H - 1)
    elif 0x2589 <= o <= 0x258F:                                           # left seven eighths to one
        n = 0x2590 - o
        fill(0, 0, max(0, round(W * n / 8) - 1), H - 1)
    elif o == 0x2590: fill(half_w, 0, W - 1, H - 1)                     # right half
    elif o in (0x2591, 0x2592, 0x2593):                                   # shades
        for y in range(H):
            for x in range(W):
                on = {0x2591: x % 2 == 0 and y % 2 == 0, 0x2592: (x + y) % 2 == 0, 0x2593: not (x % 2 == 1 and y % 2 == 1)}[o]
                px[y][x] = int(on)
    elif o == 0x2594: fill(0, 0, W - 1, max(0, round(H / 8) - 1))       # upper eighth
    elif o == 0x2595: fill(W - max(1, round(W / 8)), 0, W - 1, H - 1)    # right eighth
    else:                                                                 # quadrants
        q = {0x2596: "3", 0x2597: "4", 0x2598: "1", 0x2599: "134", 0x259A: "14", 0x259B: "123",
             0x259C: "124", 0x259D: "2", 0x259E: "23", 0x259F: "234"}[o]
        boxes = {"1": (0, 0, half_w - 1, half_h - 1), "2": (half_w, 0, W - 1, half_h - 1),
                 "3": (0, half_h, half_w - 1, H - 1), "4": (half_w, half_h, W - 1, H - 1)}
        for k in q:
            fill(*boxes[k])
    return px


def load(fam, existing):
    """-> [(codepoint, glyph name, bdf-shaped glyph, outline)] for a fixed-width family."""
    if not fam.mono:
        return []
    W, H = _cell(fam)
    out = []
    for cp in range(FIRST, LAST):
        if cp in existing:
            continue
        name = unicodedata.name(chr(cp), "")
        if not name:
            continue
        px = _draw(name, W, H)
        nb = ((W + 7) // 8) * 8
        rows = tuple(sum(bit << (nb - 1 - x) for x, bit in enumerate(row)) for row in px)
        g = (W, W, H, 0, -int(fam.line_desc), rows)
        out.append((cp, "uni%04X" % cp, g, outline(g)))
    return out
