"""Companion fonts repackaged from existing SIL OFL 1.1 fonts, renamed and
scaled to this set's 12-px-em units:

  Vignette   general UI icons, from Font Awesome 4.7.0 (brand logos left out)
  Cartouche  Egyptian hieroglyphs, from Noto Sans Egyptian Hieroglyphs

Neither source is a pixel design, so both are restyled to match the set: every
glyph is redrawn as a 1-bit bitmap at a native size (icons 14 px, the grid Font
Awesome 4 was designed on; hieroglyphs 24 px with their hairlines thickened so
they survive). The font carries that bitmap as an embedded strike plus a
pixel-shaped outline for other sizes.
"""
import math
import os
import re

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

import bitmap
import cjk
from engine import Family

UPM = 1200

VENDOR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "vendor")
FA_FONT = os.path.join(VENDOR, "font-awesome", "fontawesome-webfont.ttf")
FA_NAMES = os.path.join(VENDOR, "font-awesome", "_variables.scss")
NOTO_EH = os.path.join(VENDOR, "noto-egyptian", "NotoSansEgyptianHieroglyphs-Regular.ttf")

# Logos and marks of companies, products and franchises: trademarks, not ours to redistribute as icons.
BRANDS = set("""500px adn amazon android angellist apple bandcamp behance behance-square bitbucket bitbucket-square
black-tie bluetooth bluetooth-b buysellads cc-amex cc-diners-club cc-discover cc-jcb cc-mastercard cc-paypal cc-stripe
cc-visa chrome codepen codiepie connectdevelop contao creative-commons css3 dashcube delicious deviantart digg dribbble
dropbox drupal edge eercast empire envira etsy expeditedssl fa font-awesome facebook facebook-f facebook-official
facebook-square firefox first-order flickr fonticons fort-awesome forumbee foursquare free-code-camp ge get-pocket gg
gg-circle git git-square github github-alt github-square gitlab gittip glide glide-g google google-plus
google-plus-circle google-plus-official google-plus-square google-wallet gratipay grav hacker-news houzz html5 imdb
instagram internet-explorer ioxhost joomla jsfiddle lastfm lastfm-square leanpub linkedin linkedin-square linode linux
maxcdn meanpath medium meetup mixcloud modx odnoklassniki odnoklassniki-square opencart openid opera optin-monster
pagelines paypal pied-piper pied-piper-alt pied-piper-pp pinterest pinterest-p pinterest-square product-hunt qq quora
ra ravelry rebel reddit reddit-alien reddit-square renren resistance safari scribd sellsy shirtsinbulk simplybuilt
skyatlas skype slack slideshare snapchat snapchat-ghost snapchat-square soundcloud spotify stack-exchange
stack-overflow steam steam-square stumbleupon stumbleupon-circle superpowers telegram tencent-weibo themeisle trello
tripadvisor tumblr tumblr-square twitch twitter twitter-square usb viacoin viadeo viadeo-square vimeo vimeo-square
vine vk wechat weibo weixin whatsapp wikipedia-w windows wordpress wpbeginner wpexplorer wpforms xing xing-square
y-combinator y-combinator-square yahoo yc yc-square yelp yoast youtube youtube-play youtube-square""".split())


def fa_names():
    """-> {codepoint: [icon names]} from Font Awesome's own variable list."""
    out = {}
    for m in re.finditer(r'\$fa-var-([\w-]+): "\\(\w+)";', open(FA_NAMES, encoding="utf-8").read()):
        out.setdefault(int(m.group(2), 16), []).append(m.group(1))
    return out


def repack(src, family, keep, copyright, version, license_text, outdir, line=None):
    """Copy the kept glyphs of `src` into a new font called `family`."""
    tt = TTFont(src)
    scale = UPM / tt["head"].unitsPerEm
    gs = tt.getGlyphSet()
    hmtx = tt["hmtx"]
    order, glyphs, metrics, cmap = [".notdef"], {}, {}, {}

    def emit(new, old):
        rec = DecomposingRecordingPen(gs)
        gs[old].draw(rec)
        pen = TTGlyphPen(None)
        rec.replay(TransformPen(pen, (scale, 0, 0, scale, 0, 0)))
        gl = pen.glyph()
        if gl.numberOfContours:
            gl.coordinates.toInt()
        gl.recalcBounds(None)
        glyphs[new] = gl
        metrics[new] = (int(round(hmtx[old][0] * scale)), getattr(gl, "xMin", 0) if gl.numberOfContours else 0)

    emit(".notdef", tt.getGlyphOrder()[0])
    for cp, old in sorted(tt.getBestCmap().items()):
        if not keep(cp):
            continue
        new = "u%04X" % cp
        if new not in glyphs:
            order.append(new); emit(new, old)
        cmap[cp] = new
    hh = tt["hhea"]
    asc, desc = line or (int(round(hh.ascent * scale)), int(round(-hh.descent * scale)))
    fb = FontBuilder(UPM, isTTF=True)
    fb.setupGlyphOrder(order); fb.setupCharacterMap(cmap); fb.setupGlyf(glyphs); fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=asc, descent=-desc, lineGap=0)
    fb.setupNameTable({"copyright": copyright, "familyName": family, "styleName": "Regular",
                       "uniqueFontIdentifier": f"{version};FOLD;{family}-Regular", "fullName": f"{family} Regular",
                       "psName": f"{family}-Regular", "version": f"Version {version}",
                       "licenseDescription": license_text, "licenseInfoURL": "https://openfontlicense.org"})
    ymax = max(getattr(g, "yMax", 0) for g in glyphs.values()); ymin = min(getattr(g, "yMin", 0) for g in glyphs.values())
    fb.setupOS2(sTypoAscender=asc, sTypoDescender=-desc, sTypoLineGap=0, usWinAscent=max(asc, ymax),
                usWinDescent=max(desc, -ymin), fsSelection=0x40 | 0x80, version=4, achVendID="FOLD")
    fb.setupPost(keepGlyphNames=False)
    fb.font["OS/2"].recalcUnicodeRanges(fb.font)
    os.makedirs(outdir, exist_ok=True)
    out = os.path.join(outdir, f"{family}-Regular.ttf")
    fb.save(out)
    return out, len(order)


SS = 8


def pixelize(smooth, family, ppem, thresh, grow, copyright, version, license_text, outdir, art=None):
    """Redraw every glyph of the font file `smooth` as 1-bit pixels at `ppem` and build the final font."""
    tt = TTFont(smooth)
    upm = tt["head"].unitsPerEm
    hmtx = tt["hmtx"]
    font = ImageFont.truetype(smooth, ppem * SS)
    asc = math.ceil(tt["hhea"].ascent * ppem / upm); desc = math.ceil(-tt["hhea"].descent * ppem / upm)
    top = asc + 3; rows = top + desc + 3
    order, glyphs, metrics, cmap, extra = [".notdef", "space"], {}, {}, {0x20: "space", 0xA0: "space"}, {}

    def add(name, g):
        gl = cjk.outline(g)
        glyphs[name] = gl
        metrics[name] = (g[0] * 100, getattr(gl, "xMin", 0) if gl.numberOfContours else 0)
        extra[name] = g

    box = ppem * 3 // 4
    add(".notdef", (box + 2, box, box, 1, 0, tuple([(1 << box) - 1] + [(1 << (box - 1)) | 1] * (box - 2) + [(1 << box) - 1])
                    if box % 8 == 0 else _rows(np.pad(np.zeros((box - 2, box - 2), bool), 1, constant_values=True))))
    add("space", (ppem // 2, 0, 0, 0, 0, ()))
    for cp, old in sorted(tt.getBestCmap().items()):
        if cp in (0x20, 0xA0):
            continue
        adv = max(1, int(round(hmtx[old][0] * ppem / upm)))
        pad = 4
        big = Image.new("L", ((adv + 2 * pad) * SS, rows * SS), 0)
        ImageDraw.Draw(big).text((pad * SS, top * SS), chr(cp), font=font, fill=255, anchor="ls")
        if grow:
            big = big.filter(ImageFilter.MaxFilter(grow))        # thicken hairlines so they survive
        cov = np.asarray(big, dtype=float).reshape(rows, SS, adv + 2 * pad, SS).mean(axis=(1, 3)) / 255
        a = cov >= thresh
        ys, xs = np.nonzero(a)
        name = "u%04X" % cp
        if art and cp in art:                 # hand-drawn replacement, centred in the advance and the cell
            rows_ = art[cp]; w = len(rows_[0]); h = len(rows_)
            adv = max(adv, w + 1)
            px = np.array([[c == "#" for c in r] for r in rows_])
            g = (adv, w, h, (adv - w) // 2, int(math.floor((asc - desc) / 2 - h / 2 + .5)), _rows(px))
        elif len(ys):
            r0, r1, c0, c1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
            g = (adv, c1 - c0, r1 - r0, c0 - pad, top - r1, _rows(a[r0:r1, c0:c1]))
        else:
            g = (adv, 0, 0, 0, 0, ())
        order.append(name); add(name, g); cmap[cp] = name
    fb = FontBuilder(ppem * 100, isTTF=True)
    fb.setupGlyphOrder(order); fb.setupCharacterMap(cmap); fb.setupGlyf(glyphs); fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=asc * 100, descent=-desc * 100, lineGap=0)
    fb.setupNameTable({"copyright": copyright, "familyName": family, "styleName": "Regular",
                       "uniqueFontIdentifier": f"{version};FOLD;{family}-Regular", "fullName": f"{family} Regular",
                       "psName": f"{family}-Regular", "version": f"Version {version}",
                       "licenseDescription": license_text, "licenseInfoURL": "https://openfontlicense.org"})
    ymax = max(getattr(g, "yMax", 0) for g in glyphs.values()); ymin = min(getattr(g, "yMin", 0) for g in glyphs.values())
    fb.setupOS2(sTypoAscender=asc * 100, sTypoDescender=-desc * 100, sTypoLineGap=0, usWinAscent=max(asc * 100, ymax),
                usWinDescent=max(desc * 100, -ymin), fsSelection=0x40 | 0x80, version=4, achVendID="FOLD")
    fb.setupPost(keepGlyphNames=False)
    fb.font["OS/2"].recalcUnicodeRanges(fb.font)
    out = os.path.join(outdir, f"{family}-Regular.ttf")
    fb.save(out)
    fam = Family(name=family, style="Regular", em=ppem, cap=ppem * .75, xh=ppem * .5, asc=asc, desc=desc,
                 hx=.5, hy=.5, r=1, sb=(0, 0), wkey="ag", line_asc=asc, line_desc=desc)
    nbits = bitmap.Strike(out, fam, None, extra, order).embed()
    return out, len(order), nbits, ppem


def load_art(path, names):
    """Hand-drawn icons: blocks of "icon <name> [<name> ...]" followed by rows of '#' and '.'."""
    art = {}
    if not os.path.exists(path):
        return art
    lines = open(path, encoding="utf-8").read().split("\n") + [""]
    by_name = {n: cp for cp, ns in names.items() for n in ns}
    i = 0
    while i < len(lines):
        if lines[i].startswith("icon "):
            j = i + 1
            while lines[j].strip():
                j += 1
            rows = lines[i + 1:j]
            w = max(len(r) for r in rows)
            rows = [r.ljust(w, ".") for r in rows]
            for n in lines[i].split()[1:]:
                if n not in by_name:
                    raise SystemExit(f"{path}: unknown icon name {n!r}")
                art[by_name[n]] = rows
            i = j
        else:
            i += 1
    return art


def _rows(a):
    """Boolean pixel array -> BDF-style row integers (MSB = leftmost pixel, rows padded to whole bytes)."""
    w = a.shape[1]; nbits = ((w + 7) // 8) * 8
    return tuple(int(sum(1 << (nbits - 1 - x) for x in np.nonzero(r)[0])) for r in a)


def _restyle(src, family, keep, note, ppem, thresh, grow, version, license_text, outdir, art=None):
    tmp = os.path.join(outdir, "tmp-smooth")
    smooth, _ = repack(src, family, keep, note, version, license_text, tmp)
    try:
        return pixelize(smooth, family, ppem, thresh, grow, note, version, license_text, outdir, art)
    finally:
        os.remove(smooth); os.rmdir(tmp)


def build_icons(outdir, version, copyright, license_text):
    if not (os.path.exists(FA_FONT) and os.path.exists(FA_NAMES)):
        return None
    names = fa_names()
    keep = lambda cp: cp in (0x20, 0xA0) or (cp in names and not any(n in BRANDS for n in names[cp]))
    note = copyright + " Icons redrawn as pixels from Font Awesome 4.7.0, Copyright Dave Gandy 2016 (brand logos omitted)."
    art = load_art(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "pixels", "Vignette-Regular.txt"), names)
    return _restyle(FA_FONT, "Vignette", keep, note, 14, .5, 0, version, license_text, outdir, art)


def build_hieroglyphs(outdir, version, copyright, license_text):
    if not os.path.exists(NOTO_EH):
        return None
    keep = lambda cp: cp in (0x20, 0xA0) or 0x13000 <= cp <= 0x1345F
    note = copyright + " Hieroglyphs redrawn as pixels from Noto Sans Egyptian Hieroglyphs, Copyright 2017 Google Inc."
    return _restyle(NOTO_EH, "Cartouche", keep, note, 24, .5, 5, version, license_text, outdir)
