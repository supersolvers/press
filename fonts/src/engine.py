"""Stroke-skeleton font engine.

Glyphs are described once as skeleton strokes on a pixel-like grid; each family
supplies its own proportions and pen (hx = half stem width, hy = half bar
thickness), so the same skeleton yields a heavy, high-contrast face or a light
monoline one. Skeletons are expanded to outlines with skia-pathops.
"""
import math
from dataclasses import dataclass, field

import pathops
from pathops import LineCap, LineJoin, PathOp

KAPPA = 0.5523


@dataclass
class Family:
    name: str
    style: str
    em: float        # em height in px
    cap: float       # cap height in px
    xh: float        # x-height in px
    asc: float       # ascender height (b, d, l) in px
    desc: float      # descender depth in px (positive)
    hx: float        # half vertical-stem width in px
    hy: float        # half horizontal-bar thickness in px
    r: float         # bowl corner radius in px
    sb: tuple        # default (left, right) side bearings in px
    wkey: str        # which column of the width table to use
    px: int = 100    # font units per px
    line_asc: float = 0   # vertical metrics in px
    line_desc: float = 0
    mark_gap: float = 1.0
    wadd: float = 0       # extra ink width per glyph (bold)
    weight: int = 400
    chicago: bool = False # use the Chicago-style letterforms (arched A, upright V W X Y)
    mono: int = 0         # fixed advance in px (0 = proportional)
    wcol: str = ""        # optional extra width-table column that overrides wkey where present
    cjk_adv: int = 0      # advance for full-width CJK in px (0 = as imported)
    compose: bool = False # accented letters made from the plain one in pixels (see compose.py)

    @property
    def upm(self):
        return int(round(self.em * self.px))


def ink_for(fam, inks):
    """Ink width of a glyph in this family, from its width-table entry."""
    base = inks.get(fam.wcol, inks[fam.wkey]) + fam.wadd
    if fam.mono:                       # one width for everything but the truly narrow marks
        if "mono" in inks or base >= 3:
            # a heavy pen cannot fit three stems in the normal width: those letters take the whole cell
            return fam.mono - 2 + (1 if fam.hx >= 1 and inks.get("wide3") else 0)
        return base
    return base


class Ctx:
    """Drawing context for one glyph."""
    registry = {}

    def __init__(self, fam, ink, lsb=None, rsb=None):
        self.f = fam
        self.ink = ink
        self.lsb = fam.sb[0] if lsb is None else lsb
        self.rsb = fam.sb[1] if rsb is None else rsb
        self.parts = []
        self.anchors = {}
        f = fam
        self.B = f.hy                 # centreline of a bar sitting on the baseline
        self.X = f.xh - f.hy          # ... touching the x-height
        self.C = f.cap - f.hy         # ... touching the cap height
        self.A = f.asc - f.hy         # ... touching the ascender height
        self.D = -f.desc + f.hy       # ... touching the descender line
        self.hx, self.hy, self.r = f.hx, f.hy, f.r
        self.m = (f.hx + f.hy) / 2    # half-width of the even pen used for symbols
        self.ch = fam.chicago          # Chicago-style family: use its own letterforms
        self.mono = fam.mono > 0      # monospaced family: narrow letters get serifs to fill the cell
        self.gen = fam.wcol == "ge"    # Geneva-style family: wide rounds, M and W with full-height points
        self.heavymono = self.mono and fam.hx >= 1   # three-stem letters get a hairline middle stem
        self.upright = self.ch or self.mono   # build V W X M N from upright stems (clean in a narrow cell)
        self.zero = False             # zero advance (combining mark)
        self.dy = 0.0                 # vertical offset applied to everything (marks)
        self.dx = 0.0

    def use_pen(self, hx, hy):
        """Switch pen for this glyph (e.g. Hebrew wants heavy bars and light stems)."""
        f = self.f
        self.hx, self.hy = hx, hy
        self.B = hy; self.X = f.xh - hy; self.C = f.cap - hy; self.A = f.asc - hy; self.D = -f.desc + hy

    # -- coordinates -------------------------------------------------------
    @property
    def adv(self):
        return 0 if self.zero else self.lsb + self.ink + self.rsb

    def x(self, t):
        return self.lsb + self.hx + t * (self.ink - 2 * self.hx) + self.dx

    def snap(self, y):
        """Snap a bar centreline to the pixel grid."""
        return math.floor(y - self.hy + 0.5) + self.hy

    def cy(self, t):
        return self.snap(self.B + t * (self.C - self.B))

    def xy(self, t):
        return self.snap(self.B + t * (self.X - self.B))

    def ay(self, t):
        return self.snap(self.B + t * (self.A - self.B))

    # -- primitives --------------------------------------------------------
    def s(self, pts, closed=False, hx=None, hy=None, absx=False, clip=True):
        """Stroke a polyline. pts: (xfrac, y) or (xfrac, y, radius_factor)."""
        hx = self.hx if hx is None else hx
        hy = self.hy if hy is None else hy
        P = []
        for p in pts:
            px_ = (p[0] + self.dx) if absx else self.x(p[0])
            P.append((px_, p[1] + self.dy, (p[2] if len(p) > 2 else 0) * self.r))
        self.parts.append(_stroke(P, closed, hx, hy, clip))

    def rect(self, x0, y0, x1, y1):
        p = pathops.Path()
        p.moveTo(x0, y0); p.lineTo(x1, y0); p.lineTo(x1, y1); p.lineTo(x0, y1); p.close()
        self.parts.append(p)

    def dot(self, t, ybot, w=None, h=None):
        """Square dot, centred on x(t), sitting on ybot (edge coordinate)."""
        w = 2 * self.hx if w is None else w
        h = max(2 * self.hx, 2 * self.hy) if h is None else h
        cx = self.x(t)
        self.rect(cx - w / 2, ybot + self.dy, cx + w / 2, ybot + h + self.dy)

    def disc(self, cx, cy, rad):
        """Filled circle at absolute px coordinates."""
        k = KAPPA * rad
        p = pathops.Path()
        p.moveTo(cx + rad, cy)
        p.cubicTo(cx + rad, cy + k, cx + k, cy + rad, cx, cy + rad)
        p.cubicTo(cx - k, cy + rad, cx - rad, cy + k, cx - rad, cy)
        p.cubicTo(cx - rad, cy - k, cx - k, cy - rad, cx, cy - rad)
        p.cubicTo(cx + k, cy - rad, cx + rad, cy - k, cx + rad, cy)
        p.close()
        self.parts.append(p)

    def sub(self, ch):
        """Draw another registered glyph in its own context and return that context."""
        fn, inks, sbs = self.registry[ch]
        sb = sbs[self.f.wkey] or (None, None)
        c = Ctx(self.f, ink_for(self.f, inks), sb[0], sb[1])
        fn(c)
        return c

    def place(self, ch, scale=1.0, tx=0.0, ty=0.0):
        """Stamp a scaled copy of glyph ch; returns its scaled advance."""
        c = self.sub(ch)
        for p in c.parts:
            self.parts.append(p.transform(scale, 0, 0, scale, tx + self.dx, ty + self.dy))
        return c.adv * scale

    def rot180(self, ch, ytop):
        """Glyph ch rotated half a turn, its baseline landing on ytop."""
        c = self.sub(ch)
        for p in c.parts:
            self.parts.append(p.transform(-1, 0, 0, -1, c.adv, ytop))

    def result(self):
        if not self.parts:
            return pathops.Path()
        out = pathops.Path()
        for p in self.parts:
            out.addPath(p)
        out.simplify(fix_winding=True, keep_starting_points=False)
        return out


def _rounded(P, closed):
    """Polyline with per-vertex rounding -> list of drawing commands."""
    n = len(P)
    cmds = []

    def trim(i):
        x, y, r = P[i]
        if r <= 0 or (not closed and i in (0, n - 1)):
            return None
        a = P[(i - 1) % n]; b = P[(i + 1) % n]
        la = math.hypot(a[0] - x, a[1] - y); lb = math.hypot(b[0] - x, b[1] - y)
        # neighbouring rounded corners share a segment half each
        ha = la / 2 if (a[2] > 0 and (closed or (i - 1) % n not in (0, n - 1))) else la
        hb = lb / 2 if (b[2] > 0 and (closed or (i + 1) % n not in (0, n - 1))) else lb
        da = min(r, ha); db = min(r, hb)
        pin = (x + (a[0] - x) * da / la, y + (a[1] - y) * da / la)
        pout = (x + (b[0] - x) * db / lb, y + (b[1] - y) * db / lb)
        c1 = (pin[0] + (x - pin[0]) * KAPPA, pin[1] + (y - pin[1]) * KAPPA)
        c2 = (pout[0] + (x - pout[0]) * KAPPA, pout[1] + (y - pout[1]) * KAPPA)
        return pin, c1, c2, pout

    T = [trim(i) for i in range(n)]
    if closed:
        start = T[0][3] if T[0] else P[0][:2]
        cmds.append(("M", start))
        for k in range(1, n + 1):
            i = k % n
            if T[i]:
                pin, c1, c2, pout = T[i]
                cmds.append(("L", pin)); cmds.append(("C", c1, c2, pout))
            else:
                cmds.append(("L", P[i][:2]))
        cmds.append(("Z",))
    else:
        cmds.append(("M", P[0][:2]))
        for i in range(1, n):
            if T[i]:
                pin, c1, c2, pout = T[i]
                cmds.append(("L", pin)); cmds.append(("C", c1, c2, pout))
            else:
                cmds.append(("L", P[i][:2]))
    return cmds


def _stroke(P, closed, hx, hy, clip):
    P = list(P)
    xs = [p[0] for p in P]; ys = [p[1] for p in P]
    box = (min(xs) - hx, min(ys) - hy, max(xs) + hx, max(ys) + hy)
    if not closed:
        ext = 4 * max(hx, hy)
        for end, nb in ((0, 1), (-1, -2)):
            x, y, r = P[end]; ax, ay, _ = P[nb]
            dx, dy = x - ax, y - ay
            L = math.hypot(dx, dy)
            if abs(dx) > 1e-6 and abs(dy) > 1e-6:      # diagonal terminal: overshoot, then clip flat
                e = ext
            else:                                      # axis-aligned: square off to the pen edge
                e = hx if abs(dx) > 1e-6 else hy
            P[end] = (x + dx / L * e, y + dy / L * e, r)
    k = hy / hx                                       # squash x so the pen is a circle
    path = pathops.Path()
    for c in _rounded(P, closed):
        if c[0] == "M":
            path.moveTo(c[1][0] * k, c[1][1])
        elif c[0] == "L":
            path.lineTo(c[1][0] * k, c[1][1])
        elif c[0] == "C":
            path.cubicTo(c[1][0] * k, c[1][1], c[2][0] * k, c[2][1], c[3][0] * k, c[3][1])
        else:
            path.close()
    path.stroke(2 * hy, LineCap.BUTT_CAP, LineJoin.MITER_JOIN, 8)
    path = path.transform(1 / k, 0, 0, 1, 0, 0)
    path.simplify(fix_winding=True, keep_starting_points=False)
    if clip:
        b = pathops.Path()
        b.moveTo(box[0], box[1]); b.lineTo(box[2], box[1]); b.lineTo(box[2], box[3]); b.lineTo(box[0], box[3]); b.close()
        path = pathops.op(path, b, PathOp.INTERSECTION)
    return path
