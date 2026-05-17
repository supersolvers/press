"""Latin beyond ASCII: accents, composites (generated from Unicode
decompositions), special letters, and typographic punctuation."""
import math
import unicodedata

from glyphs import GLYPHS, g, stem, bar, ring


# ------------------------------------------------------------------ marks
def _mp(c):
    p = max(.5, .85 * c.m)
    return p, dict(hx=p, hy=p, absx=True)


def _dims(c):
    return (1.6 if c.hx >= 1 else 1.3) * getattr(c, "mark_scale", 1.0), 2.0     # half-width, height


def _dotsize(c):
    return max(1.0, 1.6 * c.hx)


def m_grave(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); c.s([(cx - w, y + h - p), (cx + .3 * w, y + p)], **k)

def m_acute(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); c.s([(cx - .3 * w, y + p), (cx + w, y + h - p)], **k)

def m_circ(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); c.s([(cx - w, y + p), (cx, y + h - p), (cx + w, y + p)], **k)

def m_caron(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); c.s([(cx - w, y + h - p), (cx, y + p), (cx + w, y + h - p)], **k)

def m_tilde(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); lo, hi = y + p, y + h - p
    c.s([(cx - w - .4, lo), (cx - w / 2, hi, .6), (cx, (lo + hi) / 2), (cx + w / 2, lo, .6), (cx + w + .4, hi)], **k)

def m_macron(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); c.s([(cx - w, y + h / 2), (cx + w, y + h / 2)], **k)

def m_breve(c, cx, y):
    p, k = _mp(c); w, h = _dims(c)
    c.s([(cx - w, y + h - p), (cx - w, y + p, .7), (cx + w, y + p, .7), (cx + w, y + h - p)], **k)

def m_ring(c, cx, y):
    p, k = _mp(c); w = 1.1
    c.s([(cx - w, y + p, .6), (cx + w, y + p, .6), (cx + w, y + 2 * w + p, .6), (cx - w, y + 2 * w + p, .6)], closed=True, hx=.45, hy=.45, absx=True)

# Dots are whole pixels on whole-pixel positions. A dot of a fraction of a pixel, or a gap of one
# at a half-pixel offset, comes out of the one-bit rendering as a bar: two dots so drawn read as a
# breve or a macron, and ä could not be told from ă or ā.
def _pixel_dot(c):
    return max(1, round(_dotsize(c)))

def m_dot(c, cx, y):
    d = _pixel_dot(c); x0 = round(cx - d / 2); c.rect(x0, y, x0 + d, y + d)

def m_dieresis(c, cx, y):
    d = _pixel_dot(c); gap = d          # in a heavy face the dots are two pixels and so is the gap
    x0 = round(cx - (2 * d + gap) / 2)
    for x in (x0, x0 + d + gap):
        c.rect(x, y, x + d, y + d)

def m_hung(c, cx, y):
    m_acute(c, cx - 1.0, y); m_acute(c, cx + 1.0, y)

def m_cedilla(c, cx, y):
    p, k = _mp(c); c.s([(cx, .3), (cx + .7, -1.0), (cx - .8, -2.0)], **k)

def m_ogonek(c, cx, y):
    p, k = _mp(c); x0 = c.lsb + c.ink - 1.0 if c.ink > 3 else cx
    c.s([(x0, .3), (x0 - .9, -1.1, .8), (x0 + .4, -2.2 + p)], **k)

def m_commabelow(c, cx, y):
    p, k = _mp(c); c.s([(cx + .35, -.9), (cx - .45, -2.6)], **k)

def m_commaabove(c, cx, y):
    p, k = _mp(c); c.s([(cx + .45, y + 2), (cx - .35, y + .3)], **k)

def m_dotbelow(c, cx, y):
    d = _pixel_dot(c); x0 = round(cx - d / 2); c.rect(x0, -1 - d, x0 + d, -1)

def m_caronalt(c, cx, y):
    p, k = _mp(c); xr = c.lsb + c.ink
    top = y - c.f.mark_gap
    c.s([(xr + 1.0, top), (xr + .5, top - 2.2)], **k)
    c.rsb += 1


def m_hook(c, cx, y):
    p, k = _mp(c); w, h = _dims(c); lo, hi = y + p, y + h - p
    c.s([(cx - .8, hi), (cx + .5, hi, .7), (cx + .5, (lo + hi) / 2 + .1, .7), (cx - .2, lo)], **k)

def m_horn(c, cx, y):
    p, k = _mp(c); xr = c.lsb + c.ink; t = y - c.f.mark_gap
    c.s([(xr - c.hx, t - .5), (xr + .9, t + 1.0)], **k)
    c.rsb += 1

def m_psili(c, cx, y):
    p, k = _mp(c); lo, hi = y + p, y + 2 - p
    c.s([(cx - .5, hi), (cx + .4, hi, .8), (cx + .4, lo + .2, .8), (cx - .4, lo)], **k)

def m_dasia(c, cx, y):
    p, k = _mp(c); lo, hi = y + p, y + 2 - p
    c.s([(cx + .5, hi), (cx - .4, hi, .8), (cx - .4, lo + .2, .8), (cx + .4, lo)], **k)

def m_iotasub(c, cx, y):
    p, k = _mp(c); c.s([(cx, -1.0), (cx, -2.6)], **k)


COMB = {0x300: m_grave, 0x301: m_acute, 0x302: m_circ, 0x303: m_tilde, 0x304: m_macron,
        0x306: m_breve, 0x307: m_dot, 0x308: m_dieresis, 0x30A: m_ring, 0x30B: m_hung,
        0x30C: m_caron, 0x323: m_dotbelow, 0x326: m_commabelow, 0x327: m_cedilla, 0x328: m_ogonek,
        0x309: m_hook, 0x31B: m_horn, 0x313: m_psili, 0x314: m_dasia, 0x342: m_tilde, 0x345: m_iotasub}
BELOW = (m_dotbelow, m_commabelow, m_cedilla, m_ogonek, m_horn, m_iotasub, m_caronalt)   # not stacked above the letter
NARROW = (m_psili, m_dasia, m_acute, m_grave, m_hook)
WIDE = (m_circ, m_caron, m_tilde, m_macron, m_breve, m_dieresis, m_hung)
SPACING = {"`": None, "´": m_acute, "¨": m_dieresis, "¯": m_macron, "¸": m_cedilla, "ˆ": m_circ,
           "ˇ": m_caron, "˘": m_breve, "˙": m_dot, "˚": m_ring, "˛": m_ogonek, "˜": m_tilde, "˝": m_hung}


def top_of(c, base):
    f = c.f
    if base.isupper():
        return f.cap + f.mark_gap
    if base in "bdfhklt":
        return f.asc + f.mark_gap
    return f.xh + f.mark_gap


def _half(c, mark):
    w = _dims(c)[0]
    return getattr(mark, "half", None) or ((w + .5) if mark in WIDE else (.9 * w if mark in NARROW else 0))


def _height(c, mark):
    return _pixel_dot(c) + 1 if mark in (m_dot, m_dieresis) else 2.0


def place_above(c, marks, cx, y):
    """One accent, or two combined the way the language expects."""
    if len(marks) == 1:
        marks[0](c, cx, y); return
    a, b = marks[0], marks[1]
    if (a is m_circ and b in (m_acute, m_grave, m_hook)) or (a in (m_psili, m_dasia) and b in (m_acute, m_grave)):
        # side by side: Vietnamese tone beside the circumflex, Greek accent beside the breathing
        w = _dims(c)[0]
        if a is m_circ:
            a(c, cx - .6, y); c.mark_scale = .75
            b(c, cx + w + (.7 if b is not m_grave else 1.3), y + 1); c.mark_scale = 1.0
        else:
            c.mark_scale = .75; a(c, cx - 1.0, y); b(c, cx + 1.0, y); c.mark_scale = 1.0
    else:                                     # stacked: tilde over circumflex, tone over breve, ...
        a(c, cx, y); b(c, cx, y + _height(c, a))
    for extra in marks[2:]:
        extra(c, cx, y + 4)


def composite(ch, base, marks):
    if not isinstance(marks, (list, tuple)):
        marks = [marks]
    above = [m for m in marks if m not in BELOW]
    draw_base = {"i": "ı", "j": "ȷ", "і": "ı", "ј": "ȷ"}.get(base, base) if above else base
    fn, inks, sbs = GLYPHS[draw_base]
    greek_cap = "\u0391" <= base <= "\u03a9" and any(m in NARROW or m is m_tilde for m in above)

    def draw(c, fn=fn, base=base, marks=marks, above=above):
        if greek_cap:                         # accents and breathings stand to the left of a capital
            x0 = c.lsb; c.lsb += math.ceil(1.5 * len(above) + .4)
            fn(c); c.mark_scale = .75
            for i, m in enumerate(above):
                m(c, x0 + 1.0 + 1.6 * i, c.f.cap - 2.4)
            c.mark_scale = 1.0
        else:
            half = max([_half(c, m) for m in above] + [0])
            if len(above) > 1 and above[0] is m_circ and above[1] in (m_acute, m_grave, m_hook):
                half += .6
            need = half - (c.ink / 2 + c.lsb) + .25
            if need > 0:                      # narrow letter: make room for the accent (whole px)
                need = math.ceil(need - .01); c.lsb += need; c.rsb += need
            fn(c)
            if above:
                place_above(c, above, c.lsb + c.ink / 2, top_of(c, base))
        for m in marks:
            if m in BELOW:
                m(c, c.lsb + c.ink / 2, top_of(c, base))
    GLYPHS[ch] = (draw, inks, sbs)


# ---------------------------------------------------------- extra letters
@g("ı", 2, 1, agsb=(1, 1), mono=5)
def _(c):
    if c.mono:
        c.s([(0, c.X), (.5, c.X), (.5, c.B)]); bar(c, c.B); return
    stem(c, .5, c.B, c.X)

@g("ȷ", 4, 2, agsb=(0, 1), mono=5)
def _(c):
    if c.mono:
        c.s([(.25, c.X), (.75, c.X), (.75, c.D, 1), (0, c.D)]); return
    c.s([(1, c.X), (1, c.D, 1), (0, c.D)])

@g("ſ", 5, 4)
def _(c):
    c.s([(1, c.A), (.34, c.A, 1), (.34, c.B)])

@g("Æ", 11, 10)
def _(c):
    c.s([(0, c.B), (.5, c.C)]); stem(c, .5, c.B, c.C)
    bar(c, c.C, .5, 1); bar(c, c.B, .5, 1); bar(c, c.cy(.5), .5, .9); bar(c, c.cy(.3), .18, .5)

@g("æ", 10, 8)
def _(c):
    m = c.xy(.55); e = c.xy(.5)
    c.s([(.06, c.X), (.5, c.X, 1), (.5, c.B)]); c.s([(.5, m), (0, m, 1), (0, c.B, 1), (.5, c.B)])
    c.s([(.5, e), (1, e), (1, c.X, 1), (.5, c.X)]); c.s([(.5, c.xy(.3)), (.5, c.B, 1), (1, c.B)])

@g("Œ", 11, 10)
def _(c):
    c.s([(.55, c.C), (0, c.C, 1), (0, c.B, 1), (.55, c.B)]); stem(c, .55, c.B, c.C)
    bar(c, c.C, .55, 1); bar(c, c.B, .55, 1); bar(c, c.cy(.5), .55, .92)

@g("œ", 10, 8)
def _(c):
    e = c.xy(.5); ring(c, c.B, c.X, 0, .5)
    c.s([(.5, e), (1, e), (1, c.X, 1), (.5, c.X, 1), (.5, c.B, 1), (1, c.B)])

@g("Ø", 7, 7)
def _(c):
    ring(c, c.B, c.C); c.s([(0, c.B - .8), (1, c.C + .8)], hx=c.hy, hy=c.hy)

@g("ø", 6, 5)
def _(c):
    ring(c, c.B, c.X); c.s([(0, c.B - .8), (1, c.X + .8)], hx=c.hy, hy=c.hy)

@g(["Ð", "Đ"], 8, 8)
def _(c):
    c.s([(.14, c.B), (.14, c.C), (1, c.C, 1.4), (1, c.B, 1.4)], closed=True); bar(c, c.cy(.5), 0, .5)

@g("ð", 6, 5)
def _(c):
    ring(c, c.B, c.X); c.s([(1, c.xy(.5)), (1, c.A - 1.2, 1.2), (.3, c.A)])
    c.s([(.3, c.A - 1.6), (1, c.A - .2)], hx=c.hy, hy=c.hy)

@g("đ", 6, 5, chsb=(0, 2), agsb=(0, 2))
def _(c):
    GLYPHS["d"][0](c); y = (c.f.xh + c.f.asc) / 2 + .2
    c.s([(c.lsb + c.ink - 2 * c.hx - 1.2, y), (c.lsb + c.ink + 1, y)], absx=True, hx=c.hy, hy=c.hy)

@g("Þ", 7, 6)
def _(c):
    stem(c, 0, c.B, c.C); c.s([(0, c.cy(.78)), (1, c.cy(.78), 1), (1, c.cy(.22), 1), (0, c.cy(.22))])

@g("þ", 6, 5)
def _(c):
    stem(c, 0, c.D, c.A); c.s([(0, c.X), (1, c.X, 1), (1, c.B, 1), (0, c.B)])

@g("ß", 7, 6)
def _(c):
    m = c.ay(.58)
    c.s([(0, c.B), (0, c.A, 1), (.82, c.A, 1), (.82, m, .8), (.42, m)])
    c.s([(.42, m), (1, m, 1), (1, c.B, 1), (.38, c.B)])

@g("Ł", 7, 6)
def _(c):
    c.s([(.16, c.C), (.16, c.B), (1, c.B)]); c.s([(0, c.cy(.32)), (.5, c.cy(.62))], hx=c.m, hy=c.m)

@g("ł", 4, 3)
def _(c):
    stem(c, .5, c.B, c.A); c.s([(0, c.ay(.38)), (1, c.ay(.62))], hx=c.m * .9, hy=c.m * .9)

@g("Ħ", 8, 8)
def _(c):
    stem(c, .12, c.B, c.C); stem(c, .88, c.B, c.C); bar(c, c.cy(.45), .12, .88); bar(c, c.cy(.78))

@g("ħ", 6, 5)
def _(c):
    GLYPHS["h"][0](c); y = (c.f.xh + c.f.asc) / 2 + .2
    c.s([(c.lsb - .6, y), (c.lsb + 2 * c.hx + 1.2, y)], absx=True, hx=c.hy, hy=c.hy)

@g("Ŋ", 7, 7)
def _(c):
    stem(c, 0, c.B, c.C); c.s([(0, c.C), (1, c.C, 1.2), (1, c.D + 1, 1), (.45, c.D + 1)])

@g("ŋ", 6, 5)
def _(c):
    stem(c, 0, c.B, c.X); c.s([(0, c.X), (1, c.X, 1), (1, c.D, 1), (.4, c.D)])

@g("ĸ", 6, 5)
def _(c):
    stem(c, 0, c.B, c.X); c.s([(1, c.X), (0, c.xy(.45)), (1, c.B)])

@g("Ŧ", 6, 7)
def _(c):
    bar(c, c.C); stem(c, .5, c.B, c.C); bar(c, c.cy(.5), .2, .8)

@g("ŧ", 5, 4)
def _(c):
    GLYPHS["t"][0](c); bar(c, c.xy(.5), 0, .8)

@g("µ", 6, 5)
def _(c):
    stem(c, 0, c.D, c.X); stem(c, 1, c.B, c.X); c.s([(0, c.X), (0, c.B, 1), (1, c.B)])


@g("ə", 6, 5)
def _(c):
    c.rot180("e", c.f.xh)


# ------------------------------------------------- punctuation and symbols
def _even(c, f=1.0):
    return dict(hx=c.m * f, hy=c.m * f)


@g("¡", 2, 1, agsb=(1, 1))
def _(c):
    c.rot180("!", c.f.xh)

@g("¿", 6, 5)
def _(c):
    c.rot180("?", c.f.xh)

@g(["–", "−"], 6, 5)
def _(c):
    c.s([(0, c.cy(.45)), (1, c.cy(.45))], **_even(c))

@g("—", 12, 11, chsb=(0, 0), agsb=(0, 0))
def _(c):
    c.s([(0, c.cy(.45)), (1, c.cy(.45))], **_even(c))

def _quote(c, cx, ytop, close):
    """Raised comma: head with a tail, pointing down (closing) or up (opening)."""
    d = max(1.0, 1.7 * c.hx); p = max(.3, c.hx * .55)
    if close:
        c.rect(cx - d / 2, ytop - d, cx + d / 2, ytop)
        c.s([(cx + d / 2 - p, ytop - d + .2), (cx + d / 2 - p - .5, ytop - d - 1.3)], hx=p, hy=p, absx=True)
    else:
        c.rect(cx - d / 2, ytop - d - 1.3, cx + d / 2, ytop - 1.3)
        c.s([(cx - d / 2 + p, ytop - 1.5), (cx - d / 2 + p + .5, ytop)], hx=p, hy=p, absx=True)

@g("‘", 2, 1, agsb=(1, 1))
def _(c):
    _quote(c, c.lsb + c.ink / 2, c.f.cap, False)

@g("’", 2, 1, agsb=(1, 1))
def _(c):
    _quote(c, c.lsb + c.ink / 2, c.f.cap, True)

@g("‚", 2, 1, agsb=(1, 1))
def _(c):
    _quote(c, c.lsb + c.ink / 2, max(1.0, 1.7 * c.hx), True)

@g("“", 5, 3, agsb=(1, 1))
def _(c):
    w = (c.ink - max(1.0, 1.7 * c.hx)) / 2
    _quote(c, c.lsb + c.ink / 2 - w, c.f.cap, False); _quote(c, c.lsb + c.ink / 2 + w, c.f.cap, False)

@g("”", 5, 3, agsb=(1, 1))
def _(c):
    w = (c.ink - max(1.0, 1.7 * c.hx)) / 2
    _quote(c, c.lsb + c.ink / 2 - w, c.f.cap, True); _quote(c, c.lsb + c.ink / 2 + w, c.f.cap, True)

@g("„", 5, 3, agsb=(1, 1))
def _(c):
    w = (c.ink - max(1.0, 1.7 * c.hx)) / 2; t = max(1.0, 1.7 * c.hx)
    _quote(c, c.lsb + c.ink / 2 - w, t, True); _quote(c, c.lsb + c.ink / 2 + w, t, True)

@g("‹", 3, 3)
def _(c):
    m = c.xy(.5); c.s([(1, m + 2), (0, m), (1, m - 2)], **_even(c))

@g("›", 3, 3)
def _(c):
    m = c.xy(.5); c.s([(0, m + 2), (1, m), (0, m - 2)], **_even(c))

@g("«", 7, 6)
def _(c):
    m = c.xy(.5)
    for o in (0, .5):
        c.s([(o + .5, m + 2), (o, m), (o + .5, m - 2)], **_even(c))

@g("»", 7, 6)
def _(c):
    m = c.xy(.5)
    for o in (0, .5):
        c.s([(o, m + 2), (o + .5, m), (o, m - 2)], **_even(c))

@g("…", 10, 7, agsb=(1, 1))
def _(c):
    for t in (0, .5, 1):
        c.dot(t, 0)

@g("·", 2, 1, agsb=(1, 1))
def _(c):
    c.dot(.5, c.cy(.45) - c.hx)

@g("•", 4, 3, agsb=(1, 1))
def _(c):
    c.disc(c.lsb + c.ink / 2, c.cy(.45), c.ink / 2)

@g("°", 4, 3)
def _(c):
    c.s([(0, c.C - 2.4, .8), (1, c.C - 2.4, .8), (1, c.C, .8), (0, c.C, .8)], closed=True, hx=.5, hy=.5)

@g("¦", 2, 1, agsb=(1, 1))
def _(c):
    m = (c.D + c.C + 1) / 2; stem(c, .5, c.D, m - 1); stem(c, .5, m + 1, c.C + 1)

@g("†", 6, 5)
def _(c):
    stem(c, .5, c.D + 1, c.C); bar(c, c.cy(.7))

@g("‡", 6, 5)
def _(c):
    stem(c, .5, c.D + 1, c.C); bar(c, c.cy(.72)); bar(c, c.cy(.18))

@g("§", 6, 5)
def _(c):
    a, b = c.cy(.68), c.cy(.28)
    ring(c, b, a); c.s([(1, c.C + .5), (0, c.C + .5, 1), (0, a)]); c.s([(0, c.D + 1), (1, c.D + 1, 1), (1, b)])

@g("¶", 7, 6)
def _(c):
    stem(c, .6, c.D + 1, c.C); stem(c, 1, c.D + 1, c.C)
    c.s([(1, c.C), (0, c.C, 1), (0, c.cy(.5), 1), (.6, c.cy(.5))])

@g("¬", 6, 5)
def _(c):
    m = c.cy(.45); c.s([(0, m), (1, m), (1, m - 2)], **_even(c))

@g("±", 6, 5)
def _(c):
    GLYPHS["+"][0](c); c.s([(0, c.B + c.m - c.hy), (1, c.B + c.m - c.hy)], **_even(c))

@g("×", 6, 5)
def _(c):
    m = c.cy(.45); c.s([(0, m + 2.2), (1, m - 2.2)], **_even(c)); c.s([(0, m - 2.2), (1, m + 2.2)], **_even(c))

@g("÷", 6, 5)
def _(c):
    m = c.cy(.45); d = max(1.0, 1.5 * c.m)
    c.s([(0, m), (1, m)], **_even(c)); cx = c.lsb + c.ink / 2
    c.rect(cx - d / 2, m + 1.4, cx + d / 2, m + 1.4 + d); c.rect(cx - d / 2, m - 1.4 - d, cx + d / 2, m - 1.4)

@g("¢", 5, 5)
def _(c):
    GLYPHS["c"][0](c); c.s([(.5, c.B - 1.2), (.5, c.X + 1.2)], hx=c.hx * .55)

@g("£", 7, 6)
def _(c):
    c.s([(1, c.cy(.75)), (1, c.C, 1), (.3, c.C, 1), (.3, c.B)]); bar(c, c.B); bar(c, c.cy(.45), 0, .72)

@g("¥", 6, 7)
def _(c):
    GLYPHS["Y"][0](c); bar(c, c.cy(.42), .1, .9); bar(c, c.cy(.2), .1, .9)

@g("€", 8, 7)
def _(c):
    c.s([(1, c.cy(.78)), (1, c.C, 1), (.2, c.C, 1), (.2, c.B, 1), (1, c.B, 1), (1, c.cy(.22))])
    bar(c, c.cy(.6), 0, .7); bar(c, c.cy(.38), 0, .7)

@g("¤", 6, 5)
def _(c):
    a, b = c.cy(.2), c.cy(.75); ring(c, a, b, .12, .88)
    for x0, y0, x1, y1 in ((0, b + .9, .25, b - .4), (1, b + .9, .75, b - .4), (0, a - .9, .25, a + .4), (1, a - .9, .75, a + .4)):
        c.s([(x0, y0), (x1, y1)], **_even(c, .7))


def _sup(c, text, scale, y, x0=None):
    x = c.lsb if x0 is None else x0
    for ch in text:
        x += c.place(ch, scale, x, y)
    return x

for _ch, _src in (("¹", "1"), ("²", "2"), ("³", "3"), ("ª", "a"), ("º", "o")):
    @g(_ch, 4, 3, chsb=(0, 1), agsb=(0, 1))
    def _(c, src=_src):
        c.place(src, .6, c.lsb - .2, c.f.cap * .4)

for _ch, (_n, _d) in {"¼": "14", "½": "12", "¾": "34"}.items():
    @g(_ch, 10, 9)
    def _(c, n=_n, d=_d):
        c.place(n, .55, c.lsb - .2, c.f.cap * .45)
        c.s([(.22, c.B), (.78, c.C)], **_even(c, .7))
        c.place(d, .55, c.lsb + c.ink * .56, 0)

@g(["©", "®"], 10, 9)
def _(c):
    pass  # replaced below (needs the letter)

def _circled(letter):
    def draw(c):
        c.s([(0, c.B, 2.2), (1, c.B, 2.2), (1, c.C, 2.2), (0, c.C, 2.2)], closed=True, hx=.45, hy=.45)
        s = .55; sub = c.sub(letter)
        c.place(letter, s, c.lsb + (c.ink - sub.adv * s) / 2, c.f.cap * (1 - s) / 2)
    return draw

GLYPHS["©"] = (_circled("C"), {"ch": 10, "ag": 9}, {"ch": None, "ag": None})
GLYPHS["®"] = (_circled("R"), {"ch": 10, "ag": 9}, {"ch": None, "ag": None})

@g("™", 10, 9)
def _(c):
    x = c.place("T", .5, c.lsb - .2, c.f.cap * .5); c.place("M", .5, c.lsb - .2 + x, c.f.cap * .5)


# ------------------------------------------------ spacing accents, composites
for _ch, _m in SPACING.items():
    if _m is None:
        continue
    @g(_ch, 4, 3)
    def _(c, m=_m):
        m(c, c.lsb + c.ink / 2, c.f.xh + c.f.mark_gap)

def auto_composites(codepoints):
    """Register every precomposed letter whose base and accents we can draw."""
    bases = set(GLYPHS)
    for cp in codepoints:
        ch = chr(cp)
        nfd = unicodedata.normalize("NFD", ch)
        if len(nfd) < 2 or ch in GLYPHS or nfd[0] not in bases or any(ord(m) not in COMB for m in nfd[1:]):
            continue
        b, marks = nfd[0], [COMB[ord(m)] for m in nfd[1:]]
        if b.isupper() and m_iotasub in marks:
            continue                          # capitals take an adscript iota instead: not drawn yet
        if len(marks) == 1:
            mk = marks[0]
            if mk is m_caron and b in "dltL":
                mk = m_caronalt
            if mk is m_cedilla and b in "GKLNRklnr":
                mk = m_commabelow
            if mk is m_cedilla and b == "g":
                mk = m_commaabove
            marks = [mk]
        composite(ch, b, marks)


auto_composites(list(range(0xC0, 0x250)) + list(range(0x1E00, 0x1F00)))
