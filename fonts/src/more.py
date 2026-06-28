"""Coverage beyond the core: extended Latin and Cyrillic letters, polytonic
Greek, Hebrew, and extra symbols."""
from glyphs import GLYPHS, stem, bar, ring
from latin import auto_composites, _mp, _dotsize
from scripts import reg, same, mid, both, _ze, _zhe, _che, _en, _ge


def tick(c, t, y=None):
    """Short descender tick, as on Қ Ң Ҳ Ҷ."""
    y = c.B if y is None else y
    c.s([(t, y), (t, y - 1.5)])


def base(ch):
    return GLYPHS[ch][0]


# ============================================================ LATIN EXTRAS
def _schwa(c, T):
    m = mid(c, T)
    c.s([(1, m), (0, m), (0, c.B, 1), (1, c.B, 1), (1, T, 1), (0, T, 1), (0, mid(c, T, .8))])

both("Ə", None, (7, 7), None, _schwa)
same("ǝӘә", "əƏə")

def _open_e(c, T):
    m = mid(c, T)
    c.s([(1, mid(c, T, .8)), (1, T, 1), (0, T, 1), (0, m, 1), (.6, m)]); c.s([(.6, m), (0, m, 1), (0, c.B, 1), (1, c.B, 1), (1, mid(c, T, .2))])

both("Ɛ", "ɛ", (6, 6), (5, 4), _open_e)

def _open_o(c, T):
    c.s([(0, mid(c, T, .75)), (0, T, 1), (1, T, 1), (1, c.B, 1), (0, c.B, 1), (0, mid(c, T, .25))])

both("Ɔ", "ɔ", (7, 6), (5, 5), _open_o)
reg("Ǝ", lambda c: (c.s([(0, c.C), (1, c.C), (1, c.B), (0, c.B)]), bar(c, c.cy(.5), .2, 1)), 6, 6)

def _hooked(letter, x_stem):
    """Capital with a flag to the left of its stem (Ɓ Ɗ Ƙ)."""
    def draw(c):
        c.lsb += 1; base(letter)(c); x = c.lsb + c.hx
        c.s([(x - 1 - c.hx, c.C), (x, c.C)], absx=True)
    return draw

for _ch, _b in zip("ƁƊƘ", "BDK"):
    GLYPHS[_ch] = (_hooked(_b, 0), GLYPHS[_b][1], GLYPHS[_b][2])

reg("ɓ", lambda c: (c.s([(0, c.B), (0, c.A, 1), (.7, c.A)]), c.s([(0, c.X), (1, c.X, 1), (1, c.B, 1), (0, c.B)])), 6, 5)
reg("ɗ", lambda c: (c.s([(.8, c.B), (.8, c.A, 1), (1.25, c.A)]), c.s([(.8, c.X), (0, c.X, 1), (0, c.B, 1), (.8, c.B)])), 7, 6)
reg("ƙ", lambda c: (c.s([(0, c.B), (0, c.A, 1), (.6, c.A)]), c.s([(1, c.X), (0, c.xy(.42)), (1, c.B)])), 6, 5)
reg("Ƴ", lambda c: (c.s([(0, c.C), (.5, c.cy(.45)), (1, c.C - .8), (1, c.C, 1), (1.3, c.C)]), stem(c, .5, c.B, c.cy(.45))), 7, 7)
reg("ƴ", lambda c: (c.s([(0, c.X), (.5, c.B)]), c.s([(1.2, c.X), (1, c.X, .6), (1 - .5 * (c.X - c.D) / (c.X - c.B), c.D)])), 6, 5)

def _cap_eszett(c):
    m = c.cy(.55)
    c.s([(0, c.B), (0, c.C, 1), (.85, c.C, 1), (.85, m, .8), (.42, m)]); c.s([(.42, m), (1, m, 1), (1, c.B, 1), (.38, c.B)])

reg("ẞ", _cap_eszett, 8, 7)

def _pair(a, b, gap=0):
    def draw(c):
        x = c.place(a, 1, 0, 0); c.place(b, 1, x + gap, 0)
    return draw

def _pair_w(a, b, gap=0):
    ia, ib = GLYPHS[a][1], GLYPHS[b][1]
    return {k: ia[k] + ib[k] + 1 + gap for k in ("ch", "ag")}

for _ch, (_a, _b) in {"Ĳ": "IJ", "ĳ": "ij", "Ǉ": "LJ", "ǈ": "Lj", "ǉ": "lj", "Ǌ": "NJ", "ǋ": "Nj", "ǌ": "nj", "ŉ": "’n"}.items():
    GLYPHS[_ch] = (_pair(_a, _b), _pair_w(_a, _b), {"ch": (0, 0), "ag": (0, 0)})

def _ldot(letter, T):
    def draw(c):
        base(letter)(c); c.rsb += 2; d = _dotsize(c)
        x = c.lsb + c.ink + 1; y = (c.f.cap if T else c.f.asc) / 2
        c.rect(x - d / 2, y - d / 2, x + d / 2, y + d / 2)
    return draw

GLYPHS["Ŀ"] = (_ldot("L", True), GLYPHS["L"][1], GLYPHS["L"][2])
GLYPHS["ŀ"] = (_ldot("l", False), GLYPHS["l"][1], GLYPHS["l"][2])

auto_composites(list(range(0xC0, 0x250)) + list(range(0x1E00, 0x1F00)))      # picks up the new bases


# ====================================================== CYRILLIC EXTENDED
def _ka(c, T):
    stem(c, 0, c.B, T); c.s([(1, T), (0, mid(c, T, .45)), (1, c.B)])

def _ha(c, T):
    base("Х" if T == c.C else "х")(c)

def with_tick(fn, t=1, shift=True):
    def draw(c, T):
        fn(c, T); tick(c, t)
    return draw

def _ghe_bar(c, T):
    _ge(c, T); c.s([(c.lsb - .6, mid(c, T)), (c.lsb + 2 * c.hx + 1.6, mid(c, T))], absx=True)

def _oe(c, T):
    ring(c, c.B, T); bar(c, mid(c, T))

def _shha(c, T):
    stem(c, 0, c.B, T); c.s([(0, mid(c, T, .55)), (1, mid(c, T, .55), 1), (1, c.B)])

def _es_tail(c, T):
    base("С" if T == c.C else "с")(c); p, k = _mp(c); cx = c.lsb + c.ink / 2
    c.s([(cx, .2), (cx, -1.6)], **k)

def _ze_tail(c, T):
    _ze(c, T); p, k = _mp(c); cx = c.lsb + c.ink / 2
    c.s([(cx, .2), (cx, -1.6)], **k)

def _bashkir_ka(c, T):
    c.s([(0, T), (.22, T), (.22, c.B)]); c.s([(1, T), (.22, mid(c, T, .45)), (1, c.B)])

def _en_tick(c, T):
    stem(c, 0, c.B, T); stem(c, .85, c.B, T); bar(c, mid(c, T), 0, .85); c.s([(.85, c.B), (1, c.B), (1, c.B - 1.5)])

def _che_tick(c, T):
    m = mid(c, T, .45); c.s([(0, T), (0, m, 1), (.85, m)]); stem(c, .85, c.B, T); c.s([(.85, c.B), (1, c.B), (1, c.B - 1.5)])

def _ka_tick(c, T):
    stem(c, 0, c.B, T); c.s([(.85, T), (0, mid(c, T, .45)), (.85, c.B), (1, c.B), (1, c.B - 1.5)])

def _zhe_tick(c, T):
    m = mid(c, T); stem(c, .45, c.B, T)
    c.s([(0, T), (.45, m), (0, c.B)]); c.s([(.9, T), (.45, m), (.9, c.B), (1, c.B), (1, c.B - 1.5)])

def _ha_tick(c, T):
    c.s([(0, T), (.85, c.B), (1, c.B), (1, c.B - 1.5)]); c.s([(.85, T), (0, c.B)])

def _ue(c, T):       # straight Ү
    m = mid(c, T, .45); c.s([(0, T), (.5, m), (1, T)]); stem(c, .5, c.B if T == c.C else c.D, m)

def _ue_bar(c, T):
    _ue(c, T); y = mid(c, T, .25) if T == c.C else c.B - 1
    bar(c, y, .15, .85)

W7, W6 = (7, 7), (6, 5)
for _up, _lo, _wu, _wl, _fn in (
        ("Ғ", "ғ", (7, 6), (6, 5), _ghe_bar), ("Қ", "қ", (8, 7), (7, 6), _ka_tick), ("Ң", "ң", (8, 8), (7, 6), _en_tick),
        ("Ҳ", "ҳ", (8, 8), (7, 6), _ha_tick), ("Ҷ", "ҷ", (8, 7), (7, 6), _che_tick), ("Җ", "җ", (11, 10), (10, 8), _zhe_tick),
        ("Ө", "ө", W7, W6, _oe), ("Һ", None, (7, 6), None, _shha), ("Ҫ", "ҫ", (7, 6), (5, 5), _es_tail),
        ("Ҙ", "ҙ", (6, 6), (5, 4), _ze_tail), ("Ҡ", "ҡ", (8, 7), (7, 6), _bashkir_ka),
        ("Ү", "ү", W7, W6, _ue), ("Ұ", "ұ", W7, W6, _ue_bar)):
    both(_up, _lo, _wu, _wl, _fn)
same("һӀӏӔӕ", "hIIÆæ")
auto_composites(range(0x400, 0x530))


# ======================================================== GREEK POLYTONIC
auto_composites(range(0x1F00, 0x2000))
same("ι", "ι")


# ================================================================== HEBREW
def heb(fn):
    """Hebrew letters: heavy bars, light stems (the reverse of Latin), a little above x-height."""
    def draw(c):
        if c.hx > c.hy:
            c.use_pen(c.hy, c.hx)
        c.T = c.f.xh + 1 - c.hy
        c.M = c.snap((c.B + c.T) / 2)
        fn(c)
    return draw

HEB = {
    "א": (7, 6, lambda c: (c.s([(.05, c.T), (.95, c.B)]), c.s([(1, c.T), (1, c.M + .5), (.62, c.M)]), c.s([(0, c.B), (0, c.M - .5), (.38, c.M)]))),
    "ב": (6, 5, lambda c: (c.s([(0, c.T), (.8, c.T, .8), (.8, c.B)]), bar(c, c.B))),
    "ג": (5, 4, lambda c: (c.s([(.1, c.T), (.75, c.T, .5), (.75, c.B)]), c.s([(.75, c.M - .5), (0, c.B)]))),
    "ד": (6, 5, lambda c: (bar(c, c.T), stem(c, .8, c.B, c.T))),
    "ה": (7, 6, lambda c: (c.s([(0, c.T), (1, c.T, .8), (1, c.B)]), stem(c, 0, c.B, c.T - 2.5))),
    "ו": (3, 2, lambda c: c.s([(0, c.T), (1, c.T, .5), (1, c.B)])),
    "ז": (4, 3, lambda c: (bar(c, c.T), stem(c, .5, c.B, c.T))),
    "ח": (7, 6, lambda c: c.s([(0, c.B), (0, c.T), (1, c.T), (1, c.B)])),
    "ט": (7, 6, lambda c: c.s([(0, c.T), (0, c.B, 1), (1, c.B, 1), (1, c.T, .8), (.5, c.T), (.5, c.T - 1.8)])),
    "י": (3, 2, lambda c: c.s([(0, c.T), (1, c.T, .5), (1, c.T - 3)])),
    "כ": (6, 5, lambda c: c.s([(0, c.T), (1, c.T, 1), (1, c.B, 1), (0, c.B)])),
    "ך": (6, 5, lambda c: c.s([(0, c.T), (1, c.T, .5), (1, c.D)])),
    "ל": (6, 5, lambda c: c.s([(0, c.A), (0, c.T), (1, c.T, .5), (1, c.M), (.4, c.B)])),
    "מ": (7, 6, lambda c: (c.s([(0, c.T), (.18, c.T - 1), (.18, c.B)]), c.s([(.18, c.T - 1), (.42, c.T), (1, c.T, .8), (1, c.B), (.5, c.B)]))),
    "ם": (7, 6, lambda c: c.s([(0, c.T), (1, c.T, .7), (1, c.B), (0, c.B)], closed=True)),
    "נ": (4, 3, lambda c: c.s([(.15, c.T), (1, c.T, .5), (1, c.B), (0, c.B)])),
    "ן": (3, 2, lambda c: c.s([(0, c.T), (1, c.T, .5), (1, c.D)])),
    "ס": (7, 6, lambda c: c.s([(0, c.T, .3), (1, c.T, .8), (1, c.B, 1.2), (0, c.B, 1.2)], closed=True)),
    "ע": (7, 6, lambda c: (c.s([(1, c.T), (1, c.B, 1.2), (0, c.B)]), c.s([(.3, c.T), (.45, c.B)]))),
    "פ": (6, 5, lambda c: c.s([(0, c.B), (1, c.B, 1), (1, c.T, 1), (0, c.T), (0, c.M), (.45, c.M)])),
    "ף": (6, 5, lambda c: c.s([(.45, c.M), (0, c.M), (0, c.T), (1, c.T, .5), (1, c.D)])),
    "צ": (6, 5, lambda c: (c.s([(.05, c.T), (.9, c.B)]), bar(c, c.B), c.s([(1, c.T), (.6, c.M)]))),
    "ץ": (6, 5, lambda c: (c.s([(0, c.T), (.5, c.M), (.5, c.D)]), c.s([(1, c.T), (.5, c.M)]))),
    "ק": (6, 5, lambda c: (c.s([(0, c.T), (1, c.T, .8), (1, c.M, .8), (.55, c.B)]), stem(c, 0, c.D, c.T - 2.5))),
    "ר": (6, 5, lambda c: c.s([(0, c.T), (1, c.T, .8), (1, c.B)])),
    "ש": (8, 7, lambda c: (c.s([(0, c.T), (0, c.B, 1), (1, c.B, 1), (1, c.T)]), stem(c, .5, c.B, c.T))),
    "ת": (7, 6, lambda c: (c.s([(.25, c.T), (1, c.T, .8), (1, c.B)]), c.s([(.25, c.T), (.25, c.B, .6), (0, c.B)]))),
    "־": (4, 3, lambda c: bar(c, c.T)),
    "₪": (9, 8, lambda c: (c.s([(0, c.B), (0, c.T), (.6, c.T, .5), (.6, c.M - .5)]), c.s([(1, c.T), (1, c.B), (.4, c.B, .5), (.4, c.M + .5)]))),
}
for _ch, (_wc, _wa, _fn) in HEB.items():
    reg(_ch, heb(_fn), _wc, _wa)
same("׳״", "'\"")


def niqqud(dots=(), bars=(), above=False):
    """Zero-width vowel point; the shaper centres it under (or over) the letter."""
    def draw(c):
        c.zero = True; d = 1.0
        y0 = c.f.xh + 2 if above else -1 - d
        for dx, dy in dots:
            c.rect(dx, y0 + dy, dx + d, y0 + dy + d)
        for x0, x1, dy in bars:
            c.rect(x0, y0 + dy, x1, y0 + dy + d)
    return draw

for _cp, _args in {
        0x5B0: dict(dots=[(1, 0), (1, -2)]), 0x5B4: dict(dots=[(1, 0)]), 0x5B5: dict(dots=[(0, 0), (2, 0)]),
        0x5B6: dict(dots=[(0, 0), (2, 0), (1, -2)]), 0x5B7: dict(bars=[(0, 3, 0)]), 0x5B8: dict(bars=[(0, 3, 0), (1, 2, -1)]),
        0x5B9: dict(dots=[(0, 0)], above=True), 0x5BB: dict(dots=[(0, 0), (1, -1), (2, -2)]), 0x5BC: dict(dots=[(1, 4)]),
        0x5C1: dict(dots=[(3, 0)], above=True), 0x5C2: dict(dots=[(0, 0)], above=True), 0x5C7: dict(bars=[(0, 3, 0), (1, 2, -1)]),
        0x5B1: dict(dots=[(0, 0), (2, 0), (1, -2), (4, 0), (4, -2)]), 0x5B2: dict(bars=[(0, 3, 0)], dots=[(4, 0), (4, -2)]),
        0x5B3: dict(bars=[(0, 3, 0), (1, 2, -1)], dots=[(4, 0), (4, -2)]), 0x5BD: dict(bars=[(1, 2, 0), (1, 2, -1)]),
        0x5BF: dict(bars=[(0, 3, 0)], above=True)}.items():
    reg(chr(_cp), niqqud(**_args), 3, 3)


# ================================================================= SYMBOLS
def _even(c, f=1.0):
    return dict(hx=c.m * f, hy=c.m * f)

def _numero(c):
    x = c.place("N", 1, 0, 0); c.place("º", 1, x, 0)
    y = c.f.cap * .4 - 1
    c.s([(x + c.lsb, y), (x + c.lsb + 3, y)], absx=True, hx=c.hy, hy=c.hy)

GLYPHS["№"] = (_numero, {k: GLYPHS["N"][1][k] + 5 for k in ("ch", "ag")}, {"ch": (0, 0), "ag": (0, 0)})
reg("₽", lambda c: (stem(c, .15, c.B, c.C), c.s([(.15, c.C), (1, c.C, 1), (1, c.cy(.45), 1), (.15, c.cy(.45))]), bar(c, c.cy(.45), 0, .5), bar(c, c.cy(.2), 0, .7)), 8, 7)
reg("₴", lambda c: (c.s([(0, c.cy(.8)), (0, c.C, 1), (1, c.C, 1), (1, c.cy(.62), 1), (0, c.cy(.38), 1), (0, c.B, 1), (1, c.B, 1), (1, c.cy(.2))]), bar(c, c.cy(.62), 0, 1), bar(c, c.cy(.38), 0, 1)), 7, 6)
reg("₸", lambda c: (bar(c, c.C), bar(c, c.cy(.72)), stem(c, .5, c.B, c.cy(.72))), 6, 7)
reg("₩", lambda c: (base("W")(c), bar(c, c.cy(.55)), bar(c, c.cy(.3))), 11, 11)
reg("₹", lambda c: (bar(c, c.C), bar(c, c.cy(.72)), c.s([(.1, c.C), (.6, c.C, 1), (.6, c.cy(.45), 1), (.1, c.cy(.45)), (.9, c.B)])), 7, 6)
reg("₺", lambda c: (c.s([(.25, c.C), (.25, c.B), (.7, c.B, 1.5), (1, c.cy(.45))]), c.s([(0, c.cy(.55)), (.7, c.cy(.72))], **_even(c)), c.s([(0, c.cy(.35)), (.7, c.cy(.52))], **_even(c))), 7, 6)

def _dong(c):
    base("đ")(c); c.s([(c.lsb, -1.5), (c.lsb + c.ink, -1.5)], absx=True, hx=c.hy, hy=c.hy)

GLYPHS["₫"] = (_dong, GLYPHS["đ"][1], GLYPHS["đ"][2])

def _permille(c):
    x = c.place("%", 1, 0, 0); r = 3 if c.hx < 1 else 6      # same ring as the percent sign's
    x0 = x + c.lsb - 1
    c.s([(x0 + c.hx, c.B, .8), (x0 + r - c.hx, c.B, .8), (x0 + r - c.hx, c.B + 3, .8), (x0 + c.hx, c.B + 3, .8)], closed=True, absx=True)

GLYPHS["‰"] = (_permille, {k: GLYPHS["%"][1][k] + (7 if k == "ch" else 4) for k in ("ch", "ag")}, {"ch": (0, 0), "ag": (0, 0)})
reg("′", lambda c: c.s([(1, c.C), (0, c.C - 3)], **_even(c, .8)), 3, 2)
reg("″", lambda c: (c.s([(.45, c.C), (0, c.C - 3)], **_even(c, .8)), c.s([(1, c.C), (.55, c.C - 3)], **_even(c, .8))), 5, 4)
reg("≠", lambda c: (base("=")(c), c.s([(.25, c.cy(.45) - 2.5), (.75, c.cy(.45) + 2.5)], **_even(c, .8))), 6, 5)
reg("≤", lambda c: (c.s([(1, c.cy(.45) + 3), (0, c.cy(.45) + 1), (1, c.cy(.45) - 1)], **_even(c)), c.s([(0, c.B + c.m - c.hy), (1, c.B + c.m - c.hy)], **_even(c))), 6, 5)
reg("≥", lambda c: (c.s([(0, c.cy(.45) + 3), (1, c.cy(.45) + 1), (0, c.cy(.45) - 1)], **_even(c)), c.s([(0, c.B + c.m - c.hy), (1, c.B + c.m - c.hy)], **_even(c))), 6, 5)

def _arrow(dx, dy):
    def draw(c):
        m = c.cy(.45); k = _even(c)
        if dx:
            c.s([(0, m), (1, m)], **k); t = 1 if dx > 0 else 0; u = .62 if dx > 0 else .38
            c.s([(u, m + 2), (t, m), (u, m - 2)], **k)
        else:
            c.s([(.5, m - 3.5), (.5, m + 3.5)], **k); y = m + 3.5 * dy
            c.s([(0, y - 2.2 * dy), (.5, y), (1, y - 2.2 * dy)], **k)
    return draw

for _ch, _d, _w in (("←", (-1, 0), (8, 7)), ("→", (1, 0), (8, 7)), ("↑", (0, 1), (6, 5)), ("↓", (0, -1), (6, 5))):
    reg(_ch, _arrow(*_d), *_w)


# ========================================================= COMBINING MARKS
# Zero-width marks for decomposed text (and for fontconfig's Vietnamese coverage test).
# They are drawn just left of the origin, over the preceding letter; the shaper refines the position.
from latin import COMB, SPACING   # noqa: E402

def _combining(mark):
    def draw(c):
        c.zero = True
        mark(c, -3.0, c.f.xh + c.f.mark_gap)
    return draw

for _cp, _m in COMB.items():
    reg(chr(_cp), _combining(_m), 0, 0, chsb=(0, 0), agsb=(0, 0))
same("\u02c9\u02ca\u02cb", "\u00af\u00b4`")      # spacing tone marks used with pinyin
