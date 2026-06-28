"""Width table for the Geneva-style family: round and open letters one px wider
than the MS Sans-style skeleton. Letters with a centred stem or apex keep their
(odd) width so the stem stays on the pixel grid."""
from glyphs import GLYPHS

WIDER = ("abcdeghknopqsuz" "BCDEFGHKLNOPQRSUZ"
         "αβδεηθπρσςυζξ" "ΓΘΞΠΣΩ"
         "БВГДЗИКЛПЦЧЪЫЬЭЮЯЄҐбвгдзиклнпцчъыьэюяєґ"
         "ƏəɛƐɔƆŋŊþÞßðøØ"
         "ғҒқҚңҢөӨҳҲҷҶҙҘҫҪҡҠһҺ")

_done = set()
for _ch in WIDER:
    _inks = GLYPHS[_ch][1]
    if id(_inks) not in _done:          # aliases and accented forms share one entry
        _done.add(id(_inks))
        _inks["ge"] = _inks["ag"] + 1

for _ch in "mwMWшШ":                 # three stems: take the whole cell in a heavy monospaced family
    GLYPHS[_ch][1]["wide3"] = True

GLYPHS["m"][1]["ge"] = 9          # 3 px counters; odd width keeps the middle stem on the grid
