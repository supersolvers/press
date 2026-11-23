# Building and installing PRESS: the engine (libpress.so), its headers, the
# styles and their themes, and the fonts.
#
#     make                 build the library and the styles, for release
#     make styles          build the styles alone, beside the library built last,
#                          and put the themes (themes/) with them
#     make swift-sdk       stage a native SDK under target/ for local Swift builds
#     make test            the engine's tests, and Press's if Swift is here
#     make fonts           build the fonts from their sources (fonts/) and bundle them
#     sudo make install    put the library, headers, styles and fonts under PREFIX
#     sudo make uninstall  take them away again
#
# PREFIX is where things go (default /usr/local). The fonts go in
# PREFIX/share/fonts/press, where fontconfig finds them, and their settings
# in FONTCONF (fontconfig's conf.d), which other programs need to show their
# exact pixels. They are installed only if they have been built (make
# fonts): the library carries its own copy and does not need them. The styles go in PREFIX/lib/press/styles, where the library
# looks for them, and the themes in PREFIX/share/press/themes. LIBDIR
# overrides PREFIX/lib for the library, plugins, and pkg-config file.
#
# Press itself, the Swift library, is not installed: a Swift program builds
# it from this package (Package.swift) and links the library installed here.

PREFIX ?= /usr/local
LIBDIR ?= $(PREFIX)/lib
INSTALL_FONTS ?= auto
FONTCONF ?= /etc/fonts/conf.d
DESTDIR ?=
PROFILE ?= release
CC ?= cc
SWIFT ?= swift
FONT_PYTHON ?= fonts/.venv/bin/python
FAMILIES = Cicero-Regular Agate-Regular Agate-Bold Brevier-Regular Pica-Regular Elite-Regular Fleuron-Regular Vignette-Regular Cartouche-Regular
STYLES = eightyfive eightyeight eightynine ninety ninetyone ninetythree ninetyfive ninetyeight ninetynine twothousand twothousandone twothousandfour twothousandfive twothousandtwentyseven
OUT = target/$(PROFILE)
# The name programs ask for the library by: it ends in the interface's
# version (abi_version in idl/press.toml), so one made for another version
# is never mistaken for this one. The build gives the library this name.
ABI = $(shell sed -n 's/^abi_version *= *//p' idl/press.toml)
SONAME = libpress.so.$(ABI)
# PRESS's own version, for pkg-config (press.pc.in) to tell.
VERSION = $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml)

build:
	cargo build $(if $(filter release,$(PROFILE)),--release,)
	$(MAKE) styles PROFILE=$(PROFILE)

# Each style is a library of its own, written against include/press_style.h.
styles:
	mkdir -p $(OUT)
	for s in $(STYLES); do \
		$(CC) -shared -fPIC -O2 -Wall -Wextra -Iinclude -Istyles/common styles/$$s/$$s.c -lm -o $(OUT)/libpress_style_$$s.so || exit 1; \
	done
	mkdir -p $(OUT)/themes
	cp themes/*.toml $(OUT)/themes/

swift-sdk: build
	$(MAKE) install PROFILE=$(PROFILE) PREFIX="$(abspath $(OUT)/swift-sdk)" LIBDIR="$(abspath $(OUT)/swift-sdk)/lib" DESTDIR= INSTALL_FONTS=no

test:
	cargo test
	if command -v $(SWIFT) >/dev/null; then \
		$(MAKE) swift-sdk PROFILE=debug && \
		PKG_CONFIG_PATH="$(abspath target/debug/swift-sdk)/lib/pkgconfig$${PKG_CONFIG_PATH:+:$$PKG_CONFIG_PATH}" \
		LD_LIBRARY_PATH="$(abspath target/debug/swift-sdk)/lib$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH}" \
		$(SWIFT) test; \
	fi

# The font build needs Python with fontTools, skia-pathops, numpy and Pillow
# (fonts/requirements.txt); fonts-venv makes a place for them.
fonts-venv:
	python3 -m venv --system-site-packages fonts/.venv
	fonts/.venv/bin/pip install -r fonts/requirements.txt

fonts:
	$(FONT_PYTHON) fonts/build.py
	cargo run --release -p press-fontgen -- fonts/build assets/fonts

install:
	install -Dm644 $(OUT)/libpress.so $(DESTDIR)$(LIBDIR)/$(SONAME)
	ln -sf $(SONAME) $(DESTDIR)$(LIBDIR)/libpress.so
	mkdir -p $(DESTDIR)$(LIBDIR)/pkgconfig
	sed -e 's|@PREFIX@|$(PREFIX)|' -e 's|@LIBDIR@|$(patsubst $(PREFIX)/%,$${prefix}/%,$(LIBDIR))|' -e 's|@VERSION@|$(VERSION)|' -e 's|@ABI@|$(ABI)|' press.pc.in > $(DESTDIR)$(LIBDIR)/pkgconfig/press.pc
	chmod 644 $(DESTDIR)$(LIBDIR)/pkgconfig/press.pc
	install -Dm644 include/press.h $(DESTDIR)$(PREFIX)/include/press.h
	install -Dm644 include/press_style.h $(DESTDIR)$(PREFIX)/include/press_style.h
	for s in $(STYLES); do install -Dm644 $(OUT)/libpress_style_$$s.so $(DESTDIR)$(LIBDIR)/press/styles/libpress_style_$$s.so || exit 1; done
	for t in themes/*.toml; do install -Dm644 $$t $(DESTDIR)$(PREFIX)/share/press/themes/$$(basename $$t) || exit 1; done
	install -Dm644 COPYING $(DESTDIR)$(PREFIX)/share/doc/press/COPYING
ifneq ($(INSTALL_FONTS),no)
	if [ -e fonts/build/$(firstword $(FAMILIES)).ttf ]; then $(MAKE) install-fonts; \
	else echo "make: the fonts are not built, so they are not installed for other programs (make fonts-venv fonts); the library has its own copy and needs nothing more"; fi
endif

# The fonts as files, for other programs to set type in. The library does
# not need them: it carries its own copy (assets/fonts).
install-fonts:
	for f in $(FAMILIES); do install -Dm644 fonts/build/$$f.ttf $(DESTDIR)$(PREFIX)/share/fonts/press/$$f.ttf || exit 1; done
	install -Dm644 fonts/OFL.txt $(DESTDIR)$(PREFIX)/share/doc/press/fonts/OFL.txt
	install -Dm644 fonts/fontconfig/65-press.conf $(DESTDIR)$(FONTCONF)/65-press.conf
	if [ -z "$(DESTDIR)" ] && command -v fc-cache >/dev/null; then fc-cache -f $(PREFIX)/share/fonts/press; fi

uninstall:
	rm -f $(DESTDIR)$(LIBDIR)/libpress.so $(DESTDIR)$(LIBDIR)/libpress.so.* $(DESTDIR)$(LIBDIR)/pkgconfig/press.pc $(DESTDIR)$(PREFIX)/include/press.h $(DESTDIR)$(PREFIX)/include/press_style.h
	rm -rf $(DESTDIR)$(LIBDIR)/press/styles
	rmdir $(DESTDIR)$(LIBDIR)/press 2>/dev/null || true
	rm -f $(DESTDIR)$(PREFIX)/share/press/themes/*.toml
	rmdir $(DESTDIR)$(PREFIX)/share/press/themes $(DESTDIR)$(PREFIX)/share/press 2>/dev/null || true
	rm -rf $(DESTDIR)$(PREFIX)/share/fonts/press $(DESTDIR)$(PREFIX)/share/doc/press/fonts
	rm -f $(DESTDIR)$(PREFIX)/share/doc/press/COPYING
	rmdir $(DESTDIR)$(PREFIX)/share/doc/press 2>/dev/null || true
	rm -f $(DESTDIR)$(FONTCONF)/65-press.conf
	if [ -z "$(DESTDIR)" ] && command -v fc-cache >/dev/null; then fc-cache -f; fi

.PHONY: build styles swift-sdk test fonts fonts-venv install install-fonts uninstall
