"""Automatic kerning from glyph shapes.

Each glyph is rasterised; for every scanline we record how far the ink sits
from the glyph's left edge and from its advance edge. For a pair, the smallest
combined margin over the scanlines both glyphs occupy is the visible gap. Pairs
whose gap is clearly wider than the normal stem-to-stem gap are tightened by
half the excess, capped, and written as a class-based `kern` feature.
"""
import unicodedata

import numpy as np
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont

RES = 8            # raster pixels per font px
BIG = 99.0
PUNCT = set(".,:;!?'\"-()[]/‘’‚“”„«»‹›–—…·")


def eligible(cp):
    ch = chr(cp)
    if 0x180 <= cp < 0x370 or 0x460 <= cp < 0x2000 or cp >= 0x2E80:   # extended sets and CJK: left unkerned
        return False
    return unicodedata.category(ch) in ("Lu", "Ll") or ch in PUNCT


def profiles(path, fam, names_by_cp):
    """-> {glyphname: (left_margins, right_margins)} in px, one value per scanline."""
    size = int(fam.em * RES)
    font = ImageFont.truetype(path, size)
    tt = TTFont(path)
    hmtx = tt["hmtx"]
    pad = 4 * RES
    top = int((fam.line_asc + 3) * RES)
    h = top + int((fam.line_desc + 3) * RES)
    out = {}
    for cp, name in names_by_cp.items():
        adv = hmtx[name][0] / fam.px
        w = int(adv * RES) + 2 * pad
        im = Image.new("L", (w, h), 0)
        ImageDraw.Draw(im).text((pad, top), chr(cp), font=font, fill=255, anchor="ls")
        a = np.asarray(im) > 96
        has = a.any(axis=1)
        first = np.where(has, a.argmax(axis=1), 0)
        last = np.where(has, w - 1 - a[:, ::-1].argmax(axis=1), 0)
        lm = np.where(has, (first - pad) / RES, BIG)
        rm = np.where(has, adv - (last + 1 - pad) / RES, BIG)
        out[name] = (_dilate(lm), _dilate(rm))
    return out


def _dilate(m, k=RES // 4):
    """Spread each scanline's margin a quarter px up and down, so near misses count."""
    r = m.copy()
    for d in range(1, k + 1):
        r[d:] = np.minimum(r[d:], m[:-d])
        r[:-d] = np.minimum(r[:-d], m[d:])
    return r


def compute(path, fam, target=None, strength=0.5, max_kern=2, min_kern=0.75):
    target = sum(fam.sb) if target is None else target      # the family's normal stem-to-stem gap
    tt = TTFont(path)
    cmap = tt.getBestCmap()
    names = {cp: n for cp, n in cmap.items() if eligible(cp)}
    prof = profiles(path, fam, names)
    glyphs = sorted(set(names.values()))
    LM = np.array([prof[g][0] for g in glyphs])
    RM = np.array([prof[g][1] for g in glyphs])

    # glyphs with the same profile behave identically: group them into classes
    def classes(M):
        seen, idx = {}, []
        for row in np.round(np.minimum(M[:, ::RES // 2], 4.0) * 2).astype(int):   # half-px grid: fewer, larger classes
            idx.append(seen.setdefault(row.tobytes(), len(seen)))
        return np.array(idx), len(seen)

    lcls, nl = classes(RM)      # as the left glyph of a pair, the right margin matters
    rcls, nr = classes(LM)
    lrep = [np.argmax(lcls == i) for i in range(nl)]
    rrep = [np.argmax(rcls == i) for i in range(nr)]
    R = RM[lrep]; L = LM[rrep]
    rules = []
    for i in range(nl):
        gap = (R[i][None, :] + L).min(axis=1)          # per right-class
        for j in np.nonzero(gap < BIG / 2)[0]:
            k = strength * (gap[j] - target)
            if k < min_kern:
                continue
            # whole pixels only: browsers and Qt blur every glyph after a fractional offset
            k = min(int(k + .5), max_kern)
            rules.append((i, j, -int(k * fam.px)))
    lmembers = [[g for g, c in zip(glyphs, lcls) if c == i] for i in range(nl)]
    rmembers = [[g for g, c in zip(glyphs, rcls) if c == i] for i in range(nr)]
    return rules, lmembers, rmembers


def add(path, fam):
    rules, lm, rm = compute(path, fam)
    usedl = sorted({r[0] for r in rules}); usedr = sorted({r[1] for r in rules})
    fea = ["languagesystem DFLT dflt;", "languagesystem latn dflt;",
           "languagesystem grek dflt;", "languagesystem cyrl dflt;", "languagesystem hebr dflt;"]
    for i in usedl:
        fea.append(f"@L{i} = [{' '.join(lm[i])}];")
    for j in usedr:
        fea.append(f"@R{j} = [{' '.join(rm[j])}];")
    fea.append("feature kern {")
    for i, j, v in rules:
        fea.append(f"  pos @L{i} @R{j} {v};")
    fea.append("} kern;")
    tt = TTFont(path)
    addOpenTypeFeaturesFromString(tt, "\n".join(fea), tables=["GPOS"])   # leave GSUB (locl) alone
    tt.save(path)
    return len(rules)
