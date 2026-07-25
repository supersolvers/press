"""Accented letters for a family whose cell is too narrow for the outline's own.

Drawing an accented letter from its outline squeezes letter and accent into the cell, and in an
8 px cell that makes a lump; it also leaves out any hand-tuned plain letter. A family with
`compose` set has its accented letters made here instead, the way a terminal font makes them: the
plain letter exactly as it is (hand-tuned or not), with a small accent set above it or hung below.

The accents are drawn for an 8-pixel cell whose letter sits in columns 1-6, and for the rows a
16-px family has: the line runs from 4 rows below the top of the strike, capitals start 3 rows
under that, small letters 6. Everything stays inside the line where it can:

- one accent on a small letter: the two rows just above it, a row clear;
- one on a capital: the same, two rows higher (a ring or a hook, three rows, sits on the letter);
- two on a small letter: the second three rows higher, or beside the first (Vietnamese);
- two on a capital: side by side in two rows, narrow forms, a column clear between them;
- below: from the row under the baseline.

Covered: accented Latin (U+00C0-U+024F, U+1E00-U+1EFF, Vietnamese included), Greek (U+0370-U+03FF,
polytonic U+1F00-U+1FFF) and Cyrillic (U+0400-U+052F); and Cyrillic letters with a descender whose
plain letter was hand-drawn. A hand-drawn letter in the pixel file always wins over this.
"""
import unicodedata

import numpy as np

COLS = 8

GRAVE, ACUTE, CIRC, TILDE, MACRON, BREVE, DOT, DIAER, RING, DACUTE, CARON, DGRAVE, IBREVE = (
    "̀", "́", "̂", "̃", "̄", "̆", "̇", "̈", "̊",
    "̋", "̌", "̏", "̑")
PSILI, DASIA, PERI, YPO = "̓", "̔", "͂", "ͅ"
HOOK, HORN = "̉", "̛"
DOT_BELOW, COMMA_BELOW, CEDILLA, OGONEK = "̣", "̦", "̧", "̨"
BELOW = {YPO, DOT_BELOW, COMMA_BELOW, CEDILLA, OGONEK}

# One accent over the middle of a letter; the last row of each sits just above the letter.
ONE = {
    ACUTE: ["....##..", "...##..."],
    GRAVE: ["..##....", "...##..."],
    CIRC: ["...##...", "..#..#.."],
    CARON: ["..#..#..", "...##..."],
    TILDE: ["..##.#..", ".#..##.."],
    PERI: ["..##.#..", ".#..##.."],
    MACRON: [".######."],
    BREVE: [".##..##.", "..####.."],
    IBREVE: ["..####..", ".##..##."],
    DOT: ["...##...", "...##..."],
    DIAER: [".##..##.", ".##..##."],
    RING: ["...##...", "..#..#..", "...##..."],
    DACUTE: ["..##.##.", ".##.##.."],
    DGRAVE: [".##.##..", "..##.##."],
    HOOK: ["...##...", ".....#..", "....#..."],   # a small question mark without its dot
    # The breathings are a 9 and a 6: the smooth heavy at the top with its tail below, the rough
    # with its tail on top and heavy below, so that they are told apart by their weight.
    PSILI: ["...##...", "....#..."],
    DASIA: ["...#....", "...##..."],
}
# on a capital there is one row to spare, so the two dots are one row
CAP_DIAER = [".##..##."]
# two accents on a capital, one row each, one over the other
ONE_ROW = {DIAER: [".##..##."], MACRON: [".######."], DOT: ["...##..."], ACUTE: ["....##.."], GRAVE: ["..##...."]}
# Greek: a breathing at the left, an accent at the right, side by side
LEFT = {PSILI: ["##.", ".#."], DASIA: ["#..", "##."]}
RIGHT = {ACUTE: [".##", "##."], GRAVE: ["##.", ".##"], PERI: ["##.#", "#.##"]}
# the two dots with something between them
DOTS_AND = {
    ACUTE: ["....#...", ".#.#..#."],
    GRAVE: ["...#....", ".#..#.#."],
    PERI: ["..####..", ".#....#."],
    CARON: ["..#..#..", ".#.##.#."],
    MACRON: ["..####..", ".#....#."],
}
# Vietnamese sets an acute, a grave or a hook up at the right of a circumflex or a breve. On a
# small letter there is room above it; these are the forms set there, offset to the right.
UP_RIGHT = {
    ACUTE: [".....##.", "....##.."],
    GRAVE: ["....##..", ".....##."],
    HOOK: ["....###.", "......#."],
}
# Narrow forms, for two accents side by side on a capital.
NARROW = {
    CIRC: [".#.", "#.#"],
    BREVE: ["#.#", ".#."],
    CARON: ["#.#", ".#."],
    TILDE: [".#.#", "#.#."],
    ACUTE: ["..#", ".#."],
    GRAVE: ["#..", ".#."],
    HOOK: ["##.", ".#."],
    MACRON: ["...", "###"],
    DIAER: ["...", "#.#"],
    DOT: [".", "#"],
}
# Some two-accent pairs on a capital read better wider than their narrow forms allow.
WIDE_RIGHT = {ACUTE: ["..##", ".##."], GRAVE: [".##.", "..##"], HOOK: [".###", "...#"]}
# A ring with an acute on it, on a capital: three rows sitting on the letter.
RING_ACUTE = ["..##..#.", ".#..##..", "..##...."]
# hung below the baseline
UNDER = {
    CEDILLA: ["....#...", "...##...", "...#...."],
    OGONEK: [".....#..", ".....##."],
    COMMA_BELOW: ["...##...", "..##...."],
    DOT_BELOW: ["........", "...##..."],
}
# a caron on these is written as an apostrophe beside the ascender: (column, rows above the
# capital line); on d there is no column free beside the ascender, so it floats clear above it
APOSTROPHE = {"d": (7, -3), "l": (6, 0), "t": (5, -1), "L": (4, 0)}


class Geometry:
    """Where things are in a family's cell, as rows from the top of its strike."""

    def __init__(self, strike):
        fam = strike.fam
        self.rows = strike.asc + strike.desc
        self.baseline = strike.asc                        # the first row below the letters
        self.cap = strike.asc - int(fam.cap)             # the first row of a capital
        self.x = strike.asc - int(fam.xh)                # the first row of a small letter
        self.line = strike.asc - int(fam.line_asc)       # the top row of the line


def stamp(rows, pattern, top, left=0):
    for r, line in enumerate(pattern):
        for c, p in enumerate(line):
            if p == "#" and 0 <= top + r < len(rows) and 0 <= left + c < COLS:
                rows[top + r][left + c] = 1


def over(rows, pattern, bottom, left=0):
    """Stamp a pattern so that its last row is `bottom`."""
    stamp(rows, pattern, bottom - len(pattern) + 1, left)


def side_by_side(first, second):
    """Two narrow accents next to each other, a column clear between, centred in the cell."""
    width = max(map(len, first)) + 1 + max(map(len, second))
    left = max(0, (COLS - width + 1) // 2)
    rows = max(len(first), len(second))
    first = [""] * (rows - len(first)) + first
    second = [""] * (rows - len(second)) + second
    out = []
    for a, b in zip(first, second):
        a = a.ljust(max(map(len, first)), ".")
        out.append(("." * left + a + "." + b).ljust(COLS, ".")[:COLS])
    return out


def composed(rows, base, marks, greek_capital, g):
    rows = [r[:] for r in rows]
    horn = HORN in marks
    marks = [m for m in marks if m != HORN]
    above = [m for m in marks if m not in BELOW]
    below = [m for m in marks if m in BELOW]
    inked = [r for r in range(g.rows) if any(rows[r])]
    top_ink = min(inked) if inked else g.x
    if base in "ijіј" and above:      # i and j lose their dot under an accent
        for r in range(0, g.x):
            rows[r] = [0] * COLS
        top_ink = g.x
    tall = top_ink < g.x                         # a capital, or a small letter with an ascender
    bottom = (g.cap if tall else g.x) - 2        # last row an accent takes, a row clear
    pair = tuple(above)
    if horn:
        # a horn: a short upright stroke against the letter's top right
        stamp(rows, ["#", "#"], top_ink - 1, 7)
    if above == [CARON] and base in APOSTROPHE:
        col, up = APOSTROPHE[base]
        stamp(rows, ["#", "#"], g.cap + up, col)
    elif len(above) == 1:
        m = above[0]
        if m not in ONE:
            return None
        if greek_capital and m in (ACUTE, GRAVE, PSILI, DASIA):
            # a Greek capital carries its accent or breathing before it, at the top left
            over(rows, RIGHT[m] if m in RIGHT else LEFT[m], bottom, 1)
        else:
            # a ring or a hook needs three rows: on a capital it sits right on the letter
            last = bottom + 1 if (m in (RING, HOOK) and tall) else bottom
            over(rows, CAP_DIAER if (m == DIAER and tall) else ONE[m], last)
    elif len(above) == 2 and above[0] in (CIRC, BREVE) and above[1] in UP_RIGHT and not tall:
        over(rows, ONE[above[0]], bottom)
        over(rows, UP_RIGHT[above[1]], bottom - 3)
    elif len(above) == 2 and above[0] == DIAER and above[1] in DOTS_AND:
        over(rows, DOTS_AND[above[1]], bottom)
    elif len(above) == 2 and above[0] in LEFT and above[1] in (ACUTE, GRAVE):
        over(rows, LEFT[above[0]], bottom, 1)
        over(rows, RIGHT[above[1]], bottom, 5)
    elif len(above) == 2 and above[0] in LEFT and above[1] == PERI:
        if tall:                                  # no room to stack inside the line: side by side
            over(rows, LEFT[above[0]], bottom, 1)
            over(rows, RIGHT[PERI], bottom, 4)
        else:                                     # the circumflex over the breathing
            over(rows, ONE[PERI], bottom - 3)
            over(rows, ONE[above[0]], bottom)
    elif pair == (RING, ACUTE) and tall:
        over(rows, RING_ACUTE, bottom + 1)
    elif len(above) == 2 and tall and all(m in ONE_ROW for m in above):
        over(rows, ONE_ROW[above[0]], bottom)
        over(rows, ONE_ROW[above[1]], bottom - 1)
    elif len(above) == 2 and tall and above[0] in NARROW and above[1] in NARROW:
        # side by side, the second at the right; as wide as there is room for
        second = WIDE_RIGHT.get(above[1], NARROW[above[1]])
        if max(map(len, NARROW[above[0]])) + 1 + max(map(len, second)) > COLS:
            second = NARROW[above[1]]
        over(rows, side_by_side(NARROW[above[0]], second), bottom)
    elif above:
        # one over another: the first written nearest the letter
        if any(m not in ONE for m in above):
            return None
        for n, m in enumerate(above):
            over(rows, ONE[m], bottom - 3 * n)
    for m in below:
        if m == YPO:
            # the small iota underneath, clear of whatever the letter has down there
            low = [c for c in range(COLS) if any(rows[r][c] for r in range(g.baseline, g.rows))]
            col = 3 if not any(c in (2, 3, 4) for c in low) else (1 if 1 not in low and 2 not in low else 6)
            stamp(rows, ["#", "#"], g.baseline + 1, col)
        else:
            stamp(rows, UNDER[m], g.baseline)
    return rows


def with_tail(rows, g):
    """A letter with a descender: the plain letter and a short tail at the bottom right."""
    rows = [r[:] for r in rows]
    stamp(rows, ["##", "##"], g.baseline, 6)
    return rows


COVERED = [(0xC0, 0x250), (0x370, 0x530), (0x1E00, 0x2000)]


def compose(strike, ch, pad):
    """The pixels for `ch` made from its plain letter, as the strike's array, or None to leave
    it to the outline. `strike.glyph` gives the plain letter, hand-tuned or not."""
    cp = ord(ch)
    if not any(a <= cp < b for a, b in COVERED):
        return None
    nfd = unicodedata.normalize("NFD", ch)
    base, marks = nfd[0], list(nfd[1:])
    g = Geometry(strike)
    if marks and unicodedata.category(base).startswith("L") and ord(base) in strike.cmap:
        plain = strike.glyph(base)[1]
        rows = [list(map(int, plain[r, pad:pad + COLS])) for r in range(g.rows)]
        made = composed(rows, base, marks, "GREEK CAPITAL" in unicodedata.name(base, ""), g)
    elif not marks and unicodedata.name(ch, "").endswith(" WITH DESCENDER"):
        try:
            other = unicodedata.lookup(unicodedata.name(ch)[: -len(" WITH DESCENDER")])
        except KeyError:
            return None
        if other not in strike.overrides:
            return None
        plain = strike.glyph(other)[1]
        rows = [list(map(int, plain[r, pad:pad + COLS])) for r in range(g.rows)]
        made = with_tail(rows, g)
    else:
        return None
    if made is None:
        return None
    a = np.zeros((g.rows, COLS + 2 * pad), dtype=np.uint8)
    a[:, pad:pad + COLS] = np.array(made, dtype=np.uint8)
    return a
