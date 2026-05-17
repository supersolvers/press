"""Glyph skeletons shared by all families.

Each glyph is registered with its ink width in px per family column
(ch = heavy Chicago-like face, ag = light MS Sans-like face).
x coordinates are fractions of the ink box; y coordinates are px.
A third value on a point rounds that corner (factor of the family radius).
"""
GLYPHS = {}   # char -> (func, {wkey: ink}, {wkey: (lsb, rsb)})


def g(chars, ch, ag, chsb=None, agsb=None, mono=None):
    def deco(fn):
        for c in ([chars] if isinstance(chars, str) and len(chars) == 1 else chars):
            inks = {"ch": ch, "ag": ag}
            if mono:
                inks["mono"] = mono        # full-cell design in monospaced families
            GLYPHS[c] = (fn, inks, {"ch": chsb, "ag": agsb})
        return fn
    return deco


def stem(c, t, y0, y1):
    c.s([(t, y0), (t, y1)])


def bar(c, y, t0=0, t1=1):
    c.s([(t0, y), (t1, y)])


def ring(c, y0, y1, t0=0, t1=1, r=1):
    c.s([(t0, y0, r), (t1, y0, r), (t1, y1, r), (t0, y1, r)], closed=True)


# ---------------------------------------------------------------- capitals
@g("A", 7, 7)
def _(c):
    if c.ch:      # arched A: upright sides, rounded top
        c.s([(0, c.B), (0, c.C, 1.3), (1, c.C, 1.3), (1, c.B)]); bar(c, c.cy(.35)); return
    c.s([(0, c.B), (.5, c.C), (1, c.B)])
    bar(c, c.cy(.3), .15, .85)

@g("B", 7, 6)
def _(c):
    stem(c, 0, c.B, c.C); m = c.cy(.5)
    c.s([(0, c.C), (.88, c.C, 1), (.88, m, 1), (0, m)])
    c.s([(0, m), (1, m, 1), (1, c.B, 1), (0, c.B)])

@g("C", 7, 6)
def _(c):
    c.s([(1, c.cy(.72)), (1, c.C, 1), (0, c.C, 1), (0, c.B, 1), (1, c.B, 1), (1, c.cy(.28))])

@g("D", 7, 7)
def _(c):
    c.s([(0, c.B), (0, c.C), (1, c.C, 1.4), (1, c.B, 1.4)], closed=True)

@g("E", 6, 6)
def _(c):
    c.s([(1, c.C), (0, c.C), (0, c.B), (1, c.B)]); bar(c, c.cy(.5), 0, .8)

@g("F", 6, 6)
def _(c):
    c.s([(1, c.C), (0, c.C), (0, c.B)]); bar(c, c.cy(.5), 0, .8)

@g("G", 7, 7)
def _(c):
    m = c.cy(.45)
    c.s([(1, c.cy(.75)), (1, c.C, 1), (0, c.C, 1), (0, c.B, 1), (1, c.B, 1), (1, m), (.5, m)])

@g("H", 7, 7)
def _(c):
    stem(c, 0, c.B, c.C); stem(c, 1, c.B, c.C); bar(c, c.cy(.5))

@g("I", 2, 1, agsb=(1, 1), mono=5)
def _(c):
    if c.mono:
        bar(c, c.C); bar(c, c.B)
    stem(c, .5, c.B, c.C)

@g("J", 6, 5)
def _(c):
    c.s([(1, c.C), (1, c.B, 1), (0, c.B, 1), (0, c.cy(.33))])

@g("K", 7, 6)
def _(c):
    stem(c, 0, c.B, c.C); c.s([(1, c.C), (0, c.cy(.45)), (1, c.B)])

@g("L", 6, 5)
def _(c):
    c.s([(0, c.C), (0, c.B), (1, c.B)])

@g("M", 10, 9)
def _(c):
    if c.upright:      # upright stems, hairline V
        stem(c, 0, c.B, c.C); stem(c, 1, c.B, c.C)
        c.s([(0, c.C), (.5, c.cy(.45 if c.mono else .3)), (1, c.C)], hx=min(.55, c.hx + .05), hy=min(.55, c.hx + .05)); return
    c.s([(0, c.B), (0, c.C), (.5, c.B if c.gen else c.cy(.25)), (1, c.C), (1, c.B)])

@g("N", 7, 7)
def _(c):
    if c.upright:
        stem(c, 0, c.B, c.C); stem(c, 1, c.B, c.C); c.s([(0, c.C), (1, c.B)], hx=min(.6, c.hx + .1), hy=min(.6, c.hx + .1)); return
    c.s([(0, c.B), (0, c.C), (1, c.B), (1, c.C)])

@g("O", 7, 7)
def _(c):
    ring(c, c.B, c.C)

@g("P", 7, 6)
def _(c):
    stem(c, 0, c.B, c.C); m = c.cy(.45)
    c.s([(0, c.C), (1, c.C, 1), (1, m, 1), (0, m)])

@g("Q", 7, 7)
def _(c):
    ring(c, c.B, c.C); c.s([(.5, c.cy(.3)), (1, c.B - 1)])

@g("R", 7, 7)
def _(c):
    stem(c, 0, c.B, c.C); m = c.cy(.45)
    c.s([(0, c.C), (1, c.C, 1), (1, m, 1), (0, m)])
    if c.ch:      # leg drops almost vertically
        c.s([(.45, m), (1, m - 1.6), (1, c.B)])
    else:
        c.s([(.45, m), (1, c.B)])

@g("S", 6, 6)
def _(c):
    m = c.cy(.5)
    c.s([(1, c.cy(.8)), (1, c.C, 1), (0, c.C, 1), (0, m, 1), (1, m, 1), (1, c.B, 1), (0, c.B, 1), (0, c.cy(.2))])

@g("T", 6, 7)
def _(c):
    bar(c, c.C); stem(c, .5, c.B, c.C)

@g("U", 7, 7)
def _(c):
    c.s([(0, c.C), (0, c.B, 1), (1, c.B, 1), (1, c.C)])

@g("V", 7, 7)
def _(c):
    if c.upright:      # upright sides tapering to a point
        k = c.cy(.7 if c.mono else .4)      # a longer taper keeps V apart from U in a narrow cell
        c.s([(0, c.C), (0, k), (.5, c.B), (1, k), (1, c.C)]); return
    c.s([(0, c.C), (.5, c.B), (1, c.C)])

@g("W", 11, 11)
def _(c):
    if c.upright:      # three uprights joined at the foot
        c.s([(0, c.C), (0, c.B, 1), (.5, c.B)]); c.s([(.5, c.B), (.5, c.cy(.75))], hx=.5 if c.heavymono else c.hx); c.s([(.5, c.B), (1, c.B, 1), (1, c.C)]); return
    c.s([(0, c.C), (.25, c.B), (.5, c.C if c.gen else c.cy(.8)), (.75, c.B), (1, c.C)])

@g("X", 7, 7)
def _(c):
    if c.upright:
        c.s([(0, c.C), (0, c.cy(.78)), (1, c.cy(.22)), (1, c.B)]); c.s([(1, c.C), (1, c.cy(.78)), (0, c.cy(.22)), (0, c.B)]); return
    c.s([(0, c.C), (1, c.B)]); c.s([(1, c.C), (0, c.B)])

@g("Y", 6, 7)
def _(c):
    m = c.cy(.45)
    if c.ch:
        c.s([(0, c.C), (0, m, 1), (1, m, 1), (1, c.C)]); stem(c, .5, c.B, m); return
    c.s([(0, c.C), (.5, m), (1, c.C)]); stem(c, .5, c.B, m)

@g("Z", 6, 7)
def _(c):
    c.s([(0, c.C), (1, c.C), (0, c.B), (1, c.B)])


# --------------------------------------------------------------- lowercase
@g("a", 6, 5)
def _(c):
    m = c.xy(.55)
    if c.hx >= 1:     # heavy pens get a hooked terminal; at 1 px it would close the counter
        c.s([(0, c.xy(.82)), (0, c.X, 1), (1, c.X, 1), (1, c.B)])
    else:
        c.s([(.1, c.X), (1, c.X, 1), (1, c.B)])
    c.s([(1, m), (0, m, 1), (0, c.B, 1), (1, c.B)])

@g("b", 6, 5)
def _(c):
    stem(c, 0, c.B, c.A); c.s([(0, c.X), (1, c.X, 1), (1, c.B, 1), (0, c.B)])

@g("c", 5, 5)
def _(c):
    c.s([(1, c.xy(.8)), (1, c.X, 1), (0, c.X, 1), (0, c.B, 1), (1, c.B, 1), (1, c.xy(.2))])

@g("d", 6, 5)
def _(c):
    stem(c, 1, c.B, c.A); c.s([(1, c.X), (0, c.X, 1), (0, c.B, 1), (1, c.B)])

@g("e", 6, 5)
def _(c):
    m = c.xy(.5)
    c.s([(0, m), (1, m), (1, c.X, 1), (0, c.X, 1), (0, c.B, 1), (1, c.B, 1), (1, c.xy(.2))])

@g("f", 5, 4)
def _(c):
    c.s([(1, c.A), (.34, c.A, 1), (.34, c.B)]); bar(c, c.X)

@g("g", 6, 5)
def _(c):
    c.s([(1, c.X), (0, c.X, 1), (0, c.B, 1), (1, c.B)])
    c.s([(1, c.X), (1, c.D, 1), (.1, c.D)])

@g("h", 6, 5)
def _(c):
    stem(c, 0, c.B, c.A); c.s([(0, c.X), (1, c.X, 1), (1, c.B)])

@g("i", 2, 1, agsb=(1, 1), mono=5)
def _(c):
    if c.mono:
        c.s([(0, c.X), (.5, c.X), (.5, c.B)]); bar(c, c.B); c.dot(.5, c.f.xh + 1); return
    stem(c, .5, c.B, c.X); c.dot(.5, c.f.xh + 1)

@g("j", 4, 2, agsb=(0, 1), mono=5)
def _(c):
    if c.mono:
        c.s([(.25, c.X), (.75, c.X), (.75, c.D, 1), (0, c.D)]); c.dot(.75, c.f.xh + 1); return
    c.s([(1, c.X), (1, c.D, 1), (0, c.D)]); c.dot(1, c.f.xh + 1)

@g("k", 6, 5)
def _(c):
    stem(c, 0, c.B, c.A); c.s([(1, c.X), (0, c.xy(.42)), (1, c.B)])

@g("l", 2, 1, agsb=(1, 1), mono=5)
def _(c):
    if c.mono:      # flag at the top, tail curving right
        c.s([(0, c.A), (.5, c.A), (.5, c.B, 1), (1, c.B)]); return
    stem(c, .5, c.B, c.A)

@g("m", 10, 8)
def _(c):
    stem(c, 0, c.B, c.X)
    if c.heavymono:
        c.s([(0, c.X), (1, c.X, 1), (1, c.B)]); c.s([(.5, c.B), (.5, c.X)], hx=.5); return
    c.s([(0, c.X), (.5, c.X, 1), (.5, c.B)]); c.s([(.5, c.X), (1, c.X, 1), (1, c.B)])

@g("n", 6, 5)
def _(c):
    stem(c, 0, c.B, c.X); c.s([(0, c.X), (1, c.X, 1), (1, c.B)])

@g("o", 6, 5)
def _(c):
    ring(c, c.B, c.X)

@g("p", 6, 5)
def _(c):
    stem(c, 0, c.D, c.X); c.s([(0, c.X), (1, c.X, 1), (1, c.B, 1), (0, c.B)])

@g("q", 6, 5)
def _(c):
    stem(c, 1, c.D, c.X); c.s([(1, c.X), (0, c.X, 1), (0, c.B, 1), (1, c.B)])

@g("r", 5, 3)
def _(c):
    stem(c, 0, c.B, c.X); c.s([(0, c.X), (1, c.X, 1), (1, c.xy(.7))])

@g("s", 5, 4)
def _(c):
    m = c.xy(.5)
    if c.ch:
        c.s([(1, c.xy(.82)), (1, c.X, 1), (0, c.X, 1), (0, m, 1), (1, m, 1), (1, c.B, 1), (0, c.B, 1), (0, c.xy(.18))])
    else:
        c.s([(1, c.X), (0, c.X, 1), (0, m, 1), (1, m, 1), (1, c.B, 1), (0, c.B)])

@g("t", 5, 4)
def _(c):
    c.s([(.34, c.A - 1), (.34, c.B, 1), (1, c.B)]); bar(c, c.X)

@g("u", 6, 5)
def _(c):
    stem(c, 1, c.B, c.X); c.s([(0, c.X), (0, c.B, 1), (1, c.B)])

@g("v", 6, 5)
def _(c):
    if c.upright:
        k = c.xy(.75 if c.mono else .4)
        c.s([(0, c.X), (0, k), (.5, c.B), (1, k), (1, c.X)]); return
    c.s([(0, c.X), (.5, c.B), (1, c.X)])

@g("w", 10, 7)
def _(c):
    if c.upright:
        c.s([(0, c.X), (0, c.B, 1), (.5, c.B)]); c.s([(.5, c.B), (.5, c.xy(.75))], hx=.5 if c.heavymono else c.hx); c.s([(.5, c.B), (1, c.B, 1), (1, c.X)]); return
    c.s([(0, c.X), (.25, c.B), (.5, c.xy(.8)), (.75, c.B), (1, c.X)])

@g("x", 6, 5)
def _(c):
    if c.upright:
        c.s([(0, c.X), (0, c.xy(.8)), (1, c.xy(.2)), (1, c.B)]); c.s([(1, c.X), (1, c.xy(.8)), (0, c.xy(.2)), (0, c.B)]); return
    c.s([(0, c.X), (1, c.B)]); c.s([(1, c.X), (0, c.B)])

@g("y", 6, 5)
def _(c):
    if c.ch:      # a u with a descending tail
        c.s([(0, c.X), (0, c.B, 1), (1, c.B)]); c.s([(1, c.X), (1, c.D, 1), (.1, c.D)]); return
    c.s([(0, c.X), (.5, c.B)])
    c.s([(1, c.X), (1 - .5 * (c.X - c.D) / (c.X - c.B), c.D)])

@g("z", 6, 5)
def _(c):
    c.s([(0, c.X), (1, c.X), (0, c.B), (1, c.B)])


# ------------------------------------------------------------------ digits
@g("0", 6, 5)
def _(c):
    ring(c, c.B, c.C)
    if c.mono:      # slashed, to tell it from O
        c.s([(.25, c.cy(.25)), (.75, c.cy(.75))], hx=.5, hy=.5)

@g("1", 6, 5)
def _(c):
    if c.mono:
        c.s([(0, c.cy(.72)), (.5, c.C), (.5, c.B)]); bar(c, c.B); return
    c.s([(.1, c.cy(.72)), (.6, c.C), (.6, c.B)])

@g("2", 6, 5)
def _(c):
    c.s([(0, c.cy(.75)), (0, c.C, 1), (1, c.C, 1), (1, c.cy(.55), 1), (0, c.B), (1, c.B)])

@g("3", 6, 5)
def _(c):
    m = c.cy(.55)
    c.s([(0, c.C), (1, c.C, 1), (1, m, 1), (.35, m)])
    c.s([(.35, m), (1, m, 1), (1, c.B, 1), (0, c.B)])

@g("4", 6, 5)
def _(c):
    m = c.cy(.3); c.s([(.75, c.B), (.75, c.C), (0, m), (1, m)])

@g("5", 6, 5)
def _(c):
    m = c.cy(.6)
    c.s([(1, c.C), (0, c.C), (0, m), (1, m, 1), (1, c.B, 1), (0, c.B)])

@g("6", 6, 5)
def _(c):
    m = c.cy(.55)
    c.s([(.9, c.C), (0, c.C, 1), (0, c.B, 1), (1, c.B, 1), (1, m, 1), (0, m)])

@g("7", 6, 5)
def _(c):
    c.s([(0, c.C), (1, c.C), (.35, c.B)])

@g("8", 6, 5)
def _(c):
    m = c.cy(.55); ring(c, m, c.C, .06, .94); ring(c, c.B, m)

@g("9", 6, 5)
def _(c):
    m = c.cy(.45)
    c.s([(.1, c.B), (1, c.B, 1), (1, c.C, 1), (0, c.C, 1), (0, m, 1), (1, m)])


# ------------------------------------------------------------- punctuation
@g(" ", 3, 2)
def _(c):
    pass

@g(".", 2, 1, agsb=(1, 1))
def _(c):
    c.dot(.5, 0)

@g(",", 2, 1, agsb=(1, 1))
def _(c):
    c.dot(.5, 0); c.s([(.5, c.B), (.5, c.B - 1.2)], hx=c.hx * .6)

@g(":", 2, 1, agsb=(1, 1))
def _(c):
    c.dot(.5, 0); c.dot(.5, c.f.xh - max(2 * c.hx, 1) - 1)

@g(";", 2, 1, agsb=(1, 1))
def _(c):
    c.dot(.5, 0); c.s([(.5, c.B), (.5, c.B - 1.2)], hx=c.hx * .6); c.dot(.5, c.f.xh - max(2 * c.hx, 1) - 1)

@g("!", 2, 1, agsb=(1, 1))
def _(c):
    stem(c, .5, c.cy(.42), c.C); c.dot(.5, 0)

@g("?", 6, 5)
def _(c):
    c.s([(0, c.cy(.75)), (0, c.C, 1), (1, c.C, 1), (1, c.cy(.58), 1), (.5, c.cy(.42)), (.5, c.cy(.28))])
    c.dot(.5, 0)

@g("'", 2, 1, agsb=(1, 1))
def _(c):
    stem(c, .5, c.cy(.68), c.C)

@g('"', 5, 3)
def _(c):
    stem(c, 0, c.cy(.68), c.C); stem(c, 1, c.cy(.68), c.C)

@g("-", 4, 3)
def _(c):
    c.s([(0, c.xy(.5)), (1, c.xy(.5))], hx=c.m, hy=c.m)

@g("_", 7, 6, chsb=(0, 0), agsb=(0, 0))
def _(c):
    c.s([(0, -1.5), (1, -1.5)], hx=c.hy, hy=c.hy)

@g("+", 6, 5)
def _(c):
    m = c.cy(.45); k = dict(hx=c.m, hy=c.m)
    c.s([(0, m), (1, m)], **k); c.s([(.5, m - 2.5 + c.m), (.5, m + 2.5 - c.m)], **k)

@g("=", 6, 5)
def _(c):
    m = c.cy(.45); k = dict(hx=c.m, hy=c.m)
    c.s([(0, m + 1.25), (1, m + 1.25)], **k); c.s([(0, m - 1.25), (1, m - 1.25)], **k)

@g("<", 5, 5)
def _(c):
    m = c.cy(.45); c.s([(1, m + 2.5), (0, m), (1, m - 2.5)], hx=c.m, hy=c.m)

@g(">", 5, 5)
def _(c):
    m = c.cy(.45); c.s([(0, m + 2.5), (1, m), (0, m - 2.5)], hx=c.m, hy=c.m)

@g("/", 5, 4)
def _(c):
    c.s([(0, c.B - 1), (1, c.C)])

@g("\\", 5, 4)
def _(c):
    c.s([(1, c.B - 1), (0, c.C)])

@g("|", 2, 1, agsb=(1, 1))
def _(c):
    stem(c, .5, c.D, c.C + 1)

@g("(", 4, 3)
def _(c):
    c.s([(1, c.C + 1), (0, c.C + 1, 1.6), (0, c.D, 1.6), (1, c.D)])

@g(")", 4, 3)
def _(c):
    c.s([(0, c.C + 1), (1, c.C + 1, 1.6), (1, c.D, 1.6), (0, c.D)])

@g("[", 4, 3)
def _(c):
    c.s([(1, c.C + 1), (0, c.C + 1), (0, c.D), (1, c.D)])

@g("]", 4, 3)
def _(c):
    c.s([(0, c.C + 1), (1, c.C + 1), (1, c.D), (0, c.D)])

@g("{", 5, 4)
def _(c):
    m = c.snap((c.C + 1 + c.D) / 2)
    c.s([(1, c.C + 1), (.5, c.C + 1, 1), (.5, m, .8), (0, m)])
    c.s([(1, c.D), (.5, c.D, 1), (.5, m, .8), (0, m)])

@g("}", 5, 4)
def _(c):
    m = c.snap((c.C + 1 + c.D) / 2)
    c.s([(0, c.C + 1), (.5, c.C + 1, 1), (.5, m, .8), (1, m)])
    c.s([(0, c.D), (.5, c.D, 1), (.5, m, .8), (1, m)])

@g("*", 6, 5)
def _(c):
    m = c.C - 2
    stem(c, .5, m - 2, m + 2); c.s([(0, m + 1.3), (1, m - 1.3)]); c.s([(0, m - 1.3), (1, m + 1.3)])

@g("^", 6, 5)
def _(c):
    c.s([(0, c.C - 3), (.5, c.C), (1, c.C - 3)])

@g("`", 3, 2)
def _(c):
    c.s([(0, c.C + .5), (1, c.C - 1.5)])

@g("~", 7, 6)
def _(c):
    m = c.cy(.45)
    c.s([(0, m - .5), (.25, m + 1, 1), (.5, m + .25), (.75, m - .5, 1), (1, m + 1)], hx=c.m, hy=c.m)

@g("#", 8, 7)
def _(c):
    m = c.cy(.5)
    k = dict(hx=.5, hy=.5)      # upright hairlines keep the counters open at small sizes
    c.s([(.28, c.B), (.28, c.C)], **k); c.s([(.72, c.B), (.72, c.C)], **k)
    c.s([(0, m + 1.5), (1, m + 1.5)], **k); c.s([(0, m - 1.5), (1, m - 1.5)], **k)

@g("$", 6, 5)
def _(c):
    m = c.cy(.5)
    c.s([(1, c.cy(.8)), (1, c.C, 1), (0, c.C, 1), (0, m, 1), (1, m, 1), (1, c.B, 1), (0, c.B, 1), (0, c.cy(.2))])
    c.s([(.5, c.B - 1), (.5, c.C + 1)], hx=c.hx * .6)

@g("%", 11, 9)
def _(c):
    k = 3 / (c.ink - 2 * c.hx) if c.hx < 1 else 4 / (c.ink - 2 * c.hx)
    ring(c, c.C - 3, c.C, 0, k, .8); ring(c, c.B, c.B + 3, 1 - k, 1, .8)
    c.s([(.22, c.B), (.78, c.C)])

@g("&", 8, 7)
def _(c):
    c.s([(1, c.B), (.15, c.cy(.72)), (.15, c.C, 1), (.7, c.C, 1), (.7, c.cy(.72), 1),
         (0, c.cy(.3), 1), (0, c.B, 1), (.6, c.B, 1), (1, c.cy(.45))])

@g("@", 11, 10)
def _(c):
    t, b = c.C, c.D + 1
    c.s([(.75, b), (0, b, 1.5), (0, t, 1.5), (1, t, 1.5), (1, c.cy(.2), 1), (.72, c.cy(.2))])
    ring(c, c.cy(.2), c.cy(.7), .3, .72, .9)
    stem(c, .72, c.cy(.2), c.cy(.7))
