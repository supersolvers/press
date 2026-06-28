"""Greek (monotonic) and Cyrillic (U+0400-045F plus Ґ ґ)."""
from glyphs import GLYPHS, stem, bar, ring
from latin import _mp, _dims, _dotsize, auto_composites, composite, m_dieresis


def reg(ch, fn, chw, agw, chsb=None, agsb=None):
    GLYPHS[ch] = (fn, {"ch": chw, "ag": agw}, {"ch": chsb, "ag": agsb})


def same(new_chars, old_chars):
    for n, o in zip(new_chars, old_chars):
        GLYPHS[n] = GLYPHS[o]


def mid(c, T, t=.5):
    return c.snap(c.B + t * (T - c.B))


def both(up, lo, wu, wl, fn):
    """Register one skeleton at cap height (up) and at x-height (lo)."""
    if up:
        reg(up, lambda c: fn(c, c.C), *wu)
    if lo:
        reg(lo, lambda c: fn(c, c.X), *wl)


# ================================================================== GREEK
same("ΑΒΕΖΗΙΚΜΝΟΡΤΥΧ", "ABEZHIKMNOPTYX")
same("οκμ", "oĸµ")
reg("ν", lambda c: c.s([(0, c.X), (.5, c.B), (1, c.X)]), 6, 5)   # always pointed, unlike υ

reg("Γ", lambda c: c.s([(1, c.C), (0, c.C), (0, c.B)]), 6, 5)
reg("Δ", lambda c: c.s([(0, c.B), (.5, c.C), (1, c.B)], closed=True), 8, 7)
reg("Θ", lambda c: (ring(c, c.B, c.C), bar(c, c.cy(.5), .28, .72)), 7, 7)
reg("Λ", lambda c: c.s([(0, c.B), (.5, c.C), (1, c.B)]), 7, 7)
reg("Ξ", lambda c: (bar(c, c.C), bar(c, c.B), bar(c, c.cy(.5), .15, .85)), 6, 6)
reg("Π", lambda c: c.s([(0, c.B), (0, c.C), (1, c.C), (1, c.B)]), 7, 7)
reg("Σ", lambda c: c.s([(1, c.C), (0, c.C), (.55, c.cy(.5)), (0, c.B), (1, c.B)]), 6, 6)
reg("Φ", lambda c: (ring(c, c.cy(.15), c.cy(.85)), stem(c, .5, c.B, c.C)), 8, 7)
reg("Ψ", lambda c: (c.s([(0, c.C), (0, c.cy(.35), 1), (1, c.cy(.35), 1), (1, c.C)]), stem(c, .5, c.B, c.C)), 8, 7)
reg("Ω", lambda c: c.s([(0, c.B), (.3, c.B), (0, c.cy(.4), 1), (0, c.C, 1), (1, c.C, 1), (1, c.cy(.4), 1), (.7, c.B), (1, c.B)]), 8, 7)


def _alpha(c):
    c.s([(.8, c.X), (0, c.X, 1), (0, c.B, 1), (.8, c.B)]); c.s([(.8, c.X), (.8, c.B, .8), (1, c.B)])

def _beta(c):
    m = c.ay(.55)
    c.s([(0, c.D), (0, c.A, 1), (.85, c.A, 1), (.85, m, .8), (.35, m)])
    c.s([(.35, m), (1, m, 1), (1, c.B, 1), (0, c.B)])

def _gamma(c):
    k = c.xy(.15); c.s([(0, c.X), (.5, k), (1, c.X)]); stem(c, .5, c.D, k)

def _delta(c):
    ring(c, c.B, c.X); c.s([(.9, c.A), (.1, c.A, 1), (.1, (c.A + c.X) / 2 + .3, .8), (.7, c.X)])

def _epsilon(c):
    m = c.xy(.5)
    c.s([(1, c.X), (0, c.X, 1), (0, m, 1), (.6, m)]); c.s([(.6, m), (0, m, 1), (0, c.B, 1), (1, c.B)])

def _zeta(c):
    c.s([(0, c.A), (1, c.A), (0, c.xy(.55), 1), (0, c.B, 1), (1, c.B, 1), (1, c.D + .8, 1), (.45, c.D + .8)])

def _eta(c):
    stem(c, 0, c.B, c.X); c.s([(0, c.X), (1, c.X, 1), (1, c.D)])

def _theta(c):
    ring(c, c.B, c.A); bar(c, c.ay(.5))

def _iota(c):
    c.s([(.2, c.X), (.2, c.B, 1), (1, c.B)])

def _lambda(c):
    c.s([(.1, c.A), (1, c.B)]); c.s([(.52, c.ay(.5)), (0, c.B)])

def _xi(c):
    m = c.ay(.6)
    c.s([(1, c.A), (.15, c.A, 1), (.15, m, 1), (.75, m)])
    c.s([(.75, m), (0, m, 1), (0, c.B, 1), (1, c.B, 1), (1, c.D + .8, 1), (.45, c.D + .8)])

def _pi(c):
    bar(c, c.X); stem(c, .2, c.B, c.X); stem(c, .8, c.B, c.X)

def _rho(c):
    ring(c, c.B, c.X); stem(c, 0, c.D, c.xy(.5))

def _sigma(c):
    ring(c, c.B, c.X, 0, .85); bar(c, c.X, .45, 1)

def _fsigma(c):
    c.s([(1, c.X), (0, c.X, 1), (0, c.B, 1), (1, c.B, 1), (1, c.D + .8, 1), (.4, c.D + .8)])

def _tau(c):
    bar(c, c.X); c.s([(.45, c.X), (.45, c.B, 1), (1, c.B)])

def _upsilon(c):
    c.s([(0, c.X), (0, c.B, 1), (1, c.B, 1), (1, c.X)])

def _phi(c):
    ring(c, c.B, c.X); stem(c, .5, c.D, c.A)

def _chi(c):
    c.s([(0, c.X), (1, c.D)]); c.s([(1, c.X), (0, c.D)])

def _psi(c):
    c.s([(0, c.X), (0, c.B, 1), (1, c.B, 1), (1, c.X)]); stem(c, .5, c.D, c.A)

def _omega(c):
    k = c.xy(.6)
    c.s([(0, c.X), (0, c.B, 1), (.5, c.B, 1), (.5, k)]); c.s([(.5, k), (.5, c.B, 1), (1, c.B, 1), (1, c.X)])


for _ch, _fn, _w in (("α", _alpha, (7, 6)), ("β", _beta, (6, 5)), ("γ", _gamma, (6, 5)), ("δ", _delta, (6, 5)),
                     ("ε", _epsilon, (5, 4)), ("ζ", _zeta, (5, 5)), ("η", _eta, (6, 5)), ("θ", _theta, (6, 5)),
                     ("λ", _lambda, (6, 5)), ("ξ", _xi, (5, 5)), ("π", _pi, (7, 6)), ("ρ", _rho, (6, 5)),
                     ("σ", _sigma, (7, 6)), ("ς", _fsigma, (5, 5)), ("τ", _tau, (5, 5)), ("υ", _upsilon, (6, 5)),
                     ("φ", _phi, (8, 7)), ("χ", _chi, (6, 5)), ("ψ", _psi, (8, 7)), ("ω", _omega, (9, 7))):
    reg(_ch, _fn, *_w)
reg("ι", _iota, 3, 2, agsb=(1, 1))
GLYPHS["ι"][1]["mono"] = 5


def m_tonos(c, cx, y):
    p, k = _mp(c); c.s([(cx - .15, y + p), (cx + .45, y + 2 - p)], **k)

def m_dialytika_tonos(c, cx, y):
    d = max(1.0, _dotsize(c) * .8); o = d / 2 + (1.4 if c.hx >= 1 else 1.0)   # dots clear of the accent
    for s in (-1, 1):
        c.rect(cx + s * o - d / 2, y, cx + s * o + d / 2, y + d)
    p, k = _mp(c); c.s([(cx, y + 1), (cx + .35, y + 2.8)], hx=max(.5, p * .8), hy=max(.5, p * .8), absx=True)

m_dialytika_tonos.half = 3.0


for _ch, _b in zip("άέήίόύώ", "αεηιουω"):
    composite(_ch, _b, m_tonos)
for _ch, _b in zip("ϊϋΪΫ", "ιυΙΥ"):
    composite(_ch, _b, m_dieresis)
for _ch, _b in zip("ΐΰ", "ιυ"):
    composite(_ch, _b, m_dialytika_tonos)


def _cap_tonos(base):
    fn, inks, sbs = GLYPHS[base]

    def draw(c):
        p, k = _mp(c); x0 = c.lsb
        c.lsb += 2 if base != "Α" else 1
        fn(c)
        c.s([(x0 + .9, c.f.cap - p), (x0 + .3, c.f.cap - 2.2)], **k)
    return draw, inks, sbs

for _ch, _b in zip("ΆΈΉΊΌΎΏ", "ΑΕΗΙΟΥΩ"):
    GLYPHS[_ch] = _cap_tonos(_b)

reg("΄", lambda c: m_tonos(c, c.lsb + c.ink / 2, c.f.xh + c.f.mark_gap), 3, 2)
reg("΅", lambda c: m_dialytika_tonos(c, c.lsb + c.ink / 2, c.f.xh + c.f.mark_gap), 5, 4)
GLYPHS[";"] = GLYPHS[";"]; GLYPHS["·"] = GLYPHS["·"]


# =============================================================== CYRILLIC
same("АВЕКМНОРСТХЅІЈ", "ABEKMHOPCTXSIJ")
same("аеорсхѕіјкф", "aeopcxsijĸφ")
same("ГПФ", "ΓΠΦ")

W7, W6 = (7, 7), (6, 5)


def _be_cap(c, T):
    m = mid(c, T, .55); c.s([(1, T), (0, T), (0, c.B)]); c.s([(0, m), (1, m, 1), (1, c.B, 1), (0, c.B)])

def _be(c):
    ring(c, c.B, c.X); c.s([(0, c.xy(.5)), (0, c.A, 1.3), (1, c.A)])

def _ve(c, T):
    m = mid(c, T); stem(c, 0, c.B, T)
    c.s([(0, T), (.88, T, 1), (.88, m, 1), (0, m)]); c.s([(0, m), (1, m, 1), (1, c.B, 1), (0, c.B)])

def _ge(c, T):
    c.s([(1, T), (0, T), (0, c.B)])

def _ghe_up(c, T):
    c.s([(1, T + 1.5), (1, T), (0, T), (0, c.B)])

def _de(c, T):
    c.s([(.14, c.B), (.3, T), (.86, T), (.86, c.B)]); bar(c, c.B)
    c.s([(0, c.B), (0, c.B - 1.5)]); c.s([(1, c.B), (1, c.B - 1.5)])

def _zhe(c, T):
    m = mid(c, T); stem(c, .5, c.B, T)
    c.s([(0, T), (.5, m), (0, c.B)]); c.s([(1, T), (.5, m), (1, c.B)])

def _ze(c, T):
    # Chamfered bowls that leave a notch at the waist: without it з is indistinguishable from э at small sizes.
    m = mid(c, T, .55); k = .35
    up = [(1, T - 1, k), (1, m + 1, k)] if m + 1 < T - 1 else [(1, (T + m) / 2, k)]
    lo = [(1, m - 1, k), (1, c.B + 1, k)] if c.B + 1 < m - 1 else [(1, (m + c.B) / 2, k)]
    head = [(0, c.cy(.8)), (0, T, 1)] if T == c.C and c.hx < 1 else [(0, T)]   # a hook closes the bowl under a heavy pen
    c.s(head + [(.68, T, k)] + up + [(.68, m, k), (.45, m)])
    c.s([(.45, m), (.68, m, k)] + lo + [(.68, c.B, k), (0, c.B)])

def _i(c, T):
    if c.upright:
        stem(c, 0, c.B, T); stem(c, 1, c.B, T); c.s([(0, c.B), (1, T)], hx=min(.6, c.hx + .1), hy=min(.6, c.hx + .1)); return
    c.s([(0, T), (0, c.B), (1, T), (1, c.B)])

def _el(c, T):
    c.s([(0, c.B), (.22, c.B, 1), (.22, T), (1, T), (1, c.B)])

def _em(c, T):
    if c.upright:
        stem(c, 0, c.B, T); stem(c, 1, c.B, T); c.s([(0, T), (.5, mid(c, T, .45 if c.mono else .3)), (1, T)], hx=min(.55, c.hx + .05), hy=min(.55, c.hx + .05)); return
    c.s([(0, c.B), (0, T), (.5, mid(c, T, .25)), (1, T), (1, c.B)])

def _en(c, T):
    stem(c, 0, c.B, T); stem(c, 1, c.B, T); bar(c, mid(c, T))

def _pe(c, T):
    c.s([(0, c.B), (0, T), (1, T), (1, c.B)])

def _te(c, T):
    bar(c, T); stem(c, .5, c.B, T)

def _tse(c, T):
    c.s([(0, T), (0, c.B), (1, c.B), (1, c.B - 1.5)]); stem(c, .84, c.B, T)

def _che(c, T):
    m = mid(c, T, .45); c.s([(0, T), (0, m, 1), (1, m)]); stem(c, 1, c.B, T)

def _sha(c, T):
    c.s([(0, T), (0, c.B), (1, c.B), (1, T)]); c.s([(.5, c.B), (.5, T)], hx=.5 if c.heavymono else c.hx)

def _shcha(c, T):
    c.s([(0, T), (0, c.B), (1, c.B), (1, c.B - 1.5)]); stem(c, .44, c.B, T); stem(c, .88, c.B, T)

def _soft(c, T, x0=0, x1=1):
    m = mid(c, T, .55); stem(c, x0, c.B, T); c.s([(x0, m), (x1, m, 1), (x1, c.B, 1), (x0, c.B)])

def _hard(c, T):
    c.s([(0, T), (.28, T)]); _soft(c, T, .28, 1)

def _yeru(c, T):
    _soft(c, T, 0, .6); stem(c, 1, c.B, T)

def _e(c, T):
    m = mid(c, T)
    if T == c.C:
        c.s([(0, c.cy(.72)), (0, T, 1), (1, T, 1), (1, c.B, 1), (0, c.B, 1), (0, c.cy(.28))])
    else:
        c.s([(0, T), (1, T, 1), (1, c.B, 1), (0, c.B)])
    bar(c, m, .2, 1)

def _ye(c, T):
    m = mid(c, T)
    if T == c.C:
        c.s([(1, c.cy(.72)), (1, T, 1), (0, T, 1), (0, c.B, 1), (1, c.B, 1), (1, c.cy(.28))])
    else:
        c.s([(1, T), (0, T, 1), (0, c.B, 1), (1, c.B)])
    bar(c, m, 0, .62)

def _yu(c, T):
    stem(c, 0, c.B, T); bar(c, mid(c, T), 0, .36); ring(c, c.B, T, .36, 1)

def _ya(c, T):
    m = mid(c, T, .45); stem(c, 1, c.B, T)
    c.s([(1, T), (0, T, 1), (0, m, 1), (1, m)]); c.s([(.55, m), (0, c.B)])

def _dzhe(c, T):
    c.s([(0, T), (0, c.B), (1, c.B), (1, T)]); c.s([(.5, c.B), (.5, c.B - 1.5)])

def _lje(c, T):
    c.s([(0, c.B), (.12, c.B, 1), (.12, T), (.55, T), (.55, c.B)]); _soft(c, T, .55, 1)

def _nje(c, T):
    stem(c, 0, c.B, T); bar(c, mid(c, T, .55), 0, .55); _soft(c, T, .55, 1)

def _tshe(c, T):
    m = mid(c, T, .55); bar(c, T, 0, .7); stem(c, .3, c.B, T); c.s([(.3, m), (1, m, 1), (1, c.B)])

def _dje(c, T):
    m = mid(c, T, .55); bar(c, T, 0, .7); stem(c, .3, c.B, T)
    c.s([(.3, m), (1, m, 1), (1, c.B, 1), (.62, c.B)])

def _dje_small(c):
    stem(c, .12, c.B, c.A); c.s([(.12, c.X), (1, c.X, 1), (1, c.D, 1), (.5, c.D)])
    y = (c.f.xh + c.f.asc) / 2 + .2
    c.s([(c.lsb - .1, y), (c.lsb + 2 * c.hx + 1.6, y)], absx=True, hx=c.hy, hy=c.hy)


for _up, _lo, _wu, _wl, _fn in (
        ("Б", None, (7, 6), None, _be_cap), ("В", "в", (7, 6), W6, _ve), (None, "г", None, (5, 4), _ge),
        ("Ґ", "ґ", (6, 5), (5, 4), _ghe_up), ("Д", "д", (8, 8), (7, 6), _de), ("Ж", "ж", (10, 9), (9, 7), _zhe),
        ("З", "з", (6, 6), (5, 4), _ze), ("И", "и", W7, W6, _i), ("Л", "л", W7, W6, _el), (None, "м", None, (8, 7), _em),
        (None, "н", None, W6, _en), (None, "п", None, W6, _pe), (None, "т", None, (6, 5), _te),
        ("Ц", "ц", (8, 8), (7, 6), _tse), ("Ч", "ч", (7, 6), W6, _che), ("Ш", "ш", (10, 9), (9, 7), _sha),
        ("Щ", "щ", (11, 10), (10, 8), _shcha), ("Ь", "ь", (7, 6), W6, _soft), ("Ъ", "ъ", (8, 7), (7, 6), _hard),
        ("Ы", "ы", (10, 9), (9, 7), _yeru), ("Э", "э", (7, 6), (5, 5), _e), ("Є", "є", (7, 6), (5, 5), _ye),
        ("Ю", "ю", (10, 9), (9, 7), _yu), ("Я", "я", W7, W6, _ya), ("Џ", "џ", W7, W6, _dzhe),
        ("Љ", "љ", (11, 10), (10, 8), _lje), ("Њ", "њ", (11, 10), (10, 8), _nje),
        ("Ћ", None, (8, 7), None, _tshe), ("Ђ", None, (8, 7), None, _dje)):
    both(_up, _lo, _wu, _wl, _fn)

reg("б", _be, 6, 5)
reg("ђ", _dje_small, 7, 6)
GLYPHS["ћ"] = GLYPHS["ħ"]
def _u_cap(c):
    c.s([(0, c.C), (.5, c.cy(.3))]); c.s([(1, c.C), (.3, c.B)])

def _u_small(c):      # always diagonal, so it cannot be mistaken for ц or ч
    c.s([(0, c.X), (.5, c.B)]); c.s([(1, c.X), (1 - .5 * (c.X - c.D) / (c.X - c.B), c.D)])

reg("у", _u_small, 6, 5)
reg("У", _u_cap, 7, 7)

auto_composites(range(0x400, 0x460))
