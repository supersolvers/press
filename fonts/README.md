# The press fonts

Pixel typefaces for a one-bit desktop, each drawn for the one size it is used at,
in the spirit of the system faces of the 1984 Macintosh to Windows 95. They
began as a project of their own, Fonts of Old; that project is now this folder.
`SELECTION.md` is the survey it started from.

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

Each font file carries outlines, for any size, and the hand-tuned pixels at
its own size as an embedded bitmap. press draws only the pixels (see
`crates/press-fontgen` and `crates/press-core/src/font.rs`); other programs
use the files through fontconfig, and need `fontconfig/65-press.conf` to show
the pixels rather than smoothed outlines.

## Building

From the top of the repository:

```sh
make fonts-venv    # once: Python with fontTools, skia-pathops, numpy and Pillow
make fonts         # build every family into fonts/build, and bundle them into press
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

1. **Drawn.** `src/` draws each letter as an outline from a skeleton and a
   pen (`engine.py`; the letters in `glyphs.py`, `latin.py`, `scripts.py`,
   `more.py`), with each family's widths, weight and heights (`build.py`,
   `FAMILIES`). Accented letters are put together from a letter and its
   accents (`latin.py`).
2. **Rendered.** The outline is rendered at the family's size and every pixel
   more than about two-fifths covered is ink (`bitmap.py`).
3. **Corrected by hand.** Where that is not good enough, the letter is drawn
   in pixels in `pixels/<Family>.txt`, which wins over the rendering:

   ```
   glyph Ж top=7
   .##.#.##
   .##.#.##
   ...
   ```

   `#` is ink, anything else blank; `top` is the row of the cell the first line
   is on (row 0 is the top of the strike; the baseline is below row
   `line_asc + 3`). Column 0 is the left edge of the advance.
4. **Composed in pixels** (Elite only, whose 8-pixel cell is too narrow for
   rendered accents): an accented letter is its plain letter, hand-tuned or
   not, with a small accent set above or hung below in pixels
   (`src/compose.py`). So an accented letter always follows its plain one; a
   hand-drawn one in the pixel file still wins.
5. **Box drawing** (Pica and Elite) is made in pixels from the characters'
   Unicode names (`src/boxes.py`), filling the cell so that lines join.
6. **Imported.** The CJK characters' pixels are taken as they are from the
   BDF files in `vendor/fusion-pixel` (`src/cjk.py`); the pictographs and the
   hieroglyphs are redrawn from `vendor/font-awesome` and
   `vendor/noto-egyptian` (`src/borrow.py`).

To check a change, build the family and render some sample text, as in the
README images. Look for strokes that merge into a solid block, like a
3 × 3 square in a light family. Check that similar letters still look
distinct. In the heavy faces, ä and ă once rendered pixel for pixel the same.

## Licence

The fonts are under the SIL Open Font License 1.1 (`OFL.txt`), not press's
GPL: anyone may use, change and pass them on, so long as a changed version
does not take their names. What they borrow keeps its own licence, in
`vendor/`: Fusion Pixel Font and the fonts it is built from (OFL 1.1), Font
Awesome 4.7 (the font OFL 1.1, its name list MIT) and Noto Sans Egyptian
Hieroglyphs (OFL 1.1).

None of the families is converted from Apple's or Microsoft's fonts, and none
uses their names; `65-press.conf` maps the old names to these, for documents
and programs that ask for them.
