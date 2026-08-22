"""CJK from Fusion Pixel Font 12 px (SIL OFL 1.1; see vendor/fusion-pixel/).

The upstream BDF bitmaps are imported as they are: each glyph becomes a
pixel-shaped outline (so it scales) and its exact bitmap goes into the embedded
strike. zh_Hans is the default form; glyphs that differ in the Japanese, Korean
and Traditional Chinese builds are added as alternates behind the `locl` feature.
"""
import os
import pickle

import pathops
from fontTools.pens.ttGlyphPen import TTGlyphPen

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VENDOR = os.path.join(HERE, "vendor", "fusion-pixel")
FILE = "fusion-pixel-12px-proportional-%s.bdf"
DEFAULT = "zh_hans"
VARIANTS = {"ja": "JAN", "ko": "KOR", "zh_hant": "ZHT"}       # BDF language -> OpenType language tag
RANGES = [(0x2E80, 0x2FE0), (0x3000, 0x3400), (0x3400, 0x4DC0), (0x4E00, 0xA000), (0xAC00, 0xD7A4),
          (0xF900, 0xFB00), (0xFE30, 0xFE50), (0xFF00, 0xFFF0)]
PX = 100


def wanted(cp):
    return any(a <= cp < b for a, b in RANGES)


def parse_bdf(path):
    """-> {codepoint: (advance, w, h, xoff, yoff, rows)}; rows are ints, MSB = leftmost pixel of a byte-padded row."""
    out = {}
    cp = adv = bbx = None
    rows = None
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            if rows is not None:
                if line.startswith("ENDCHAR"):
                    if cp is not None and wanted(cp):
                        out[cp] = (adv,) + bbx + (tuple(rows),)
                    rows = None
                else:
                    rows.append(int(line.strip(), 16))
            elif line.startswith("ENCODING"):
                cp = int(line.split()[1])
            elif line.startswith("DWIDTH"):
                adv = int(line.split()[1])
            elif line.startswith("BBX"):
                bbx = tuple(int(v) for v in line.split()[1:5])
            elif line.startswith("BITMAP"):
                rows = []
    return out


def pixels(g):
    """-> list of (x, y) lit pixels in px, y up, relative to the glyph origin."""
    adv, w, h, xo, yo, rows = g
    nbits = ((w + 7) // 8) * 8
    pts = []
    for r, v in enumerate(rows):
        y = yo + h - 1 - r
        for x in range(w):
            if v >> (nbits - 1 - x) & 1:
                pts.append((xo + x, y))
    return pts


def outline(g):
    """Pixel-shaped TrueType glyph: horizontal runs merged into one clean outline."""
    adv, w, h, xo, yo, rows = g
    nbits = ((w + 7) // 8) * 8
    path = pathops.Path()
    for r, v in enumerate(rows):
        y = (yo + h - 1 - r) * PX
        x = 0
        while x < w:
            if v >> (nbits - 1 - x) & 1:
                x0 = x
                while x < w and v >> (nbits - 1 - x) & 1:
                    x += 1
                a, b = (xo + x0) * PX, (xo + x) * PX
                path.moveTo(a, y); path.lineTo(b, y); path.lineTo(b, y + PX); path.lineTo(a, y + PX); path.close()
            else:
                x += 1
    path.simplify(fix_winding=True, keep_starting_points=False)
    pen = TTGlyphPen(None)
    path.draw(pen)
    gl = pen.glyph()
    gl.recalcBounds(None)
    return gl


def load(existing):
    """-> dict(glyphs={name: bdf glyph}, cmap={cp: name}, locl={tag: {name: altname}}, outlines={name: TTGlyph})"""
    files = {lang: os.path.join(VENDOR, FILE % lang) for lang in [DEFAULT] + list(VARIANTS)}
    if not all(os.path.exists(p) for p in files.values()):
        return None
    cache = os.path.join(HERE, "build", "cjk-cache.pkl")
    stamp = tuple(os.path.getmtime(p) for p in files.values()) + (os.path.getmtime(__file__),)
    data = None
    if os.path.exists(cache):
        with open(cache, "rb") as f:
            data = pickle.load(f)
        if data.get("stamp") != stamp:
            data = None
    if data is None:
        base = parse_bdf(files[DEFAULT])
        glyphs, cmap, locl = {}, {}, {}
        for cp, g in sorted(base.items()):
            name = "uni%04X" % cp
            glyphs[name] = g; cmap[cp] = name
        for lang, tag in VARIANTS.items():
            alt = parse_bdf(files[lang])
            subs = {}
            for cp, g in sorted(alt.items()):
                if cp in base and g != base[cp]:
                    name = "uni%04X.%s" % (cp, tag)
                    glyphs[name] = g; subs["uni%04X" % cp] = name
                elif cp not in base:              # only this language's build has it: make it the default
                    name = "uni%04X" % cp
                    if name not in glyphs:
                        glyphs[name] = g; cmap[cp] = name
            locl[tag] = subs
        data = dict(stamp=stamp, glyphs=glyphs, cmap=cmap, locl=locl,
                    outlines={n: outline(g) for n, g in glyphs.items()})
        os.makedirs(os.path.dirname(cache), exist_ok=True)
        with open(cache, "wb") as f:
            pickle.dump(data, f)
    drop = {n for cp, n in data["cmap"].items() if cp in existing}
    if drop:                                       # never replace a glyph the family draws itself
        data = dict(data, cmap={cp: n for cp, n in data["cmap"].items() if cp not in existing})
    return data


def locl_fea(data, present):
    lines = ["languagesystem DFLT dflt;", "languagesystem hani dflt;", "languagesystem kana dflt;", "languagesystem hang dflt;"]
    for tag in data["locl"]:
        lines += ["languagesystem hani %s;" % tag, "languagesystem kana %s;" % tag, "languagesystem hang %s;" % tag]
    lines.append("feature locl {")
    for script in ("hani", "kana", "hang"):
        lines.append("  script %s;" % script)
        for tag, subs in data["locl"].items():
            lines.append("  language %s;" % tag)
            for a, b in subs.items():
                if a in present and b in present:
                    lines.append("    sub %s by %s;" % (a, b))
    lines.append("} locl;")
    return "\n".join(lines)
