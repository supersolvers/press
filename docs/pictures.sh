#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Draw again the pictures in docs/ that come straight from the examples:
# each look's own, the gallery in each look, the black and white ones of the
# controls, and the sheets of several windows side by side.
#
#     make && docs/pictures.sh
#
# It needs a C compiler, Swift, and ImageMagick (convert, montage). Nothing
# is shown on the screen. The pictures of programs on a desktop, under a
# pointer or at a screen's scale (content, fractional, pointers, stretch,
# styles-on-ode, text-drag, accents, rich-text) are taken by hand and are
# not drawn here.
set -eu

cd "$(dirname "$0")/.."
OUT=${PROFILE:-release}
LIB=$PWD/target/$OUT
[ -e "$LIB/libpress.so" ] || { echo "pictures: build the library first (make)" >&2; exit 1; }
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

export LD_LIBRARY_PATH="$LIB" PRESS_STYLE_PATH="$LIB" PRESS_BACKEND=headless
unset PRESS_SCALE PRESS_SNAPSHOT PRESS_STYLE || true

LIBRARIES="eightyfive eightyeight eightynine ninety ninetyone ninetythree ninetyfive ninetyeight ninetynine twothousand twothousandone twothousandfour twothousandfive twothousandtwentyseven"
BLUE='#587494'

for e in controls files hello notes scroll shuffle tools; do
    ${CC:-cc} examples/c/$e.c -Iinclude -L"$LIB" -lpress -lm -o "$WORK/$e"
done
# the same three at their own size, for the sheets of several windows
for e in files hello tools; do
    sed 's/^\( *CHECK(press_window_snapshot(.*\), 2));$/\1, 1));/' examples/c/$e.c > "$WORK/$e-small.c"
    ${CC:-cc} "$WORK/$e-small.c" -Iinclude -L"$LIB" -lpress -lm -o "$WORK/$e-small"
done
swift build --package-path examples/swift >/dev/null
GALLERY=$(swift build --package-path examples/swift --show-bin-path)/GalleryUI

# every example in every look, twice the size, as --snapshot draws them
for look in eightyfour $LIBRARIES themes/*.toml; do
    name=$(basename "$look" .toml)
    for e in controls files hello notes scroll shuffle tools; do
        PRESS_STYLE=$name "$WORK/$e" --snapshot "$WORK/$name-$e" >/dev/null
    done
    for e in files hello tools; do
        PRESS_STYLE=$name "$WORK/$e-small" --snapshot "$WORK/small-$name-$e" >/dev/null
    done
done
# a single picture has no number: give it its ending
for f in "$WORK"/*-controls "$WORK"/*-notes "$WORK"/*-scroll "$WORK"/*-shuffle "$WORK"/*-tools; do mv "$f" "$f.png"; done

# the looks, one by one: the files example, and menus where the README shows them
for look in $LIBRARIES; do
    cp "$WORK/$look-files-1.png" docs/$look.png
    PRESS_STYLE=$look PRESS_SNAPSHOT=docs/press-$look.png "$GALLERY" >/dev/null
done
PRESS_STYLE=eightyfour PRESS_SNAPSHOT=docs/press.png "$GALLERY" >/dev/null
for look in eightyfive ninetyone ninetyeight ninetynine twothousandone twothousandfive; do
    cp "$WORK/$look-hello-5.png" docs/$look-menus.png
done

# black and white, a control at a time
cp "$WORK/eightyfour-controls.png" docs/controls.png
cp "$WORK/eightyfour-notes.png" docs/notes.png
cp "$WORK/eightyfour-scroll.png" docs/scrollbars.png
cp "$WORK/eightyfour-shuffle.png" docs/shuffle.png
cp "$WORK/eightyfour-hello-4.png" docs/menus.png
cp "$WORK/eightyfour-hello-5.png" docs/submenu.png
cp "$WORK/eightyfour-hello-6.png" docs/context.png
cp "$WORK/eightyfour-hello-7.png" docs/popup.png

# side by side with a grey rule between
beside() {
    out=$1; shift
    montage "$@" -tile x1 -gravity north -geometry +5+0 -background '#808080' "$WORK/beside.png"
    convert "$WORK/beside.png" -shave 5x0 "$out"
}
beside docs/keyboard.png "$WORK/eightyfour-hello-1.png" "$WORK/eightyfour-hello-2.png" "$WORK/eightyfour-hello-3.png"
beside docs/files.png "$WORK/eightyfour-files-1.png" "$WORK/eightyfour-files-2.png"

# several windows on one ground, at their own size
sheet() {
    out=$1; ground=$2; across=$3; example=$4; shift 4
    set -- $(for look in "$@"; do echo "$WORK/small-$look-$example.png"; done)
    montage "$@" -tile "${across}x" -gravity northwest -geometry +6+6 -background "$ground" "$out"
}
sheet docs/frames.png "$BLUE" 3 hello-5 ninetyfive ninetynine twothousand twothousandone twothousandfive
sheet docs/frames-more.png "$BLUE" 2 hello-5 eightyfive eightynine ninety twothousandfour
sheet docs/frames-three.png "$BLUE" 3 hello-5 eightyeight ninetythree ninetyeight
sheet docs/tools.png "$BLUE" 3 tools eightyfour ninetyfive ninetynine twothousandone twothousandfour twothousandfive
sheet docs/themes.png white 3 files-1 ninetyfive-desert ninetynine-sage twothousandfive-graphite twothousandone-olive twothousandone-silver twothousandfour-slate

echo "pictures: drawn into docs/"
