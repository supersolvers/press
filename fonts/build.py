#!/usr/bin/env python3
"""Build the fonts: ./.venv/bin/python build.py  ->  build/*.ttf"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "src"))

from fontTools.agl import UV2AGL
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen

from engine import Ctx, Family, ink_for
from glyphs import GLYPHS
import latin  # noqa: F401  (registers more glyphs)
import scripts  # noqa: F401  (Greek and Cyrillic)
import more  # noqa: F401  (extended Latin/Cyrillic, polytonic Greek, Hebrew, symbols)
import geneva  # noqa: F401  (width column for the Geneva-style family)
import bitmap
import borrow
import boxes
import cjk
import kern
import symbols
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

Ctx.registry = GLYPHS

# Working names — placeholders until final names are chosen.
FAMILIES = [
    # heavy, high-contrast UI face in the spirit of Chicago 12
    Family(name="Cicero", style="Regular", em=12, cap=9, xh=7, asc=10, desc=3,
           hx=1.0, hy=0.5, r=1.8, sb=(0, 1), wkey="ch", chicago=True, line_asc=13, line_desc=3),
    # light monoline UI face in the spirit of MS Sans Serif 8
    Family(name="Agate", style="Regular", em=11, cap=8, xh=6, asc=8, desc=2,
           hx=0.5, hy=0.5, r=3.0, sb=(0, 1), wkey="ag", line_asc=11, line_desc=2),
    # its bold: stems doubled, every glyph one px wider, as the bitmap original was emboldened
    Family(name="Agate", style="Bold", em=11, cap=8, xh=6, asc=8, desc=2,
           hx=1.0, hy=0.5, r=3.0, sb=(0, 1), wkey="ag", line_asc=11, line_desc=2,
           wadd=1, weight=700),
    # light, airy sans in the spirit of Geneva 12: Agate's skeleton on a taller 12 px body with looser spacing
    Family(name="Brevier", style="Regular", em=12, cap=9, xh=7, asc=10, desc=3,
           hx=0.5, hy=0.5, r=3.2, sb=(1, 1), wkey="ag", wcol="ge", line_asc=13, line_desc=3),
    # monospaced face in the spirit of Monaco 12: every glyph in a 7 px cell
    Family(name="Pica", style="Regular", em=12, cap=9, xh=7, asc=9, desc=3,
           hx=0.5, hy=0.5, r=2.2, sb=(1, 1), wkey="ag", line_asc=12, line_desc=3,
           mono=7, cjk_adv=14),
    # heavy monospaced face in the spirit of Fixedsys / the VGA Terminal font: 8x16 cell, doubled stems
    Family(name="Elite", style="Regular", em=16, cap=10, xh=7, asc=10, desc=3,
           hx=1.0, hy=0.5, r=1.6, sb=(1, 1), wkey="ch", line_asc=13, line_desc=3,
           mono=8, cjk_adv=16, compose=True),
]

COPYRIGHT = "Copyright 2026 The Fonts of Old Project Authors."
CJK_COPYRIGHT = ("CJK glyphs from Fusion Pixel Font, Copyright (c) 2022 TakWolf, which incorporates "
                 "Ark Pixel Font, Copyright (c) 2021 TakWolf; Cubic 11 (with JF Dot M+H 12, Copyright (c) 2005 "
                 "M+ FONTS PROJECT, and M+ BITMAP FONTS, Copyright (C) 2002-2004 COZ); and Galmuri, "
                 "Copyright (c) 2019-2025 Lee Minseo.")
VERSION = "0.001"
LICENSE = ("This Font Software is licensed under the SIL Open Font License, Version 1.1. "
           "This license is available with a FAQ at: https://openfontlicense.org")


def gname(ch):
    cp = ord(ch)
    return UV2AGL.get(cp, "uni%04X" % cp)


def draw(fam, ch):
    fn, inks, sbs = GLYPHS[ch]
    sb = sbs[fam.wkey] or (None, None)
    c = Ctx(fam, ink_for(fam, inks), sb[0], sb[1])
    fn(c)
    return c


def build(fam, outdir):
    px = fam.px
    order = [".notdef"]
    glyphs, metrics, cmap = {}, {}, {}

    def emit(name, path, adv):
        pen = TTGlyphPen(None)
        path.convertConicsToQuads()
        shift = 0
        if fam.mono and adv:               # fixed pitch: centre whatever was drawn in the cell
            sx = min(1.0, fam.mono / adv)      # composites wider than the cell are squeezed to fit
            shift = round((fam.mono - adv * sx) / 2) * px
            adv = fam.mono
        else:
            sx = 1.0
        path.draw(TransformPen(Cu2QuPen(pen, 1.0, reverse_direction=False), (px * sx, 0, 0, px, shift, 0)))
        gl = pen.glyph()
        glyphs[name] = gl
        gl.recalcBounds(None)
        metrics[name] = (int(round(adv * px)), getattr(gl, "xMin", 0) if gl.numberOfContours else 0)

    nd = Ctx(fam, 5 + fam.wadd if fam.wkey == "ag" else 6)
    nd.s([(0, nd.B), (1, nd.B), (1, nd.C), (0, nd.C)], closed=True)
    emit(".notdef", nd.result(), nd.adv)

    for ch in sorted(GLYPHS):
        c = draw(fam, ch)
        n = gname(ch)
        order.append(n)
        cmap[ord(ch)] = n
        emit(n, c.result(), c.adv)
    if " " in GLYPHS:
        cmap[0xA0] = gname(" ")

    # CJK: imported pixel glyphs (see src/cjk.py); never replaces a glyph drawn above
    han = cjk.load(set(cmap))
    extra = {}
    if han:
        keep = set(han["cmap"].values())
        for name, g in han["glyphs"].items():
            if name.split(".")[0] not in keep:
                continue
            order.append(name)
            gl = han["outlines"][name]
            glyphs[name] = gl
            cell = fam.cjk_adv if g[0] > fam.mono else fam.mono                    # full-width = two cells, half-width = one
            pad = (cell - g[0]) // 2 if fam.mono and cell >= g[0] else 0
            odd = (cell - g[0]) % 2 if fam.mono and cell >= g[0] else 0
            # a left bearing larger than xMin shifts the glyph right without touching its outline
            metrics[name] = ((g[0] + 2 * pad + odd) * px, (getattr(gl, "xMin", 0) if gl.numberOfContours else 0) + pad * px)
            extra[name] = (g[0] + 2 * pad + odd, g[1], g[2], g[3] + pad) + g[4:] if pad or odd else g
        cmap.update(han["cmap"])

    # box drawing and block elements, for the fixed-width families (see src/boxes.py)
    for cp, name, g, gl in boxes.load(fam, set(cmap)):
        order.append(name)
        glyphs[name] = gl
        metrics[name] = (g[0] * px, getattr(gl, "xMin", 0) if gl.numberOfContours else 0)
        extra[name] = g
        cmap[cp] = name

    fb = FontBuilder(fam.upm, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    asc, desc = int(fam.line_asc * px), int(fam.line_desc * px)
    bold = fam.style == "Bold"
    fb.setupHorizontalHeader(ascent=asc, descent=-desc, lineGap=0)
    full = f"{fam.name} {fam.style}"
    fb.setupNameTable({
        "copyright": COPYRIGHT + (" " + CJK_COPYRIGHT if han else ""),
        "familyName": fam.name, "styleName": fam.style,
        "uniqueFontIdentifier": f"{VERSION};FOLD;{fam.name}-{fam.style}",
        "fullName": full, "psName": f"{fam.name}-{fam.style}",
        "version": f"Version {VERSION}", "licenseDescription": LICENSE,
        "licenseInfoURL": "https://openfontlicense.org",
    })
    fb.setupOS2(sTypoAscender=asc, sTypoDescender=-desc, sTypoLineGap=0,
                usWinAscent=max(asc, max(getattr(g, "yMax", 0) for g in glyphs.values())),
                usWinDescent=max(desc, -min(getattr(g, "yMin", 0) for g in glyphs.values())),
                sxHeight=int(fam.xh * px), sCapHeight=int(fam.cap * px),
                fsSelection=(0x20 if bold else 0x40) | 0x80, version=4, achVendID="FOLD",
                usWeightClass=fam.weight)
    fb.setupPost(underlinePosition=-int(1.5 * px), underlineThickness=int(2 * fam.hy * px), keepGlyphNames=False)
    fb.font["OS/2"].recalcUnicodeRanges(fb.font)
    if fam.mono:
        fb.font["post"].isFixedPitch = 1
        fb.font["OS/2"].panose.bProportion = 9
        fb.font["OS/2"].xAvgCharWidth = fam.mono * px
    #                       Latin1 Latin2 Cyrillic Greek Turkish Hebrew Baltic Vietnamese + JIS, GBK, Korean, Big5
    fb.font["OS/2"].ulCodePageRange1 = sum(1 << b for b in (0, 1, 2, 3, 4, 5, 7, 8) + ((17, 18, 19, 20) if han else ()))
    if han:
        addOpenTypeFeaturesFromString(fb.font, cjk.locl_fea(han, set(order)), tables=["GSUB"])
    os.makedirs(outdir, exist_ok=True)
    out = os.path.join(outdir, f"{fam.name}-{fam.style}.ttf")
    if bold:
        fb.font["head"].macStyle = 1
    fb.save(out)
    pairs = 0 if fam.mono else kern.add(out, fam)
    here = os.path.dirname(os.path.abspath(__file__))
    strike = bitmap.Strike(out, fam, os.path.join(here, "pixels", f"{fam.name}-{fam.style}.txt"), extra, order)
    nbits = strike.embed()
    return out, len(order), pairs, nbits


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    for fam in FAMILIES:
        out, n, pairs, nbits = build(fam, os.path.join(here, "build"))
        print(f"{out}: {n} glyphs, {pairs} kerning class pairs, {nbits} bitmaps at {int(fam.em)} px")
    out, n, nbits = symbols.build(os.path.join(here, "build"), VERSION, COPYRIGHT, LICENSE)
    print(f"{out}: {n} glyphs, {nbits} bitmaps at {int(symbols.FAM.em)} px")
    for res in (borrow.build_icons(os.path.join(here, "build"), VERSION, COPYRIGHT, LICENSE),
                borrow.build_hieroglyphs(os.path.join(here, "build"), VERSION, COPYRIGHT, LICENSE)):
        if res:
            print(f"{res[0]}: {res[1]} glyphs, {res[2]} bitmaps at {res[3]} px")
