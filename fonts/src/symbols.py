"""UI symbol font in the spirit of Marlett: window-caption buttons, scroll and
menu arrows, checks and bullets. Each symbol sits on Unicode code points and,
where the slot is known, on Marlett's own character (plain and U+F0xx forms).

Slots were assigned from memory of how Windows and Wine use Marlett; only the
ones listed in SLOTS are mapped. Wine's replacement defines space, 0-9 and a-y.
"""
import os

import pathops
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from pathops import PathOp

import bitmap
from engine import Family, _stroke, KAPPA

FAM = Family(name="Fleuron", style="Regular", em=12, cap=9, xh=7, asc=9, desc=3,
             hx=.5, hy=.5, r=1, sb=(0, 0), wkey="ag", line_asc=10, line_desc=2)
ADV = 12       # every symbol is drawn in a 9x9 box (x 1..10, y 0..9) on a 12 px advance


def rect(x0, y0, x1, y1):
    p = pathops.Path()
    p.moveTo(x0, y0); p.lineTo(x1, y0); p.lineTo(x1, y1); p.lineTo(x0, y1); p.close()
    return p

def poly(pts):
    p = pathops.Path(); p.moveTo(*pts[0])
    for q in pts[1:]:
        p.lineTo(*q)
    p.close(); return p

def union(*ps):
    out = pathops.Path()
    for p in ps:
        out.addPath(p)
    out.simplify(fix_winding=True, keep_starting_points=False)
    return out

def frame(x0, y0, x1, y1, t=1, top=None):
    return pathops.op(rect(x0, y0, x1, y1), rect(x0 + t, y0 + t, x1 - t, y1 - (top or t)), PathOp.DIFFERENCE)

def line(pts, w=.5, closed=False, r=0):
    return _stroke([(x, y, r) for x, y in pts], closed, w, w, True)

def disc(cx, cy, rad):
    k = KAPPA * rad; p = pathops.Path(); p.moveTo(cx + rad, cy)
    p.cubicTo(cx + rad, cy + k, cx + k, cy + rad, cx, cy + rad); p.cubicTo(cx - k, cy + rad, cx - rad, cy + k, cx - rad, cy)
    p.cubicTo(cx - rad, cy - k, cx - k, cy - rad, cx, cy - rad); p.cubicTo(cx + k, cy - rad, cx + rad, cy - k, cx + rad, cy)
    p.close(); return p

def ring(cx, cy, rad, t=1):
    return pathops.op(disc(cx, cy, rad), disc(cx, cy, rad - t), PathOp.DIFFERENCE)

def tri(d, hw, h, shift=0):
    """Triangle pointing up/down/left/right, centred in the box so that every row
    (or column) of it falls on whole pixels; the tip is half a px blunt."""
    a, b = h / 2, .5
    along = 4.5 if h % 2 else 5.0          # centre along the pointing axis
    pts = [(-hw, -a), (hw, -a), (b, a), (-b, a)]
    if d in ("up", "down"):
        sgn = 1 if d == "up" else -1
        return poly([(5.5 + x, along + sgn * y) for x, y in pts])
    sgn = 1 if d == "right" else -1
    return poly([(along + shift + sgn * y, 4.5 + x) for x, y in pts])

def restore():
    back = pathops.op(frame(3, 3, 10, 9, 1, 2), rect(1, 0, 8, 6), PathOp.DIFFERENCE)
    return union(back, frame(1, 0, 8, 6, 1, 2))

def help_mark():
    return union(line([(3.2, 6.4), (3.2, 8.2), (7.8, 8.2), (7.8, 5.2), (5.5, 3.6), (5.5, 2.6)], .8, r=1.2), rect(4.7, 0, 6.3, 1.6))

def check():
    return line([(1.8, 4.6), (4.4, 2.0), (9.4, 7.4)], .9)

def cross(w=.8):
    return union(line([(2, .5), (9, 8.5)], w), line([(9, .5), (2, 8.5)], w))

def grip():
    return union(*[line([(10.5 - k, 0), (10.5, k)], .5) for k in (3, 6, 9)])

def chevron(d):
    pts = {"up": [(2, 2.8), (5.5, 6.3), (9, 2.8)], "down": [(2, 6.2), (5.5, 2.7), (9, 6.2)],
           "left": [(7.2, 1), (3.7, 4.5), (7.2, 8)], "right": [(3.8, 1), (7.3, 4.5), (3.8, 8)]}[d]
    return line(pts, .7)


# name: (drawing, unicode code points, Marlett slot or None)
SYMBOLS = {
    "minimize": (lambda: rect(2, 0, 8, 2), [0x1F5D5], "0"),
    "maximize": (lambda: frame(1, 0, 10, 9, 1, 2), [0x1F5D6], "1"),
    "restore": (restore, [0x1F5D7], "2"),
    "close": (cross, [0x1F5D9, 0x2715], "r"),
    "help": (help_mark, [0x2753], "s"),
    "arrowleft": (lambda: tri("left", 3.5, 4), [0x25C0], "3"),
    "arrowright": (lambda: tri("right", 3.5, 4), [0x25B6], "4"),
    "arrowup": (lambda: tri("up", 3.5, 4), [0x25B2], "5"),
    "arrowdown": (lambda: tri("down", 3.5, 4), [0x25BC], "6"),
    "smallup": (lambda: tri("up", 2.5, 3), [0x25B4], "t"),
    "smalldown": (lambda: tri("down", 2.5, 3), [0x25BE], "u"),
    "menuarrow": (lambda: tri("right", 3.5, 4, shift=1), [0x25B8], "8"),
    "menuarrowleft": (lambda: tri("left", 3.5, 4, shift=-1), [0x25C2], "w"),
    "check": (check, [0x2713], "a"),
    "checkbold": (lambda: line([(1.8, 4.6), (4.4, 2.0), (9.4, 7.4)], 1.2), [0x2714], "b"),
    "bullet": (lambda: disc(5.5, 4.5, 2.5), [0x25CF], "h"),
    "radiodot": (lambda: disc(5.5, 4.5, 1.7), [0x2022], "i"),
    "radio": (lambda: ring(5.5, 4.5, 4.5), [0x25CB], None),
    "radioon": (lambda: union(ring(5.5, 4.5, 4.5), disc(5.5, 4.5, 1.7)), [0x25C9], None),
    "box": (lambda: frame(1, 0, 10, 9), [0x2610], None),
    "boxcheck": (lambda: union(frame(1, 0, 10, 9), line([(2.8, 4.6), (4.6, 2.8), (8.2, 6.6)], .7)), [0x2611], None),
    "boxcross": (lambda: union(frame(1, 0, 10, 9), line([(3, 2), (8, 7)], .6), line([(8, 2), (3, 7)], .6)), [0x2612], None),
    "grip": (grip, [0x25E2], "o"),
    "chevronup": (lambda: chevron("up"), [0x2303], None),
    "chevrondown": (lambda: chevron("down"), [0x2304], None),
    "chevronleft": (lambda: chevron("left"), [0x2039], None),
    "chevronright": (lambda: chevron("right"), [0x203A], None),
    "menu": (lambda: union(rect(1, 1, 10, 2), rect(1, 4, 10, 5), rect(1, 7, 10, 8)), [0x2630], None),
    "more": (lambda: union(rect(1, 0, 3, 2), rect(4.5, 0, 6.5, 2), rect(8, 0, 10, 2)), [0x2026], None),
    "morevertical": (lambda: union(rect(4.5, 0, 6.5, 2), rect(4.5, 3.5, 6.5, 5.5), rect(4.5, 7, 6.5, 9)), [0x22EE], None),
    "plus": (lambda: union(rect(1, 3.5, 10, 5.5), rect(4.5, 0, 6.5, 9)), [0x002B], None),
    "minus": (lambda: rect(1, 3.5, 10, 5.5), [0x2212], None),
}


def build(outdir, version, copyright, license_text):
    px = FAM.px
    order, glyphs, metrics, cmap = [".notdef", "space"], {}, {}, {0x20: "space", 0xA0: "space", 0xF020: "space"}

    def emit(name, path):
        pen = TTGlyphPen(None)
        path.convertConicsToQuads()
        path.draw(TransformPen(Cu2QuPen(pen, 1.0, reverse_direction=False), (px, 0, 0, px, 0, 0)))
        gl = pen.glyph(); gl.recalcBounds(None)
        glyphs[name] = gl
        metrics[name] = (ADV * px, getattr(gl, "xMin", 0) if gl.numberOfContours else 0)

    emit(".notdef", frame(1, 0, 10, 9)); emit("space", pathops.Path())
    for name, (fn, cps, slot) in SYMBOLS.items():
        order.append(name); emit(name, fn())
        for cp in cps:
            cmap[cp] = name
        if slot:
            cmap[ord(slot)] = name; cmap[0xF000 + ord(slot)] = name
    fb = FontBuilder(FAM.upm, isTTF=True)
    fb.setupGlyphOrder(order); fb.setupCharacterMap(cmap); fb.setupGlyf(glyphs); fb.setupHorizontalMetrics(metrics)
    asc, desc = int(FAM.line_asc * px), int(FAM.line_desc * px)
    fb.setupHorizontalHeader(ascent=asc, descent=-desc, lineGap=0)
    fb.setupNameTable({"copyright": copyright, "familyName": FAM.name, "styleName": FAM.style,
                       "uniqueFontIdentifier": f"{version};FOLD;{FAM.name}-{FAM.style}", "fullName": f"{FAM.name} {FAM.style}",
                       "psName": f"{FAM.name}-{FAM.style}", "version": f"Version {version}",
                       "licenseDescription": license_text, "licenseInfoURL": "https://openfontlicense.org"})
    fb.setupOS2(sTypoAscender=asc, sTypoDescender=-desc, sTypoLineGap=0, usWinAscent=asc, usWinDescent=desc,
                sCapHeight=int(FAM.cap * px), fsSelection=0x40 | 0x80, version=4, achVendID="FOLD")
    fb.setupPost(isFixedPitch=1)
    fb.font["OS/2"].recalcUnicodeRanges(fb.font)
    os.makedirs(outdir, exist_ok=True)
    out = os.path.join(outdir, f"{FAM.name}-{FAM.style}.ttf")
    fb.save(out)
    nbits = bitmap.Strike(out, FAM).embed()
    return out, len(order), nbits
