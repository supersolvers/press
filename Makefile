# Building and installing PRESS: the engine (libpress.so), its headers, the
# styles, and the fonts.
#
#     make                 build the library and the styles, for release
#     make styles          build the styles alone, beside the library built last
#     make test            the engine's tests, and Press's if Swift is here
#     make fonts           build the fonts from their sources (fonts/) and bundle them
#     sudo make install    put the library, headers, styles and fonts under PREFIX
#     sudo make uninstall  take them away again
#
# PREFIX is where things go (default /usr/local). The fonts go in
# PREFIX/share/fonts/press, where fontconfig finds them, and their settings
# in FONTCONF (fontconfig's conf.d), which other programs need to show their
# exact pixels. The styles go in PREFIX/lib/press/styles, where the library
# looks for them.
#
# Press itself, the Swift library, is not installed: a Swift program builds
# it from this package (Package.swift) and links the library installed here.

PREFIX ?= /usr/local
FONTCONF ?= /etc/fonts/conf.d
DESTDIR ?=
PROFILE ?= release
CC ?= cc
SWIFT ?= swift
FONT_PYTHON ?= fonts/.venv/bin/python
FAMILIES = Cicero-Regular Agate-Regular Agate-Bold Brevier-Regular Pica-Regular Elite-Regular Fleuron-Regular Vignette-Regular Cartouche-Regular
STYLES = eightyfive eightyeight eightynine ninety ninetyone ninetythree ninetyfive ninetyeight ninetynine twothousand twothousandone twothousandfour twothousandfive
OUT = target/$(PROFILE)

build:
	cargo build $(if $(filter release,$(PROFILE)),--release,)
	$(MAKE) styles PROFILE=$(PROFILE)

# Each style is a library of its own, written against include/press_style.h.
styles:
	mkdir -p $(OUT)
	for s in $(STYLES); do \
		$(CC) -shared -fPIC -O2 -Wall -Wextra -Iinclude -Istyles/common styles/$$s/$$s.c -lm -o $(OUT)/libpress_style_$$s.so || exit 1; \
	done

test:
	cargo test
	if command -v $(SWIFT) >/dev/null; then $(SWIFT) test; fi

# The font build needs Python with fontTools, skia-pathops, numpy and Pillow
# (fonts/requirements.txt); fonts-venv makes a place for them.
fonts-venv:
	python3 -m venv --system-site-packages fonts/.venv
	fonts/.venv/bin/pip install -r fonts/requirements.txt

fonts:
	$(FONT_PYTHON) fonts/build.py
	cargo run --release -p press-fontgen -- fonts/build assets/fonts

install:
	install -Dm644 $(OUT)/libpress.so $(DESTDIR)$(PREFIX)/lib/libpress.so
	install -Dm644 include/press.h $(DESTDIR)$(PREFIX)/include/press.h
	install -Dm644 include/press_style.h $(DESTDIR)$(PREFIX)/include/press_style.h
	for s in $(STYLES); do install -Dm644 $(OUT)/libpress_style_$$s.so $(DESTDIR)$(PREFIX)/lib/press/styles/libpress_style_$$s.so || exit 1; done
	for f in $(FAMILIES); do install -Dm644 fonts/build/$$f.ttf $(DESTDIR)$(PREFIX)/share/fonts/press/$$f.ttf || exit 1; done
	install -Dm644 fonts/OFL.txt $(DESTDIR)$(PREFIX)/share/doc/press/fonts/OFL.txt
	install -Dm644 fonts/fontconfig/65-press.conf $(DESTDIR)$(FONTCONF)/65-press.conf
	if [ -z "$(DESTDIR)" ] && command -v fc-cache >/dev/null; then fc-cache -f $(PREFIX)/share/fonts/press; fi

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/lib/libpress.so $(DESTDIR)$(PREFIX)/include/press.h $(DESTDIR)$(PREFIX)/include/press_style.h
	rm -rf $(DESTDIR)$(PREFIX)/lib/press/styles
	rmdir $(DESTDIR)$(PREFIX)/lib/press 2>/dev/null || true
	rm -rf $(DESTDIR)$(PREFIX)/share/fonts/press $(DESTDIR)$(PREFIX)/share/doc/press/fonts
	rmdir $(DESTDIR)$(PREFIX)/share/doc/press 2>/dev/null || true
	rm -f $(DESTDIR)$(FONTCONF)/65-press.conf
	if [ -z "$(DESTDIR)" ] && command -v fc-cache >/dev/null; then fc-cache -f; fi

.PHONY: build styles test fonts fonts-venv install uninstall
