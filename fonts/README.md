# PRESS fonts

These pixel typefaces are drawn at the sizes PRESS uses, with inspiration
from system fonts spanning the 1984 Macintosh to Windows 95. They originated
in Fonts of Old. The original font survey is in `SELECTION.md`.

| Family | Size | What it is for |
| --- | --- | --- |
| Cicero | 12 px | the interface: heavy and plain |
| Agate, Agate Bold | 11 px | a lot of words in a little room |
| Brevier | 12 px | reading: light and airy |
| Pica | 12 px, 7 px cells | columns and program text |
| Elite | 16 px, 8 px cells | a terminal: heavy |
| Fleuron | 12 px | signs for the interface (arrows, checks, bullets) |
| Vignette | 14 px | pictographs, redrawn from Font Awesome 4.7 |
| Cartouche | 24 px | Egyptian hieroglyphs, redrawn from Noto |

The text families have Latin (with Vietnamese), Greek (with polytonic),
Cyrillic and Hebrew letters of their own, and share some 34,000 Chinese,
Japanese and Korean characters taken from Fusion Pixel Font. Pica and Elite
have box drawing and block elements as well.

Each font file contains scalable outlines and a hand-tuned bitmap at its
target size. PRESS draws from the bitmap data (see `crates/press-fontgen` and
`crates/press-core/src/font.rs`). For apps that use fontconfig,
`fontconfig/65-press.conf` selects the embedded bitmaps.

## Building

From the top of the repository:

```sh
make fonts-venv    # once: Python with fontTools, skia-pathops, numpy and Pillow
make fonts         # build every family into fonts/build, and bundle them into PRESS
```

`make fonts-venv` needs Python's `venv` module (`python3-venv` on Debian and
Ubuntu) and a connection to download the packages with pip. `make install`
installs the fonts from `fonts/build`, so it needs `make fonts` first.

A build takes about a minute. To try one family alone:

```sh
cd fonts
.venv/bin/python -c "import build, os; f = next(f for f in build.FAMILIES if f.name == 'Elite'); print(build.build(f, os.path.abspath('build')))"
```

`pixels.py` previews the families at their sizes, and `pixels.py --dump "äß"`
prints letters in the form the pixel files use. `specimen.py` draws specimen
sheets.

## How a letter is made

1. **Draw the outline.** The engine in `src/engine.py` draws from a skeleton
   and a pen. The letter shapes live in `glyphs.py`, `latin.py`, `scripts.py`,
   and `more.py`. Widths, weight, and heights come from `FAMILIES` in
   `build.py`. For accented letters, `latin.py` combines the base and accents.
2. **Render at the target size.** `bitmap.py` fills any pixel where the
   outline covers more than about two-fifths of the pixel.
3. **Adjust individual glyphs.** A hand-drawn glyph in `pixels/<Family>.txt`
   takes precedence over the rendered glyph.

   ```
   glyph Ж top=7
   .##.#.##
   .##.#.##
   ...
   ```

   `#` marks ink. All other characters leave the pixel blank. `top` sets
   the row of the first line. Row 0 is the top of the strike, and the
   baseline sits below row `line_asc + 3`. Column 0 marks the left edge of
   the advance.
4. **Add accents in Elite.** Its 8-pixel cell is too narrow for rendered
   accents. `src/compose.py` places the accent above or below the current
   base bitmap, including any hand edits. A hand-drawn accented glyph
   still takes precedence.
5. **Join box-drawing lines in Pica and Elite.** `src/boxes.py` uses the
   Unicode character names to build glyphs whose lines meet at cell edges.
6. **Imported.** The CJK characters' pixels are taken as they are from the
   BDF files in `vendor/fusion-pixel` (`src/cjk.py`); the pictographs and the
   hieroglyphs are redrawn from `vendor/font-awesome` and
   `vendor/noto-egyptian` (`src/borrow.py`).

To check a change, build the family and render some sample text, as in the
README images. Look for strokes that merge into a solid block, like a
3 × 3 square in a light family. Check that similar letters still look
distinct. In the heavy faces, ä and ă once rendered pixel for pixel the same.

## Licence

The fonts are under the SIL Open Font License 1.1 (`OFL.txt`), not the toolkit's
GPL: anyone may use, change and pass them on, so long as a changed version
does not take their names. What they borrow keeps its own licence, in
`vendor/`: Fusion Pixel Font and the fonts it is built from (OFL 1.1), Font
Awesome 4.7 (the font OFL 1.1, its name list MIT) and Noto Sans Egyptian
Hieroglyphs (OFL 1.1).

None of the families is converted from Apple's or Microsoft's fonts, and none
uses their names; `65-press.conf` maps the old names to these, for documents
and programs that ask for them.
