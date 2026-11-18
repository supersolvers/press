# Fonts of Old — the selection notes

*These are the notes the fonts began from, when they were a project of their own
(Fonts of Old). They survey what fonts there were to stand in for the old system
faces, and their licences. The families in this folder are what came of it; see
README.md for them. The notes are kept as they were.*


A distro-shippable font set inspired by system typefaces from the 1984 Macintosh
to Windows 95/98. Status: **draft selection, nothing bundled or built yet.**

Licence check column:
- **checked** — licence read from the upstream page/file or from the font's own name table
- **second-hand** — stated on a catalogue/archived page only; needs a primary source before shipping
- **unchecked** — reported from memory; verify before shipping

Rules for the set:
1. Only OFL / CC0-public domain / MIT / Apache / (A)GPL-with-font-exception / LGPL / CC BY-SA.
2. Nothing converted from Apple's or Microsoft's actual font files.
3. No trademarked family names (Chicago, Geneva, New York, Monaco, Charcoal, San Francisco,
   Arial, Tahoma, Verdana, …). Ship under neutral names; map the old names with fontconfig aliases.

## Tier 1 — include as-is (already packaged by major distros)

| Stands in for | Font | Licence | Check | Metric-compatible |
|---|---|---|---|---|
| Arial / Helvetica | Liberation Sans (or Arimo) | OFL 1.1 | checked | yes |
| Times New Roman / Times | Liberation Serif (or Tinos) | OFL 1.1 | checked (Sans name table; Serif assumed same release) | yes |
| Courier New | Liberation Mono (or Cousine) | OFL 1.1 | as above | yes |
| Mac PostScript 35 (Helvetica, Times, Courier, Palatino, Bookman, Avant Garde, Symbol…) | URW base35 | AGPL-3 with font exception | checked (Debian copyright file) | yes |
| Book Antiqua / Century Gothic / Bookman Old Style | P052 / URW Gothic / URW Bookman (same package) | AGPL-3 with font exception | checked | approximate / yes / approximate — not measured |
| Verdana | DejaVu Sans | Bitstream Vera licence + public domain | checked | **no** — visual only |
| DOS / VGA / EGA / CGA ROM fonts, Windows "Terminal" | Ultimate Oldschool PC Font Pack (Debian `fonts-pc`) | CC BY-SA 4.0 | checked | exact cell sizes |

## Tier 2 — include, needs packaging work

| Stands in for | Font | Licence | Check | Notes |
|---|---|---|---|---|
| Georgia | Gelasio | OFL 1.1 | checked | metric-compatible, 4 styles |
| Comic Sans MS | Comic Relief | OFL 1.1 | checked | metric-compatible; Regular + Bold only |
| Impact | Anton | OFL 1.1 | checked | visual only |
| Arial Black | Archivo Black | OFL 1.1 | checked | visual only |
| Garamond | EB Garamond | OFL 1.1 | unchecked | visual only |
| Fixedsys | Fixedsys Excelsior | public domain / CC0, stated in README only | checked | pixel-outline TTF, correct at 16 px; shapes follow the original bitmap |
| Tahoma, MS Sans Serif, System, Small Fonts, Courier (bitmap), Marlett, Symbol | Wine fonts (Debian `fonts-wine`) | LGPL 2.1+, no embedding exception | checked | independently drawn; **family names are Microsoft's — must be renamed**; Wingdings is a 50-glyph stub |
| Chicago | ChicagoFLF | public domain | checked (name table: "You are free to use them as you wish", licence field "Public Domain") | no licence file; rename; a conflicting copyright string was reported on some aggregator copies |
| Monaco 9 | ProFont | MIT | checked | a descendant of the design, not a lookalike |
| London | Jacquard 12 / 24 | OFL 1.1 | checked | independent pixel blackletter, stand-in only |

## Tier 3 — doubtful, decide before using

| Stands in for | Font | Problem |
|---|---|---|
| MS Sans Serif 8pt | W95FA | OFL claimed on distributor page; zip not opened, no repo |
| Chicago | Sysfont | OFL per archived page only; upstream site and account are gone |
| Geneva 9 | Geneva 9.1 (FontStruct), FindersKeepers | OFL / unversioned CC BY; both are pixel-for-pixel copies of Apple's bitmap |
| London | London 20 (FontStruct) | OFL, but a recreation of the original bitmap |

## Gaps — nothing suitably licensed exists (candidates to draw here)

Mac: **New York**, **Geneva** (both bitmap and outline), **Monaco** (true lookalike), **Venice**,
**Athens**, **San Francisco**, **Toronto**, **Los Angeles**, **Cairo** (pictographs),
Seattle / Taliesin / Mobile, **Espy Sans**, **Charcoal**.

PC: **MS Serif / Tms Rmn**, **Trebuchet MS**, **Andale Mono**, **Lucida Console**,
**Lucida Sans Unicode**, **Wingdings / Webdings** (a drop-in could be built by remapping
Noto Sans Symbols 2, OFL — its coverage of those glyphs is unchecked), metric-compatible **Verdana**.

## Do not ship

- Urban Renewal (Kreative Korp city-font revivals) — no selling, no derivatives
- 98.css "Pixelated MS Sans Serif" — appears converted from Microsoft's bitmaps, no font licence
- jcs/classic-mac-fonts, nikdog/chicago-font, AUR monaco packages — Apple's actual font data
- Perfect DOS VGA 437, MonteCarlo — no licence
- Cairo Unicode, SaintFrancis, Virtue — freeware / non-commercial
- Symbola (current versions) — non-commercial
- ChiKareGo — unversioned CC BY and a self-described pixel copy

## Legal background (general, not legal advice)

In the US, typeface designs are not copyrightable (37 CFR 202.1(e); Eltra v. Ringer) but
font software is (Adobe v. Southern Software). Bitmap fonts are usually argued to fall on
the unprotected side, but that is untested for these faces and other jurisdictions differ.
Names are trademarks. Hence rules 2 and 3 above: redraw independently, rename.
