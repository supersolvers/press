# Generated from idl/press.toml by press-idl. Do not edit.
# SPDX-License-Identifier: GPL-3.0-or-later
"""press: a one-bit user interface toolkit with a flat C ABI."""
import ctypes
import enum
import itertools
import os
import threading

ABI_VERSION = 1


def _load():
    here = os.path.dirname(os.path.abspath(__file__))
    built = [os.path.join(here, "..", "..", "target", profile, "libpress.so") for profile in ("release", "debug")]
    # of the builds in this checkout, the one built last is the one meant
    built = sorted((p for p in built if os.path.exists(p)), key=os.path.getmtime, reverse=True)
    candidates = [os.environ.get("PRESS_LIB")] + built + ["libpress.so"]
    for path in candidates:
        if path and (os.path.sep not in path or os.path.exists(path)):
            try:
                return ctypes.CDLL(path)
            except OSError:
                continue
    raise OSError("libpress.so not found; build it with 'cargo build' or set PRESS_LIB")


_lib = _load()
_DESTROY = ctypes.CFUNCTYPE(None, ctypes.c_void_p)
_CALLBACK = ctypes.CFUNCTYPE(None, ctypes.c_void_p)
_trampolines = {}
_posted = {}
_finished = []
_posted_lock = threading.Lock()
_post_ids = itertools.count(1)


def _work(fn):
    """Wrap a callable as (function, userdata, destroy) for the library to call later."""
    key = next(_post_ids)

    def run(_userdata):
        fn()

    def done(_userdata):
        # This runs inside one of the trampolines being let go, so they are
        # parked rather than freed here; the next call frees them.
        with _posted_lock:
            _finished.append(_posted.pop(key, None))

    entry = (_CALLBACK(run), _DESTROY(done))
    with _posted_lock:
        _finished.clear()
        _posted[key] = entry
    return entry[0], None, entry[1]


class PressError(Exception):
    def __init__(self, status):
        self.status = status
        super().__init__(status.name)


def _check(code):
    if code != 0:
        raise PressError(Status(code))


def _s(text):
    data = text.encode("utf-8")
    return data, len(data)


def _b(data):
    data = bytes(data)
    return data, len(data)


def _h(obj):
    return obj.handle if obj is not None else 0


class _Object:
    """Wraps a handle. Destroying the wrapper does not destroy the object."""

    def __init__(self, handle):
        self.handle = handle

    def _connect(self, name, connector, cfunc_type, fn, adapt):
        callback = cfunc_type(adapt(fn)) if fn is not None else cfunc_type()
        _check(connector(self.handle, callback, None, _DESTROY()))
        # The library holds a pointer into this trampoline, so it must outlive
        # the wrapper object, which the program is free to drop.
        _trampolines[(self.handle, name)] = callback


class Status(enum.IntEnum):
    """Result of every call. Zero is success."""
    OK = 0
    NOT_INITIALIZED = 1
    INVALID_HANDLE = 2
    WRONG_TYPE = 3
    INVALID_ARGUMENT = 4
    INVALID_STATE = 5
    BACKEND = 6
    IO = 7
    INTERNAL = 8


class Orientation(enum.IntEnum):
    """Direction in which a Stack lays out its children."""
    VERTICAL = 0
    HORIZONTAL = 1


class Align(enum.IntEnum):
    """Placement of children across a Stack's axis."""
    START = 0
    CENTER = 1
    END = 2
    FILL = 3


class Key(enum.IntEnum):
    """A key that does not type a character."""
    TAB = 1
    RETURN = 2
    ESCAPE = 3
    BACKSPACE = 4
    DELETE = 5
    LEFT = 6
    RIGHT = 7
    UP = 8
    DOWN = 9
    HOME = 10
    END = 11
    MENU = 12
    CONTEXT = 13
    PAGE_UP = 14
    PAGE_DOWN = 15
    INSERT = 16
    F1 = 21
    F2 = 22
    F3 = 23
    F4 = 24
    F5 = 25
    F6 = 26
    F7 = 27
    F8 = 28
    F9 = 29
    F10 = 30
    F11 = 31
    F12 = 32


class Modifier(enum.IntFlag):
    """Modifier keys held during a key press. Combine with bitwise OR."""
    SHIFT = 1
    CONTROL = 2
    ALT = 4


class Icon(enum.IntEnum):
    """A small picture shown before an item's label in a list."""
    NONE = 0
    FOLDER = 1
    DOCUMENT = 2
    APPLICATION = 3


class LineStyle(enum.IntEnum):
    """How a line is drawn on a Picture."""
    SOLID = 0
    DOTTED = 1
    DASHED = 2


class Typeface(enum.IntEnum):
    """One of the bundled typefaces, each drawn in pixels for the one size it is used at. Every one has Western, Greek, Cyrillic and Hebrew letters of its own and the Chinese, Japanese and Korean characters they all share."""
    CICERO = 0
    AGATE = 1
    AGATE_BOLD = 2
    BREVIER = 3
    PICA = 4
    ELITE = 5


class App:
    """Process-wide state. A process has one UI thread, the one that called init, and every press call must come from it, except post."""
    @staticmethod
    def init():
        """Start press and make the calling thread the UI thread. Safe to call again from that thread; refused from any other until it has shut down."""
        _lib.press_app_init.argtypes = []
        _check(_lib.press_app_init())

    @staticmethod
    def run():
        """Show the windows and process events until quit is called or the last window closes. With the headless backend this returns at once."""
        _lib.press_app_run.argtypes = []
        _check(_lib.press_app_run())

    @staticmethod
    def start():
        """For programs with an event loop of their own, in place of run: connect to the display without waiting. Then wait on the descriptor from fd and call dispatch whenever it is readable."""
        _lib.press_app_start.argtypes = []
        _check(_lib.press_app_start())

    @staticmethod
    def fd():
        """A file descriptor that becomes readable whenever dispatch has work: input, an animation or key repeat coming due, or a change made through this API that needs drawing. Only poll it; never read from it. Valid from start until stop."""
        _lib.press_app_fd.argtypes = [ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_app_fd(ctypes.byref(out)))
        return out.value

    @staticmethod
    def dispatch():
        """Do everything that is ready, without waiting: handle input, call handlers, redraw. Returns false once quit has been called or no window is left showing. Not allowed from inside a handler."""
        _lib.press_app_dispatch.argtypes = [ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_app_dispatch(ctypes.byref(out)))
        return out.value

    @staticmethod
    def stop():
        """Disconnect from the display after start. Objects survive and can be shown again with start."""
        _lib.press_app_stop.argtypes = []
        _check(_lib.press_app_stop())

    @staticmethod
    def post(work):
        """Hand over work to be done on the UI thread. This is the one function that may be called from any thread. The work is called from the next dispatch (or by run), in the order posted, and may call any press function. If this returns an error nothing was taken: neither the work nor its destroy notifier will be called."""
        _lib.press_app_post.argtypes = [_CALLBACK, ctypes.c_void_p, _DESTROY]
        _check(_lib.press_app_post(*_work(work)))

    @staticmethod
    def copy_data(kind, data):
        """Put something other than text on the clipboard: a picture, say. `kind` says what it is, as a media type such as image/png, and `data` is the thing itself, as the bytes of a file of that type. It takes the place of whatever was on the clipboard, and any program that understands the kind can paste it. Text fields look after copying text themselves."""
        _lib.press_app_copy_data.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_app_copy_data(*_s(kind), *_b(data)))

    @staticmethod
    def copy_also(kind, data):
        """Add another kind to what this program last copied, for programs that understand that one and not the other: a picture and, with it, its caption as text/plain;charset=utf-8, say. Whoever pastes takes the kind it likes best. A kind given twice keeps the later data. It may follow copy_data or text copied from a text field; if the clipboard has since passed to another program it starts afresh, as copy_data does."""
        _lib.press_app_copy_also.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_app_copy_also(*_s(kind), *_b(data)))

    @staticmethod
    def quit():
        """Ask run to return, or dispatch to report false."""
        _lib.press_app_quit.argtypes = []
        _check(_lib.press_app_quit())

    @staticmethod
    def shutdown():
        """Destroy every object and release all callbacks. Posted work that has not been done is dropped, and its destroy notifier is called."""
        _lib.press_app_shutdown.argtypes = []
        _check(_lib.press_app_shutdown())

    @staticmethod
    def abi_version():
        """The ABI version of the loaded library."""
        _lib.press_app_abi_version.argtypes = [ctypes.POINTER(ctypes.c_uint32)]
        out = ctypes.c_uint32()
        _check(_lib.press_app_abi_version(ctypes.byref(out)))
        return out.value

    @staticmethod
    def set_style(name):
        """Choose how the program looks: the name of a style (ninetyfive, say, looked for as libpress_style_ninetyfive.so where styles are kept), the path of a style's library, or eightyfour for black and white. A style with a frame of its own has the program's windows draw their title bars and edges themselves, and one that keeps its menus in the window keeps them there on any desktop. Call it after init and before making windows. It takes the place of the desktop's choice of style; PRESS_STYLE in the program's environment, which is the user saying otherwise for one run, takes the place of this. A style that cannot be had is refused with INVALID_ARGUMENT, and the program looks as it did."""
        _lib.press_app_set_style.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_app_set_style(*_s(name)))


class Widget(_Object):
    """Base of everything that can be placed in a window, menus included. Its functions accept any handle."""
    def destroy(self):
        """Destroy the widget and everything inside it. Its handle becomes invalid."""
        _lib.press_widget_destroy.argtypes = [ctypes.c_uint64]
        _check(_lib.press_widget_destroy(self.handle))

    def set_context_menu(self, menu):
        """Give the widget a context menu: it pops up when the widget, or anything inside it, is clicked with the right button, or with the context-menu key while it has the keyboard. A window can carry one for everything in it. The menu belongs to the widget from then on. A text field without one of its own pops up Cut, Copy, Paste and Select All."""
        _lib.press_widget_set_context_menu.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_widget_set_context_menu(self.handle, _h(menu)))

    def set_typeface(self, face):
        """Set the widget's text in another of the bundled typefaces. Labels, buttons, checkboxes, text fields, text areas and pictures have one; other widgets refuse it as the wrong type. A widget is the size its text needs in its typeface."""
        _lib.press_widget_set_typeface.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_widget_set_typeface(self.handle, int(face)))

    def set_min_size(self, width, height):
        """Say how small the widget may be, at the least, in pixels. It is still as large as its kind makes it where that is larger, so this can make a button taller but not narrower than its label needs. Nought either way leaves that way to the widget."""
        _lib.press_widget_set_min_size.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_widget_set_min_size(self.handle, width, height))

    def set_stretch(self, value):
        """Say whether the widget takes up spare room when its window is larger than its content needs. Without this it goes by kind: lists, tables, tree views, text areas, scroll views and tab views grow both ways; one-line text fields and progress bars grow across; sliders and scroll bars grow along their length; a stack grows if anything in it does; the rest keep their natural size. A widget told to grow does so along the stack it is in, so an empty label makes a spring that pushes its neighbours apart; one told not to keeps its natural size both ways."""
        _lib.press_widget_set_stretch.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_widget_set_stretch(self.handle, value))

    def focus(self):
        """Give the widget the keyboard. Only text fields, buttons and checkboxes that are enabled and in a window can take it."""
        _lib.press_widget_focus.argtypes = [ctypes.c_uint64]
        _check(_lib.press_widget_focus(self.handle))

    def set_enabled(self, value):
        """Disabled widgets are drawn greyed and ignore the pointer and the keyboard."""
        _lib.press_widget_set_enabled.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_widget_set_enabled(self.handle, value))

    def get_enabled(self):
        """Disabled widgets are drawn greyed and ignore the pointer and the keyboard."""
        _lib.press_widget_get_enabled.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_widget_get_enabled(self.handle, ctypes.byref(out)))
        return out.value

    def set_tooltip(self, value):
        """A word of help, shown beside the pointer when it has rested on the widget a moment, and put away when it moves on or is pressed. An empty string takes it away."""
        _lib.press_widget_set_tooltip.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_widget_set_tooltip(self.handle, *_s(value)))


class Window(Widget):
    """A top-level window. The desktop draws its title bar and frame."""
    def __init__(self, title, width, height):
        """Create a hidden window with the given content size in pixels."""
        _lib.press_window_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int32, ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_window_new(*_s(title), width, height, ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_content(self, content):
        """Place a widget in the window; it fills the content area."""
        _lib.press_window_set_content.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_window_set_content(self.handle, _h(content)))

    def set_menu_bar(self, bar):
        """Give the window a menu bar. It is drawn across the top and takes its height from the content area."""
        _lib.press_window_set_menu_bar.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_window_set_menu_bar(self.handle, _h(bar)))

    def fit(self):
        """Make the window the size its content asks for, with room for its menu bar, in place of the size it was created with. Call it once the content and the menu bar are in place. A window already showing is made that size, as set_size makes it."""
        _lib.press_window_fit.argtypes = [ctypes.c_uint64]
        _check(_lib.press_window_fit(self.handle))

    def set_size(self, width, height):
        """Give the window a new size, the size of its inside as new takes it, with room for its menu bar where that is in the window, as fit leaves it; even while it is showing: for a window that shows one thing and then another of another size. A window of one size stays of one size, the new one."""
        _lib.press_window_set_size.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_window_set_size(self.handle, width, height))

    def show(self):
        """Make the window visible."""
        _lib.press_window_show.argtypes = [ctypes.c_uint64]
        _check(_lib.press_window_show(self.handle))

    def focused(self):
        """The widget in the window that has the keyboard, or none (a handle of 0) when nothing has. A window that has not been typed into yet answers with its first text field, which is where typing will go."""
        _lib.press_window_focused.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_window_focused(self.handle, ctypes.byref(out)))
        return out.value

    def snapshot(self, path, scale):
        """Render the window content to a PNG file, magnified by an integer scale: one bit a pixel, or in colour when the window shows a photo. An open menu is included, and the picture grows to fit it. Works with every backend."""
        _lib.press_window_snapshot.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int32]
        _check(_lib.press_window_snapshot(self.handle, *_s(path), scale))

    def click_at(self, x, y):
        """Test hook: press and release the pointer at a point in the content area. An open menu is reached at the place it hangs, even outside the window."""
        _lib.press_window_click_at.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_window_click_at(self.handle, x, y))

    def hover_at(self, x, y, milliseconds):
        """Test hook: move the pointer to a point in the content area and leave it there for a number of milliseconds, as far as the toolkit's clock is concerned: long enough, and the word of help of what is under it shows."""
        _lib.press_window_hover_at.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_window_hover_at(self.handle, x, y, milliseconds))

    def drag_at(self, x0, y0, x1, y1):
        """Test hook: press the pointer at one point in the content area, move it to another in four steps, and let it go there."""
        _lib.press_window_drag_at.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_window_drag_at(self.handle, x0, y0, x1, y1))

    def popup_menu(self, menu, x, y):
        """Pop a menu up with its corner at a point in the content area. The menu may be free-standing or a context menu, not one in a menu bar or a submenu. Its items raise activated as usual."""
        _lib.press_window_popup_menu.argtypes = [ctypes.c_uint64, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_window_popup_menu(self.handle, _h(menu), x, y))

    def context_click_at(self, x, y):
        """Test hook: press the right button at a point in the content area."""
        _lib.press_window_context_click_at.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_window_context_click_at(self.handle, x, y))

    def side_click(self, forward):
        """Test hook: press the pointer's back button, or its forward button, over the window."""
        _lib.press_window_side_click.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_window_side_click(self.handle, forward))

    def hold_modifiers(self, modifiers):
        """Test hook: say which Modifier keys are held during the clicks that follow. Pass 0 to let them go."""
        _lib.press_window_hold_modifiers.argtypes = [ctypes.c_uint64, ctypes.c_uint32]
        _check(_lib.press_window_hold_modifiers(self.handle, modifiers))

    def press_key(self, key, modifiers):
        """Test hook: press a key in the window, with any combination of Modifier bits."""
        _lib.press_window_press_key.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_uint32]
        _check(_lib.press_window_press_key(self.handle, int(key), modifiers))

    def type_text(self, text):
        """Test hook: type text into the window as if from the keyboard."""
        _lib.press_window_type_text.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_window_type_text(self.handle, *_s(text)))

    def paste_data(self, kind):
        """Ask for what is on the clipboard as a given kind (a media type such as image/png). The answer arrives a moment later as data_pasted, with nothing in it if the clipboard holds nothing of that kind."""
        _lib.press_window_paste_data.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_window_paste_data(self.handle, *_s(kind)))

    def accept_drops(self, kind):
        """Say that things of a given kind (a media type such as image/png) may be dragged from other programs and dropped on the window. Call it once for each kind, the one liked best first. What is dropped arrives as data_dropped."""
        _lib.press_window_accept_drops.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_window_accept_drops(self.handle, *_s(kind)))

    def set_title(self, value):
        """Text shown in the title bar."""
        _lib.press_window_set_title.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_window_set_title(self.handle, *_s(value)))

    def set_resizable(self, value):
        """Whether the user may change the window's size. Windows can be resized unless this is turned off, which suits a small tool that is all of a piece. Set it before the window is first shown."""
        _lib.press_window_set_resizable.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_window_set_resizable(self.handle, value))

    def set_busy(self, value):
        """Whether the window is waiting on something the program is doing. While it is, the pointer over it is an hourglass; the window still answers the pointer and the keyboard."""
        _lib.press_window_set_busy.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_window_set_busy(self.handle, value))

    def set_tag(self, value):
        """A name by which the desktop can know this window again another day, so as to put it back where it was, at the size it was. It is not shown to anyone, so it is not translated, and it should be the same each time the program shows the same thing: a document's path, say, or "preferences". Set it before the window is first shown. Desktops that do not remember windows ignore it."""
        _lib.press_window_set_tag.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_window_set_tag(self.handle, *_s(value)))

    def set_full_screen(self, value):
        """Whether the window fills its screen, with no title bar or frame and in front of the menu bar, as a film or a slide show does. Turning it on asks the desktop, which may refuse; reading it says what the window last asked for or was told. While it is on the window draws no menu bar of its own, though its shortcuts still work. Turned off, the window goes back to where it was and its size. full_screen_changed says when the desktop changes it, as when the user leaves full screen by its own means."""
        _lib.press_window_set_full_screen.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_window_set_full_screen(self.handle, value))

    def get_full_screen(self):
        """Whether the window fills its screen, with no title bar or frame and in front of the menu bar, as a film or a slide show does. Turning it on asks the desktop, which may refuse; reading it says what the window last asked for or was told. While it is on the window draws no menu bar of its own, though its shortcuts still work. Turned off, the window goes back to where it was and its size. full_screen_changed says when the desktop changes it, as when the user leaves full screen by its own means."""
        _lib.press_window_get_full_screen.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_window_get_full_screen(self.handle, ctypes.byref(out)))
        return out.value

    def on_full_screen_changed(self, fn):
        """The desktop has put the window into full screen, or taken it out, whether the program asked or not: whether it is now. The handler receives (full_screen); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_bool, ctypes.c_void_p)
        connector = _lib.press_window_on_full_screen_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("full_screen_changed", connector, cfunc, fn, lambda f: lambda _this, full_screen, _userdata: f(full_screen))

    def on_files_dropped(self, fn):
        """Files were dragged from another program, or from a list whose items stand for files, and dropped on the window. The handler is given their paths, one per line, and whether the user asked for copies (by holding Control, on most desktops) rather than for the files to be moved. What is done with them is up to the handler. Without a handler the window does not accept files. The handler receives (paths, copy); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_bool, ctypes.c_void_p)
        connector = _lib.press_window_on_files_dropped
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("files_dropped", connector, cfunc, fn, lambda f: lambda _this, paths, paths_len, copy, _userdata: f(ctypes.string_at(paths, paths_len).decode("utf-8"), copy))

    def on_data_pasted(self, fn):
        """The answer to paste_data: the kind asked for, and what the clipboard held of that kind, which is nothing if it held none. The data is lent for the length of the call. The handler receives (kind, data); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_void_p)
        connector = _lib.press_window_on_data_pasted
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("data_pasted", connector, cfunc, fn, lambda f: lambda _this, kind, kind_len, data, data_len, _userdata: f(ctypes.string_at(kind, kind_len).decode("utf-8"), ctypes.string_at(data, data_len)))

    def on_data_dropped(self, fn):
        """Something of a kind named with accept_drops was dragged from another program and dropped on the window: its kind, the thing itself, and where in the window's content it was let go. The data is lent for the length of the call. The handler receives (kind, data, x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_window_on_data_dropped
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("data_dropped", connector, cfunc, fn, lambda f: lambda _this, kind, kind_len, data, data_len, x, y, _userdata: f(ctypes.string_at(kind, kind_len).decode("utf-8"), ctypes.string_at(data, data_len), x, y))

    def on_navigate(self, fn):
        """The pointer's back or forward button, on the side of many a mouse, was pressed over the window: forward says which. What it does is the program's to say; without a handler nothing happens. The handler receives (forward); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_bool, ctypes.c_void_p)
        connector = _lib.press_window_on_navigate
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("navigate", connector, cfunc, fn, lambda f: lambda _this, forward, _userdata: f(forward))

    def on_close_requested(self, fn):
        """The user asked to close the window. Without a handler the application quits. The handler receives (); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_void_p)
        connector = _lib.press_window_on_close_requested
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("close_requested", connector, cfunc, fn, lambda f: lambda _this, _userdata: f())


class Stack(Widget):
    """Lays out its children in a row or a column."""
    def __init__(self, orientation):
        """Create an empty stack."""
        _lib.press_stack_new.argtypes = [ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_stack_new(int(orientation), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, child):
        """Append a child. A widget can have only one parent."""
        _lib.press_stack_add.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_stack_add(self.handle, _h(child)))

    def insert(self, child, index):
        """Put a child at a place among the stack's children, counting from 0, with those after it moving along; a place past the end is the end. A child already in this stack is moved to the place, which is how children are put in another order."""
        _lib.press_stack_insert.argtypes = [ctypes.c_uint64, ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stack_insert(self.handle, _h(child), index))

    def set_backdrop(self, pattern):
        """Lay the stack, and what is in it, on a pattern of black and white: eight bytes, a row of an 8 by 8 tile each, the leftmost pixel in the top bit, a set bit black, as Picture.fill takes them. The tile is laid from the window's corner, so stacks side by side join without a seam. No bytes take it away."""
        _lib.press_stack_set_backdrop.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_stack_set_backdrop(self.handle, *_b(pattern)))

    def set_spacing(self, value):
        """Gap between children, in pixels."""
        _lib.press_stack_set_spacing.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stack_set_spacing(self.handle, value))

    def set_padding(self, value):
        """Margin around the children, in pixels."""
        _lib.press_stack_set_padding.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stack_set_padding(self.handle, value))

    def set_align(self, value):
        """Placement of children across the axis."""
        _lib.press_stack_set_align.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stack_set_align(self.handle, int(value)))


class Label(Widget):
    """A line of text."""
    def __init__(self, text):
        """Create a label."""
        _lib.press_label_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_label_new(*_s(text), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_text(self, value):
        """The text shown."""
        _lib.press_label_set_text.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_label_set_text(self.handle, *_s(value)))


class Button(Widget):
    """A push button."""
    def __init__(self, label):
        """Create a button."""
        _lib.press_button_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_button_new(*_s(label), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_image(self, png):
        """Give the button a picture, in colour, before its label: the bytes of a PNG file, up to 256 pixels each way, with see-through parts if it has them. A button with a picture and an empty label is the picture alone. No bytes take the picture away."""
        _lib.press_button_set_image.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_button_set_image(self.handle, *_b(png)))

    def set_label(self, value):
        """The button's caption."""
        _lib.press_button_set_label.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_button_set_label(self.handle, *_s(value)))

    def set_is_default(self, value):
        """Mark the default button of a dialog: it gets the heavy outline and answers the Return key."""
        _lib.press_button_set_is_default.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_button_set_is_default(self.handle, value))

    def set_is_cancel(self, value):
        """Mark the button that answers the Escape key."""
        _lib.press_button_set_is_cancel.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_button_set_is_cancel(self.handle, value))

    def set_flat(self, value):
        """Whether the button lies flat, with no edge of its own until the pointer is over it or down on it, and is no wider than its label needs: a tool in a toolbar."""
        _lib.press_button_set_flat.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_button_set_flat(self.handle, value))

    def set_down(self, value):
        """Whether the button stays in, drawn held down, as a tool that is on does: Bold in a word processor's bar, say. Clicking it still raises clicked, and the program says whether it is down."""
        _lib.press_button_set_down.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_button_set_down(self.handle, value))

    def set_key(self, value):
        """Whether the button is drawn as a key of a calculator: in the black and white style, a square face with a hard shadow below and to the right, as the first system's Calculator had them. A style without such keys draws an ordinary button."""
        _lib.press_button_set_key.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_button_set_key(self.handle, value))

    def on_clicked(self, fn):
        """The button was pressed, with the pointer or from the keyboard. The handler receives (); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_void_p)
        connector = _lib.press_button_on_clicked
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("clicked", connector, cfunc, fn, lambda f: lambda _this, _userdata: f())


class Checkbox(Widget):
    """A box that is either checked or not, with a caption."""
    def __init__(self, label):
        """Create an unchecked checkbox."""
        _lib.press_checkbox_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_checkbox_new(*_s(label), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_label(self, value):
        """The box's caption."""
        _lib.press_checkbox_set_label.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_checkbox_set_label(self.handle, *_s(value)))

    def set_checked(self, value):
        """Whether the box is checked."""
        _lib.press_checkbox_set_checked.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_checkbox_set_checked(self.handle, value))

    def get_checked(self):
        """Whether the box is checked."""
        _lib.press_checkbox_get_checked.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_checkbox_get_checked(self.handle, ctypes.byref(out)))
        return out.value

    def on_toggled(self, fn):
        """The user changed the box. The handler receives (checked); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_bool, ctypes.c_void_p)
        connector = _lib.press_checkbox_on_toggled
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("toggled", connector, cfunc, fn, lambda f: lambda _this, checked, _userdata: f(checked))


class TextField(Widget):
    """One line of editable text."""
    def __init__(self, text):
        """Create a text field holding the given text."""
        _lib.press_text_field_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_text_field_new(*_s(text), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def select_all(self):
        """Select all of the text."""
        _lib.press_text_field_select_all.argtypes = [ctypes.c_uint64]
        _check(_lib.press_text_field_select_all(self.handle))

    def select(self, from_, to):
        """Select from one place in the text to another, each a byte offset into its UTF-8: `to` is where the caret goes, and is scrolled into view, so a selection made backwards has `to` first. The same place twice puts the caret there with nothing selected. A place inside a character moves to its start, and one past the end to the end. Made by the program, it does not raise selected."""
        _lib.press_text_field_select.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_text_field_select(self.handle, from_, to))

    def selection_start(self):
        """Where the selection begins, as a byte offset into the text's UTF-8; with nothing selected, where the caret is."""
        _lib.press_text_field_selection_start.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_text_field_selection_start(self.handle, ctypes.byref(out)))
        return out.value

    def selection_end(self):
        """Where the selection ends, as a byte offset into the text's UTF-8; with nothing selected, where the caret is."""
        _lib.press_text_field_selection_end.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_text_field_selection_end(self.handle, ctypes.byref(out)))
        return out.value

    def copy(self):
        """Copy the selected text to the clipboard. With nothing selected the clipboard is left alone."""
        _lib.press_text_field_copy.argtypes = [ctypes.c_uint64]
        _check(_lib.press_text_field_copy(self.handle))

    def cut(self):
        """Copy the selected text to the clipboard and delete it. Raises changed."""
        _lib.press_text_field_cut.argtypes = [ctypes.c_uint64]
        _check(_lib.press_text_field_cut(self.handle))

    def paste(self):
        """Replace the selection with the clipboard's text. Raises changed; on a display the text may arrive a moment after this returns."""
        _lib.press_text_field_paste.argtypes = [ctypes.c_uint64]
        _check(_lib.press_text_field_paste(self.handle))

    def set_text(self, value):
        """The text in the field. Setting it does not raise changed."""
        _lib.press_text_field_set_text.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_text_field_set_text(self.handle, *_s(value)))

    def get_text(self):
        """The text in the field. Setting it does not raise changed."""
        _lib.press_text_field_get_text.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
        size = ctypes.c_size_t()
        _check(_lib.press_text_field_get_text(self.handle, None, 0, ctypes.byref(size)))
        buffer = ctypes.create_string_buffer(size.value + 1)
        _check(_lib.press_text_field_get_text(self.handle, buffer, len(buffer), ctypes.byref(size)))
        return buffer.raw[:min(size.value, len(buffer) - 1)].decode("utf-8")

    def set_placeholder(self, value):
        """Greyed text shown while the field is empty."""
        _lib.press_text_field_set_placeholder.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_text_field_set_placeholder(self.handle, *_s(value)))

    def set_min_width(self, value):
        """The narrowest the field may be laid out, in pixels."""
        _lib.press_text_field_set_min_width.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_text_field_set_min_width(self.handle, value))

    def set_align(self, value):
        """Where the text sits across the field: at the start, in the middle, or at the end, as figures do. In a text area each line sits so. Text too long for the field starts at the start and scrolls as ever. Fill is taken as Start."""
        _lib.press_text_field_set_align.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_text_field_set_align(self.handle, int(value)))

    def set_password(self, value):
        """Show a bullet in place of each character. A password cannot be copied, cut or dragged out, input methods are kept away from it, and assistive technology is not told what it holds. Not available on a text area."""
        _lib.press_text_field_set_password.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_text_field_set_password(self.handle, value))

    def on_changed(self, fn):
        """The user edited the text. The handler is given the text as it now stands. The handler receives (text); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_void_p)
        connector = _lib.press_text_field_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, text, text_len, _userdata: f(ctypes.string_at(text, text_len).decode("utf-8")))

    def on_submitted(self, fn):
        """The user pressed Return in the field. The handler is given the text. The default button, if any, is pressed as well. The handler receives (text); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_void_p)
        connector = _lib.press_text_field_on_submitted
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("submitted", connector, cfunc, fn, lambda f: lambda _this, text, text_len, _userdata: f(ctypes.string_at(text, text_len).decode("utf-8")))

    def on_selected(self, fn):
        """The user moved the caret or changed the selection: by typing, the keys, the pointer or a paste. The handler is given where the selection begins and ends, as byte offsets into the text's UTF-8, the start first; equal, they are the caret. It comes after changed when both happen. A password field does not raise it. The handler receives (start, end); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_text_field_on_selected
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("selected", connector, cfunc, fn, lambda f: lambda _this, start, end, _userdata: f(start, end))


class TextArea(TextField):
    """Several lines of editable text, wrapped at word gaps to the width of the area. It keeps tabs, each four spaces wide, and holds up to two megabytes, where a text field holds 64 kilobytes. It is a TextField: every TextField function and event works on it, except that Return starts a new line, so it never raises submitted and does not press the default button."""
    def __init__(self, text):
        """Create a text area holding the given text. Line breaks in the text are kept."""
        _lib.press_text_area_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_text_area_new(*_s(text), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_rows(self, value):
        """The height of the area, in lines of text. Longer text scrolls."""
        _lib.press_text_area_set_rows.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_text_area_set_rows(self.handle, value))

    def set_plain(self, value):
        """Draw it with no frame and no scroll bar, as writing on a page: for a note pad, say. Longer text still scrolls to keep the caret in view."""
        _lib.press_text_area_set_plain.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_text_area_set_plain(self.handle, value))


class RadioButton(Widget):
    """One of a group of choices, of which one at most is chosen. Radio buttons that share a parent (the same Stack) are a group. The arrow keys move through the group, choosing as they go."""
    def __init__(self, label):
        """Create an unchosen radio button."""
        _lib.press_radio_button_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_radio_button_new(*_s(label), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_label(self, value):
        """The button's caption."""
        _lib.press_radio_button_set_label.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_radio_button_set_label(self.handle, *_s(value)))

    def set_checked(self, value):
        """Whether this is the chosen one. Setting it true unchooses the others in its group. Setting it does not raise chosen."""
        _lib.press_radio_button_set_checked.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_radio_button_set_checked(self.handle, value))

    def get_checked(self):
        """Whether this is the chosen one. Setting it true unchooses the others in its group. Setting it does not raise chosen."""
        _lib.press_radio_button_get_checked.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_radio_button_get_checked(self.handle, ctypes.byref(out)))
        return out.value

    def on_chosen(self, fn):
        """The user chose this button. Choosing the one already chosen raises nothing. The handler receives (); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_void_p)
        connector = _lib.press_radio_button_on_chosen
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("chosen", connector, cfunc, fn, lambda f: lambda _this, _userdata: f())


class Slider(Widget):
    """A number picked by position: a knob that slides along a groove. Its value runs from 0 to max; a vertical slider has its largest value at the top."""
    def __init__(self, orientation):
        """Create a slider with a value of 0 and a max of 100."""
        _lib.press_slider_new.argtypes = [ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_slider_new(int(orientation), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_value(self, value):
        """Where the knob is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range."""
        _lib.press_slider_set_value.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_slider_set_value(self.handle, value))

    def get_value(self):
        """Where the knob is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range."""
        _lib.press_slider_get_value.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_slider_get_value(self.handle, ctypes.byref(out)))
        return out.value

    def set_max(self, value):
        """The largest value, at least 1."""
        _lib.press_slider_set_max.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_slider_set_max(self.handle, value))

    def get_max(self):
        """The largest value, at least 1."""
        _lib.press_slider_get_max.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_slider_get_max(self.handle, ctypes.byref(out)))
        return out.value

    def set_step(self, value):
        """How far an arrow key or a notch of the wheel moves the value."""
        _lib.press_slider_set_step.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_slider_set_step(self.handle, value))

    def set_length(self, value):
        """The length of the slider in pixels. It is always 16 pixels thick."""
        _lib.press_slider_set_length.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_slider_set_length(self.handle, value))

    def on_changed(self, fn):
        """The user moved the knob. The handler receives (value); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_slider_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, value, _userdata: f(value))


class Divider(Widget):
    """The bar between two panes of a Stack, dragged to give one more room and the other less, as between a file manager's folders and what is in the one open. Add it to a stack between two of its children. Until it is dragged the stack lays them out as it always does; then the child before it is as long as it was left, and the children after it share what remains. The pointer shows arrows over it."""
    def __init__(self, orientation):
        """Create a divider for a stack laid out this way: horizontal for panes side by side, vertical for panes one over the other."""
        _lib.press_divider_new.argtypes = [ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_divider_new(int(orientation), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_position(self, value):
        """How long the pane before the divider is, in pixels along the stack. Reading it before it has been set or dragged gives the length the pane was last laid out at. Setting a negative length gives the pane back to the stack to lay out as it would."""
        _lib.press_divider_set_position.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_divider_set_position(self.handle, value))

    def get_position(self):
        """How long the pane before the divider is, in pixels along the stack. Reading it before it has been set or dragged gives the length the pane was last laid out at. Setting a negative length gives the pane back to the stack to lay out as it would."""
        _lib.press_divider_get_position.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_divider_get_position(self.handle, ctypes.byref(out)))
        return out.value

    def on_moved(self, fn):
        """The user let go of the divider, having dragged it; position is where it was left. The handler receives (position); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_divider_on_moved
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("moved", connector, cfunc, fn, lambda f: lambda _this, position, _userdata: f(position))


class ListBox(Widget):
    """Rows of text, one of which may be selected. It scrolls when there are more rows than fit. The arrow keys move the selection, and typing a letter jumps to the next row that starts with it."""
    def __init__(self):
        """Create an empty list box, five rows high."""
        _lib.press_list_box_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_list_box_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, label):
        """Append a row."""
        _lib.press_list_box_add.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_add(self.handle, *_s(label)))

    def clear(self):
        """Remove every row."""
        _lib.press_list_box_clear.argtypes = [ctypes.c_uint64]
        _check(_lib.press_list_box_clear(self.handle))

    def is_selected(self, index):
        """Whether an item is selected. With multiple selection several may be."""
        _lib.press_list_box_is_selected.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_list_box_is_selected(self.handle, index, ctypes.byref(out)))
        return out.value

    def set_item_selected(self, index, selected):
        """Select or unselect one item, leaving the others as they are where several may be selected. Does not raise changed."""
        _lib.press_list_box_set_item_selected.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_bool]
        _check(_lib.press_list_box_set_item_selected(self.handle, index, selected))

    def set_icon(self, index, icon):
        """Give an item a small picture before its label. Once any item has one, every label moves over so that they line up."""
        _lib.press_list_box_set_icon.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_list_box_set_icon(self.handle, index, int(icon)))

    def set_file(self, index, path):
        """Say that an item stands for a file, by its full path; an empty path takes that away. Such an item can be dragged out of its list, even one whose rows cannot be rearranged, and dropped on another window or program, which is offered the file. A window with a files_dropped handler hears of it that way."""
        _lib.press_list_box_set_file.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_set_file(self.handle, index, *_s(path)))

    def set_drag_data(self, index, kind, data):
        """Give an item something to carry when it is dragged out of its list: `kind` says what it is, as a media type such as image/png, and `data` is the thing itself. Such an item can be dragged out even of a list whose rows cannot be rearranged, and a program that accepts the kind is given the data where it is dropped. An empty kind takes it away."""
        _lib.press_list_box_set_drag_data.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_set_drag_data(self.handle, index, *_s(kind), *_b(data)))

    def add_drag_data(self, index, kind, data):
        """Give an item another kind to carry when it is dragged out, besides what set_drag_data gave it: where it is dropped takes the kind it likes best. A kind given twice keeps the later data."""
        _lib.press_list_box_add_drag_data.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_add_drag_data(self.handle, index, *_s(kind), *_b(data)))

    def set_picture(self, index, art):
        """Give an item a picture of your own in place of a stock icon. The picture is drawn in text, one line for each row of pixels: X or # is black, a full stop is white, and anything else lets the row show through. Up to 14 pixels each way. An empty string takes the picture away."""
        _lib.press_list_box_set_picture.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_set_picture(self.handle, index, *_s(art)))

    def set_picture_file(self, index, path):
        """Give an item a picture read from a PNG file of up to 14 pixels each way. It is reduced to one bit: dark pixels become black, light ones white, and transparent ones let the row show through."""
        _lib.press_list_box_set_picture_file.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_set_picture_file(self.handle, index, *_s(path)))

    def remove(self, index):
        """Remove an item. Items after it move down by one index. In a tree view, whatever hangs from it goes too. Does not raise removed."""
        _lib.press_list_box_remove.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_list_box_remove(self.handle, index))

    def move_item(self, index, position):
        """Move an item to a place among the rows as shown, counting from 0. A sorted table stops being sorted. Does not raise moved."""
        _lib.press_list_box_move_item.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_list_box_move_item(self.handle, index, position))

    def position_of(self, index):
        """Where an item is among the rows as shown, counting from 0; -1 if it is folded away in a tree."""
        _lib.press_list_box_position_of.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_list_box_position_of(self.handle, index, ctypes.byref(out)))
        return out.value

    def edit(self, index, column):
        """Start editing an item's text in place, whether or not the list is editable. In a table, column says which cell; elsewhere pass 0."""
        _lib.press_list_box_edit.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_list_box_edit(self.handle, index, column))

    def set_image(self, index, png):
        """Give an item a picture in colour, in place of its icon: the bytes of a PNG file, up to 16 pixels each way, with see-through parts if it has them. No bytes take the picture away."""
        _lib.press_list_box_set_image.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_set_image(self.handle, index, *_b(png)))

    def set_large_image(self, index, png):
        """Give an item a large picture in colour, for an icon view: the bytes of a PNG file, up to 32 pixels each way, with see-through parts if it has them. An icon view shows it in place of the item's small picture drawn double size; picked out, every other pixel of it takes the colour the style selects in. No bytes take the picture away."""
        _lib.press_list_box_set_large_image.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_list_box_set_large_image(self.handle, index, *_b(png)))

    def set_selected(self, value):
        """The index of the selected row, counting from 0; -1 when none is. Setting it scrolls the row into view and does not raise changed."""
        _lib.press_list_box_set_selected.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_list_box_set_selected(self.handle, value))

    def get_selected(self):
        """The index of the selected row, counting from 0; -1 when none is. Setting it scrolls the row into view and does not raise changed."""
        _lib.press_list_box_get_selected.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_list_box_get_selected(self.handle, ctypes.byref(out)))
        return out.value

    def set_multiple(self, value):
        """Let several items be selected at once: Control with a click adds or removes one, Shift with a click or an arrow stretches the selection, a drag stretches it, and Control+A takes everything. selected then gives the item last selected."""
        _lib.press_list_box_set_multiple.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_list_box_set_multiple(self.handle, value))

    def set_editable(self, value):
        """Let the user edit an item's text in place. Return then edits the current item, as renaming a file does, where it would otherwise open it; a double click still opens it."""
        _lib.press_list_box_set_editable.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_list_box_set_editable(self.handle, value))

    def set_accepts_rows(self, value):
        """Let rows dragged out of another list be dropped here: from the same window, another window of the program, or a list in another press program. The rows leave that list, which must be reorderable, and join this one, keeping their text and pictures. A tree view can take rows too: dropped onto an item they hang from it, dropped between two they go there. An item dragged out of a tree takes its whole branch: into another tree the branch keeps its shape, into a plain list it is laid out flat."""
        _lib.press_list_box_set_accepts_rows.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_list_box_set_accepts_rows(self.handle, value))

    def set_reorderable(self, value):
        """Let the user drag a row to a new place: press it and pull it up or down, and a line shows where it will land. Where several rows are selected, pulling one moves them all. Items keep their indices; only the order they are shown in changes. In a tree view an item can also be dropped onto another, to hang from it, and takes along whatever hangs from it. Rows dragged onto something that takes text, here or in another program, arrive as their labels."""
        _lib.press_list_box_set_reorderable.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_list_box_set_reorderable(self.handle, value))

    def set_flush(self, value):
        """Draw it with no frame of its own, to fill a window to its edges as the file windows of the original did: a double rule along its top, and its scroll bars lying on the window's edges, leaving the corner where they meet to the window's size box."""
        _lib.press_list_box_set_flush.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_list_box_set_flush(self.handle, value))

    def set_rows(self, value):
        """The height of the list, in rows."""
        _lib.press_list_box_set_rows.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_list_box_set_rows(self.handle, value))

    def set_min_width(self, value):
        """The narrowest the list may be laid out, scroll bar included, in pixels."""
        _lib.press_list_box_set_min_width.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_list_box_set_min_width(self.handle, value))

    def on_changed(self, fn):
        """The user selected a different row. The handler receives (index); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_list_box_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, index, _userdata: f(index))

    def on_activated(self, fn):
        """The user opened a row: a double click, or Return. The default button, if any, is pressed as well after Return. The handler receives (index); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_list_box_on_activated
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("activated", connector, cfunc, fn, lambda f: lambda _this, index, _userdata: f(index))

    def on_moved(self, fn):
        """The user dragged an item to a new place. index is the item, unchanged; position is where it now is among the rows. The handler receives (index, position); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_list_box_on_moved
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("moved", connector, cfunc, fn, lambda f: lambda _this, index, position, _userdata: f(index, position))

    def on_removed(self, fn):
        """The user dragged an item out of this list into another. index is the index it had; items after it have moved down by one. When several leave together the highest index is reported first, so each index is right as it is reported. The handler receives (index); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_list_box_on_removed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("removed", connector, cfunc, fn, lambda f: lambda _this, index, _userdata: f(index))

    def on_received(self, fn):
        """The user dropped an item here from another list. index is its index in this list, which is the last; position is where it is among the rows. The handler receives (index, position); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_list_box_on_received
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("received", connector, cfunc, fn, lambda f: lambda _this, index, position, _userdata: f(index, position))

    def on_edited(self, fn):
        """The user edited an item's text in place and kept the change. The list already holds the new text; column is 0 except in a table. The handler receives (index, column, text); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_void_p)
        connector = _lib.press_list_box_on_edited
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("edited", connector, cfunc, fn, lambda f: lambda _this, index, column, text, text_len, _userdata: f(index, column, ctypes.string_at(text, text_len).decode("utf-8")))


class Table(ListBox):
    """A list box with columns under a row of headings. It is a ListBox: rows are added with ListBox.add, with the cells of a row separated by tab characters, and every ListBox function and event works on it."""
    def __init__(self):
        """Create an empty table with no columns."""
        _lib.press_table_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_table_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add_column(self, title, width):
        """Add a column: its heading and its width in pixels. A cell too long for its column is cut short. The user can drag the edge between two headings to change a width."""
        _lib.press_table_add_column.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int32]
        _check(_lib.press_table_add_column(self.handle, *_s(title), width))

    def sort_by(self, column, ascending):
        """Sort by a column, smallest first or largest first, or with -1 show the rows in the order they were added."""
        _lib.press_table_sort_by.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_bool]
        _check(_lib.press_table_sort_by(self.handle, column, ascending))

    def mark_order(self, column):
        """Show a column's heading as the one the rows are in order of, without sorting them, for a program that puts its rows in order itself; -1 shows none."""
        _lib.press_table_mark_order.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_table_mark_order(self.handle, column))

    def set_column_align(self, column, align):
        """Where a column's cells and its heading sit in it: at the start, in the middle, or at the end, as numbers are. Fill is taken as Start."""
        _lib.press_table_set_column_align.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_table_set_column_align(self.handle, column, int(align)))

    def set_column_typeface(self, column, face):
        """The typeface a column's cells are written in. Its heading keeps the interface's."""
        _lib.press_table_set_column_typeface.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_table_set_column_typeface(self.handle, column, int(face)))

    def set_sortable(self, value):
        """Let the user sort the rows by clicking a column's heading; a second click turns the order round. Cells that start with numbers are ordered as numbers. Sorting changes the order rows are shown in, never their indices."""
        _lib.press_table_set_sortable.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_table_set_sortable(self.handle, value))


class IconView(ListBox):
    """A list box that shows its items as large pictures with their names beneath, in rows that fill its width, as a folder's window does. It is a ListBox: every ListBox function and event works on it. An item's picture goes by its icon (a folder, a document or an application), or is the program's own picture drawn double size. In a label with tabs only what comes before the first is shown. The arrow keys go from icon to icon. With set_reorderable on, the user can put icons where they like."""
    def __init__(self):
        """Create an empty icon view, three rows of icons high."""
        _lib.press_icon_view_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_icon_view_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_position(self, index, x, y):
        """Put an item's icon at a place in the view, measured in pixels from the top left of all the icons (not of the view, once it has been scrolled). A negative place puts it back in line with the rest. Clearing the view forgets every place."""
        _lib.press_icon_view_set_position.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_icon_view_set_position(self.handle, index, x, y))

    def on_placed(self, fn):
        """The user dragged an icon to a new place in the view, which they can do once ListBox.set_reorderable has been turned on for it. index is the item; x and y are where it now is, as for set_position. Several selected icons dragged together are each reported. The order of the items does not change. The handler receives (index, x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_icon_view_on_placed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("placed", connector, cfunc, fn, lambda f: lambda _this, index, x, y, _userdata: f(index, x, y))


class TreeView(ListBox):
    """A list box whose rows hang from one another and can be folded away. It is a ListBox: an item is known by its index, in the order items were added, and every ListBox function and event works on it. ListBox.add adds an item at the top level. Right and Left unfold and fold from the keyboard."""
    def __init__(self):
        """Create an empty tree view."""
        _lib.press_tree_view_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_tree_view_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add_child(self, parent, label):
        """Add an item under another, given by its index, or at the top level with -1. Returns the new item's index. Items start folded."""
        _lib.press_tree_view_add_child.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_tree_view_add_child(self.handle, parent, *_s(label), ctypes.byref(out)))
        return out.value

    def parent_of(self, index):
        """The index of the item an item hangs from, or -1 at the top level. It changes when the user drags the item elsewhere in a reorderable tree."""
        _lib.press_tree_view_parent_of.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_tree_view_parent_of(self.handle, index, ctypes.byref(out)))
        return out.value

    def set_expanded(self, index, expanded):
        """Unfold or fold an item to show or hide what hangs from it."""
        _lib.press_tree_view_set_expanded.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_bool]
        _check(_lib.press_tree_view_set_expanded(self.handle, index, expanded))

    def set_foldable(self, index, foldable):
        """Have an item show a fold mark even while nothing hangs from it, so that what does can be added when the user unfolds it: a folder whose folders are read only when it is opened."""
        _lib.press_tree_view_set_foldable.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_bool]
        _check(_lib.press_tree_view_set_foldable(self.handle, index, foldable))

    def on_expanded(self, fn):
        """The user unfolded or folded an item, by its mark or from the keyboard. Items added under it now show at once. The handler receives (index, expanded); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_bool, ctypes.c_void_p)
        connector = _lib.press_tree_view_on_expanded
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("expanded", connector, cfunc, fn, lambda f: lambda _this, index, expanded, _userdata: f(index, expanded))


class ProgressBar(Widget):
    """Shows how far along some work is. It fills from the left as value goes from 0 to max."""
    def __init__(self):
        """Create a progress bar with a value of 0 and a max of 100."""
        _lib.press_progress_bar_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_progress_bar_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_value(self, value):
        """How much is done, from 0 to max. A value out of range is brought into range."""
        _lib.press_progress_bar_set_value.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_progress_bar_set_value(self.handle, value))

    def get_value(self):
        """How much is done, from 0 to max. A value out of range is brought into range."""
        _lib.press_progress_bar_get_value.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_progress_bar_get_value(self.handle, ctypes.byref(out)))
        return out.value

    def set_max(self, value):
        """The value that means finished, at least 1."""
        _lib.press_progress_bar_set_max.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_progress_bar_set_max(self.handle, value))

    def get_max(self):
        """The value that means finished, at least 1."""
        _lib.press_progress_bar_get_max.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_progress_bar_get_max(self.handle, ctypes.byref(out)))
        return out.value

    def set_busy(self, value):
        """Show moving stripes in place of an amount, for work whose length is not known."""
        _lib.press_progress_bar_set_busy.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_progress_bar_set_busy(self.handle, value))

    def set_length(self, value):
        """The length of the bar in pixels."""
        _lib.press_progress_bar_set_length.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_progress_bar_set_length(self.handle, value))


class Picture(Widget):
    """A white box of a size the program chooses, for the program to draw on with lines, boxes and words: a graph, a chart, a diagram. What is drawn is kept, and drawn again whenever the window is, so draw once and again only when something changes. Places are in the picture's own pixels, from its top left corner; what falls outside is not shown. The pointer pressed, dragged and let go in it, and the wheel turned over it, are told with the place. Words are drawn in the picture's typeface, which set_typeface changes."""
    def __init__(self, width, height):
        """Create a blank picture of the given size in pixels, from 1 to 4096 each way."""
        _lib.press_picture_new.argtypes = [ctypes.c_int32, ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_picture_new(width, height, ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def clear(self):
        """Rub out everything drawn, leaving it white."""
        _lib.press_picture_clear.argtypes = [ctypes.c_uint64]
        _check(_lib.press_picture_clear(self.handle))

    def line(self, x0, y0, x1, y1, style):
        """Draw a line one pixel thick from one point to another, both ends included."""
        _lib.press_picture_line.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_picture_line(self.handle, x0, y0, x1, y1, int(style)))

    def box(self, x, y, width, height, filled):
        """Draw a box with its top left corner at a point: an outline one pixel thick, or filled in black."""
        _lib.press_picture_box.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_bool]
        _check(_lib.press_picture_box(self.handle, x, y, width, height, filled))

    def fill(self, x, y, width, height, pattern):
        """Fill a box with a pattern of black and white: `pattern` is eight bytes, a row of an 8 by 8 tile each, the leftmost pixel in the top bit, a set bit black and a clear one white. It covers what was there, so eight noughts rub the box out and eight 255s black it in; 170 and 85 by turns are a half-tone grey. The tile is laid from the picture's corner, so boxes side by side in one pattern join without a seam. Lines, outlines and words are always drawn over patterns, whichever was asked for first."""
        _lib.press_picture_fill.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_picture_fill(self.handle, x, y, width, height, *_b(pattern)))

    def image(self, x, y, width, height, bits):
        """Lay down a picture of your own making, a pixel a bit, with its top left corner at a point: `bits` is rows of `width` pixels, each row padded to whole bytes, the leftmost pixel in the top bit of a byte, set for black. It covers what was there, as fill does. For a scene drawn pixel by pixel, such as shading or a game's world, this is one call where dots would be thousands."""
        _lib.press_picture_image.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_picture_image(self.handle, x, y, width, height, *_b(bits)))

    def set_color(self, rgb):
        """Draw what follows in a colour, 0xRRGGBB: lines, boxes outlined and filled, and words. A picture draws in black until told otherwise, and again once it is cleared."""
        _lib.press_picture_set_color.argtypes = [ctypes.c_uint64, ctypes.c_uint32]
        _check(_lib.press_picture_set_color(self.handle, rgb))

    def fill_color(self, x, y, width, height, rgb):
        """Fill a box with one colour, 0xRRGGBB. It covers what was there, as fill does; lines, boxes and words lie on it wherever in the order they were drawn."""
        _lib.press_picture_fill_color.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_uint32]
        _check(_lib.press_picture_fill_color(self.handle, x, y, width, height, rgb))

    def photo(self, x, y, width, height, rgb):
        """Lay down a picture in colour with its top left corner at a point: `rgb` is rows of `width` pixels, three bytes each, red, green and blue. press is black and white, and this is its one exception, for what is a photograph by nature, such as a camera's picture; offer it in black and white too. It covers what was there, as image does."""
        _lib.press_picture_photo.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_picture_photo(self.handle, x, y, width, height, *_b(rgb)))

    def text(self, x, y, text):
        """Draw words in black with the left end of their line at x and its top at y."""
        _lib.press_picture_text.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_picture_text(self.handle, x, y, *_s(text)))

    def text_width(self, text):
        """How wide words would be drawn, in pixels, so that they can be placed: centred under a mark, say, or kept clear of an edge."""
        _lib.press_picture_text_width.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_picture_text_width(self.handle, *_s(text), ctypes.byref(out)))
        return out.value

    def line_height(self):
        """How tall a line of words is, in pixels."""
        _lib.press_picture_line_height.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_picture_line_height(self.handle, ctypes.byref(out)))
        return out.value

    def text_in(self, x, y, text, face):
        """Draw words in black in a typeface of their own, whatever the picture's, with the left end of their line at x and its top at y: a heading over plain words, say, or program text in an even face."""
        _lib.press_picture_text_in.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int32]
        _check(_lib.press_picture_text_in(self.handle, x, y, *_s(text), int(face)))

    def text_width_in(self, text, face):
        """How wide words would be drawn in a given typeface, in pixels."""
        _lib.press_picture_text_width_in.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int32, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_picture_text_width_in(self.handle, *_s(text), int(face), ctypes.byref(out)))
        return out.value

    def line_height_in(self, face):
        """How tall a line of words is in a given typeface, in pixels."""
        _lib.press_picture_line_height_in.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_picture_line_height_in(self.handle, int(face), ctypes.byref(out)))
        return out.value

    def set_size(self, width, height):
        """Change the picture's size, from 1 to 4096 pixels each way. What is drawn is kept; what falls outside is not shown."""
        _lib.press_picture_set_size.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_picture_set_size(self.handle, width, height))

    def set_description(self, value):
        """What the picture shows, in words, for those who cannot see it: "a graph of y = sin x from -10 to 10", say. Screen readers read it out."""
        _lib.press_picture_set_description.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_picture_set_description(self.handle, *_s(value)))

    def set_takes_keys(self, value):
        """Let the picture have the keyboard, for a game or a drawing worked by keys: Tab stops at it, a press in it takes the keyboard there, and it is ringed while it has it. The arrow keys and their kin are then told by key_pressed, and what is typed by typed."""
        _lib.press_picture_set_takes_keys.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_picture_set_takes_keys(self.handle, value))

    def set_raw_keys(self, value):
        """Let the picture hear every key, for a terminal or an emulator of another machine: it takes the keyboard as takes_keys does, and each key is told by key_down with the modifiers held, Tab, Return, Escape, F1 to F12, Page Up and Down, Insert, and Control or Alt with a character among them. Control and Shift with a character still choose a menu entry with that shortcut. What is typed without Control or Alt still comes by typed, and key_pressed is not raised."""
        _lib.press_picture_set_raw_keys.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_picture_set_raw_keys(self.handle, value))

    def set_right_button(self, value):
        """Let the picture hear the right button, for a remote desktop or a game: pressed in the picture, it raises right_pressed, and no context menu pops up there. Where the pointer goes while it is held is told by dragged, and letting it go by right_released."""
        _lib.press_picture_set_right_button.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_picture_set_right_button(self.handle, value))

    def set_middle_button(self, value):
        """Let the picture hear the middle button, as right_button does the right: middle_pressed, where the pointer goes while it is held by dragged, and middle_released."""
        _lib.press_picture_set_middle_button.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_picture_set_middle_button(self.handle, value))

    def on_right_pressed(self, fn):
        """The right button was pressed in a picture that hears it (right_button): where, in the picture's pixels. Until it is let go the picture hears where the pointer goes, by dragged, even outside. The handler receives (x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_right_pressed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("right_pressed", connector, cfunc, fn, lambda f: lambda _this, x, y, _userdata: f(x, y))

    def on_right_released(self, fn):
        """The right button, pressed in the picture, was let go: where, in the picture's pixels, which may be outside it. The handler receives (x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_right_released
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("right_released", connector, cfunc, fn, lambda f: lambda _this, x, y, _userdata: f(x, y))

    def on_middle_pressed(self, fn):
        """The middle button was pressed in a picture that hears it (middle_button): where, in the picture's pixels. Until it is let go the picture hears where the pointer goes, by dragged, even outside. The handler receives (x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_middle_pressed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("middle_pressed", connector, cfunc, fn, lambda f: lambda _this, x, y, _userdata: f(x, y))

    def on_middle_released(self, fn):
        """The middle button, pressed in the picture, was let go: where, in the picture's pixels, which may be outside it. The handler receives (x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_middle_released
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("middle_released", connector, cfunc, fn, lambda f: lambda _this, x, y, _userdata: f(x, y))

    def on_key_down(self, fn):
        """A key was pressed while a picture that hears every key had the keyboard: its Key value, or 0 for a character key; the character, as a Unicode code point, when Control or Alt was held with one (Control gives it in lower case), and otherwise 0; and the Modifier bits held. The handler receives (key, character, modifiers); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_key_down
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("key_down", connector, cfunc, fn, lambda f: lambda _this, key, character, modifiers, _userdata: f(key, character, modifiers))

    def on_resized(self, fn):
        """A picture told to stretch (with set_stretch) was given room of another size by its window, the first time it is laid out included: the width and height it now fills, in pixels. Draw it again for that size; its own size, from new or set_size, is the least it asks for. The handler receives (width, height); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_resized
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("resized", connector, cfunc, fn, lambda f: lambda _this, width, height, _userdata: f(width, height))

    def on_key_pressed(self, fn):
        """A key that types nothing was pressed while the picture had the keyboard: an arrow, Home, End, Backspace or Delete, as its Key value. Tab, Return and Escape keep their usual work. The handler receives (key); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_key_pressed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("key_pressed", connector, cfunc, fn, lambda f: lambda _this, key, _userdata: f(key))

    def on_typed(self, fn):
        """Something was typed while the picture had the keyboard: a letter, a figure, a space, or whatever an input method made of the keys. The handler receives (text); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char), ctypes.c_size_t, ctypes.c_void_p)
        connector = _lib.press_picture_on_typed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("typed", connector, cfunc, fn, lambda f: lambda _this, text, text_len, _userdata: f(ctypes.string_at(text, text_len).decode("utf-8")))

    def on_pressed(self, fn):
        """The pointer was pressed in the picture: where, and 1 for a single click, 2 for the second of a double click, and so on. Until it is let go the picture hears where it moves, even outside. The handler receives (x, y, clicks); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_pressed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("pressed", connector, cfunc, fn, lambda f: lambda _this, x, y, clicks, _userdata: f(x, y, clicks))

    def on_dragged(self, fn):
        """The pointer moved while held after a press in the picture. The place may be outside it. The handler receives (x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_dragged
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("dragged", connector, cfunc, fn, lambda f: lambda _this, x, y, _userdata: f(x, y))

    def on_released(self, fn):
        """The pointer pressed in the picture was let go. The handler receives (x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_released
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("released", connector, cfunc, fn, lambda f: lambda _this, x, y, _userdata: f(x, y))

    def on_scrolled(self, fn):
        """The wheel was turned over the picture: lines down, or up when negative (three to a notch of an ordinary wheel), and where the pointer was. The handler receives (lines, x, y); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_picture_on_scrolled
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("scrolled", connector, cfunc, fn, lambda f: lambda _this, lines, x, y, _userdata: f(lines, x, y))


class TabView(Widget):
    """Several pages of widgets in the space of one, with a row of tabs along the top to choose between them. Left and Right turn the pages from the keyboard."""
    def __init__(self):
        """Create a tab view with no pages."""
        _lib.press_tab_view_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_tab_view_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, title, content):
        """Add a page: the word on its tab, and the widget (usually a Stack) shown when it is chosen."""
        _lib.press_tab_view_add.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_uint64]
        _check(_lib.press_tab_view_add(self.handle, *_s(title), _h(content)))

    def set_selected(self, value):
        """The index of the page showing; -1 when there are no pages. Setting it does not raise changed."""
        _lib.press_tab_view_set_selected.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_tab_view_set_selected(self.handle, value))

    def get_selected(self):
        """The index of the page showing; -1 when there are no pages. Setting it does not raise changed."""
        _lib.press_tab_view_get_selected.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_tab_view_get_selected(self.handle, ctypes.byref(out)))
        return out.value

    def on_changed(self, fn):
        """The user turned to a different page. The handler receives (index); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_tab_view_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, index, _userdata: f(index))


class PopupButton(Widget):
    """A box showing one of a list of choices. Pressing it pops the whole list up over it, with the current choice under the pointer: drag to another and let go, or click and then click. From the keyboard, Space pops the list up."""
    def __init__(self):
        """Create a pop-up button with no choices."""
        _lib.press_popup_button_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_popup_button_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, label):
        """Append a choice. The first one added starts as the current choice."""
        _lib.press_popup_button_add.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_popup_button_add(self.handle, *_s(label)))

    def clear(self):
        """Remove every choice."""
        _lib.press_popup_button_clear.argtypes = [ctypes.c_uint64]
        _check(_lib.press_popup_button_clear(self.handle))

    def set_selected(self, value):
        """The index of the current choice, counting from 0; -1 when there are no choices. Setting it does not raise changed."""
        _lib.press_popup_button_set_selected.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_popup_button_set_selected(self.handle, value))

    def get_selected(self):
        """The index of the current choice, counting from 0; -1 when there are no choices. Setting it does not raise changed."""
        _lib.press_popup_button_get_selected.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_popup_button_get_selected(self.handle, ctypes.byref(out)))
        return out.value

    def on_changed(self, fn):
        """The user picked a different choice. Picking the current one again raises nothing. The handler receives (index); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_popup_button_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, index, _userdata: f(index))


class ScrollView(Widget):
    """A framed opening onto a widget that may be larger than the space it is given. It has a scroll bar down its right-hand side, and one along the bottom when the content is too wide. A widget inside that takes the keyboard is scrolled into view."""
    def __init__(self):
        """Create an empty scroll view, 200 by 120 pixels."""
        _lib.press_scroll_view_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_scroll_view_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_content(self, content):
        """Put a widget in the view; usually a Stack of others. It is laid out at the size it asks for, or the size of the opening if that is larger."""
        _lib.press_scroll_view_set_content.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_scroll_view_set_content(self.handle, _h(content)))

    def set_width(self, value):
        """The width the view asks for, frame and scroll bar included, in pixels."""
        _lib.press_scroll_view_set_width.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_view_set_width(self.handle, value))

    def set_height(self, value):
        """The height the view asks for, frame included, in pixels."""
        _lib.press_scroll_view_set_height.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_view_set_height(self.handle, value))

    def set_scroll_x(self, value):
        """How far the content is scrolled to the left, in pixels. A value past the end is brought back."""
        _lib.press_scroll_view_set_scroll_x.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_view_set_scroll_x(self.handle, value))

    def get_scroll_x(self):
        """How far the content is scrolled to the left, in pixels. A value past the end is brought back."""
        _lib.press_scroll_view_get_scroll_x.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_scroll_view_get_scroll_x(self.handle, ctypes.byref(out)))
        return out.value

    def set_scroll_y(self, value):
        """How far the content is scrolled up, in pixels. A value past the end is brought back."""
        _lib.press_scroll_view_set_scroll_y.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_view_set_scroll_y(self.handle, value))

    def get_scroll_y(self):
        """How far the content is scrolled up, in pixels. A value past the end is brought back."""
        _lib.press_scroll_view_get_scroll_y.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_scroll_view_get_scroll_y(self.handle, ctypes.byref(out)))
        return out.value


class ScrollBar(Widget):
    """A scroll bar: an arrow at each end and a box that slides between them. Its value runs from 0 to max. It scrolls nothing by itself; connect changed and move your content. A text area has one of its own."""
    def __init__(self, orientation):
        """Create a scroll bar with a value of 0, a max of 100 and a page of 10."""
        _lib.press_scroll_bar_new.argtypes = [ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_scroll_bar_new(int(orientation), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_value(self, value):
        """Where the box is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range."""
        _lib.press_scroll_bar_set_value.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_bar_set_value(self.handle, value))

    def get_value(self):
        """Where the box is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range."""
        _lib.press_scroll_bar_get_value.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_scroll_bar_get_value(self.handle, ctypes.byref(out)))
        return out.value

    def set_max(self, value):
        """The largest value: how far the content can scroll. With a max of 0 there is nothing to scroll and the bar goes blank."""
        _lib.press_scroll_bar_set_max.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_bar_set_max(self.handle, value))

    def get_max(self):
        """The largest value: how far the content can scroll. With a max of 0 there is nothing to scroll and the bar goes blank."""
        _lib.press_scroll_bar_get_max.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_scroll_bar_get_max(self.handle, ctypes.byref(out)))
        return out.value

    def set_page(self, value):
        """How much is in view at once, in the units of value. It sets the size of the box, and a click in the track moves by a page less a step."""
        _lib.press_scroll_bar_set_page.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_bar_set_page(self.handle, value))

    def set_step(self, value):
        """How far an arrow, or one notch of the wheel's three, moves the value."""
        _lib.press_scroll_bar_set_step.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_bar_set_step(self.handle, value))

    def set_length(self, value):
        """The length of the bar in pixels. It is always 16 pixels thick."""
        _lib.press_scroll_bar_set_length.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_scroll_bar_set_length(self.handle, value))

    def on_changed(self, fn):
        """The user moved the box, with an arrow, the track, a drag or the wheel. The handler receives (value); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_scroll_bar_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, value, _userdata: f(value))


class MenuBar(Widget):
    """The row of menu titles across the top of a window. Attach it with Window.set_menu_bar."""
    def __init__(self):
        """Create an empty menu bar."""
        _lib.press_menu_bar_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_menu_bar_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, menu):
        """Append a menu. A menu can be in only one bar."""
        _lib.press_menu_bar_add.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_menu_bar_add(self.handle, _h(menu)))


class Menu(Widget):
    """A pull-down menu. A disabled menu has a greyed title and cannot be pulled down."""
    def __init__(self, title):
        """Create an empty menu."""
        _lib.press_menu_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_menu_new(*_s(title), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, item):
        """Append an item. An item can be in only one menu."""
        _lib.press_menu_add.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_menu_add(self.handle, _h(item)))

    def add_separator(self):
        """Append a dividing line."""
        _lib.press_menu_add_separator.argtypes = [ctypes.c_uint64]
        _check(_lib.press_menu_add_separator(self.handle))

    def set_title(self, value):
        """The word shown in the menu bar."""
        _lib.press_menu_set_title.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_menu_set_title(self.handle, *_s(value)))


class MenuItem(Widget):
    """A command in a menu. A disabled item is greyed and cannot be chosen."""
    def __init__(self, label):
        """Create a menu item."""
        _lib.press_menu_item_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_menu_item_new(*_s(label), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_submenu(self, menu):
        """Hang a submenu off the item. It opens beside the item when the item is pointed at, and the item no longer raises activated. The submenu's own title is not shown. A menu can hang in only one place."""
        _lib.press_menu_item_set_submenu.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
        _check(_lib.press_menu_item_set_submenu(self.handle, _h(menu)))

    def set_label(self, value):
        """The item's text."""
        _lib.press_menu_item_set_label.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_menu_item_set_label(self.handle, *_s(value)))

    def set_checked(self, value):
        """Show a check mark beside the item. Choosing the item does not change it."""
        _lib.press_menu_item_set_checked.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_menu_item_set_checked(self.handle, value))

    def get_checked(self):
        """Show a check mark beside the item. Choosing the item does not change it."""
        _lib.press_menu_item_get_checked.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_menu_item_get_checked(self.handle, ctypes.byref(out)))
        return out.value

    def set_shortcut(self, value):
        """One character that, typed with Control, chooses the item; empty for none. In a text field, Control with A, C, X or V edits the text instead. Or the name of a key that chooses the item alone: F1 to F12 but F10, Insert, Delete or Backspace (Delete and Backspace edit a text field that has the keyboard instead)."""
        _lib.press_menu_item_set_shortcut.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_menu_item_set_shortcut(self.handle, *_s(value)))

    def on_activated(self, fn):
        """The user chose the item, from the menu or with its shortcut. The handler receives (); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_void_p)
        connector = _lib.press_menu_item_on_activated
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("activated", connector, cfunc, fn, lambda f: lambda _this, _userdata: f())


class GroupBox(Stack):
    """A box with a title, round things that belong together. It is a Stack: every Stack function works on it, and its children are laid out inside the box."""
    def __init__(self, title, orientation):
        """Create an empty box. With an empty title it is a box and no more."""
        _lib.press_group_box_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int32, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_group_box_new(*_s(title), int(orientation), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_title(self, value):
        """The words at the top of the box."""
        _lib.press_group_box_set_title.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_group_box_set_title(self.handle, *_s(value)))


class Toolbar(Stack):
    """A bar of tools, for the top of a window: a Stack that lays its children out in a row, close together, on a bar drawn by the style. Buttons in it are usually flat."""
    def __init__(self):
        """Create an empty toolbar."""
        _lib.press_toolbar_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_toolbar_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)


class StatusBar(Stack):
    """A bar of news, for the foot of a window: a Stack that lays its children out in a row on a bar drawn by the style. Labels are what it usually holds."""
    def __init__(self):
        """Create an empty status bar."""
        _lib.press_status_bar_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_status_bar_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_panels(self, value):
        """Whether each child is shown in a cell of its own, the last reaching the end of the bar. At first they are all on one strip."""
        _lib.press_status_bar_set_panels.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_status_bar_set_panels(self.handle, value))


class ButtonRow(Stack):
    """The row of buttons at the foot of a dialog: a Stack that lays its children out in a row at its far end, in the order the style of the program has them. Add the button that agrees first, then the one that calls it off, then any others; where the style has OK before Cancel they are shown as added, and where it has Cancel before OK, last first."""
    def __init__(self):
        """Create an empty row of buttons."""
        _lib.press_button_row_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_button_row_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)


class Stepper(Widget):
    """A whole number with a pair of arrows beside it, one over the other, to step it up and down. An arrow held down steps again and again. The arrow keys step it too when it has the keyboard."""
    def __init__(self):
        """Create a stepper at 0, which goes from 0 to 100 by ones."""
        _lib.press_stepper_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_stepper_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_value(self, value):
        """The number. Setting it does not raise changed; a value out of range is brought into range."""
        _lib.press_stepper_set_value.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stepper_set_value(self.handle, value))

    def get_value(self):
        """The number. Setting it does not raise changed; a value out of range is brought into range."""
        _lib.press_stepper_get_value.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_stepper_get_value(self.handle, ctypes.byref(out)))
        return out.value

    def set_min(self, value):
        """The least the number may be."""
        _lib.press_stepper_set_min.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stepper_set_min(self.handle, value))

    def set_max(self, value):
        """The most the number may be."""
        _lib.press_stepper_set_max.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stepper_set_max(self.handle, value))

    def set_step(self, value):
        """How far one press of an arrow moves the number, at least 1."""
        _lib.press_stepper_set_step.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_stepper_set_step(self.handle, value))

    def on_changed(self, fn):
        """The user stepped the number. The handler receives (value); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_stepper_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, value, _userdata: f(value))


class ComboBox(TextField):
    """A text field with a list of things it might say, dropped from a button at its end. Choosing one puts it in the field and raises changed; anything else may be typed. It is a TextField: every TextField function and event works on it."""
    def __init__(self, text):
        """Create a combo box holding the given text, with no choices yet."""
        _lib.press_combo_box_new.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_combo_box_new(*_s(text), ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def add(self, choice):
        """Append a choice to the list."""
        _lib.press_combo_box_add.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_combo_box_add(self.handle, *_s(choice)))

    def clear(self):
        """Take every choice away. The text in the field stays."""
        _lib.press_combo_box_clear.argtypes = [ctypes.c_uint64]
        _check(_lib.press_combo_box_clear(self.handle))


class RichText(Widget):
    """A document of words with looks of their own, edited as a word processor edits: each stretch of words in a font and size of its own, bold, italic, underlined, struck through and in a colour, in paragraphs aligned, indented and bulleted. Lines break at word gaps to fit the widget, or a width the program sets. It scrolls up and down, with a bar of its own. Its document is read and written as RTF. Places in it are byte offsets into the UTF-8 of its plain text, paragraphs parted by line breaks. Typing, the keys and the pointer work as in a text area; Control with B, I or U turns bold, italic or underline on or off, and Control with Z and Y undoes and does again, unless a menu has those keys. Copying puts the selection on the clipboard as RTF and as plain text; pasting takes RTF where the clipboard has it."""
    def __init__(self):
        """Create an empty rich text widget."""
        _lib.press_rich_text_new.argtypes = [ctypes.POINTER(ctypes.c_uint64)]
        out = ctypes.c_uint64()
        _check(_lib.press_rich_text_new(ctypes.byref(out)))
        _Object.__init__(self, out.value)

    def set_indents(self, left, right, first):
        """Set the indents of every paragraph the selection touches, in pixels at 96 to the inch: in from the left, in from the right, and how much further in (or, if less than nought, out) the first line goes."""
        _lib.press_rich_text_set_indents.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_rich_text_set_indents(self.handle, left, right, first))

    def indent_left(self):
        """How far in from the left the paragraph the caret is in is, in pixels."""
        _lib.press_rich_text_indent_left.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_indent_left(self.handle, ctypes.byref(out)))
        return out.value

    def indent_right(self):
        """How far in from the right the paragraph the caret is in is, in pixels."""
        _lib.press_rich_text_indent_right.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_indent_right(self.handle, ctypes.byref(out)))
        return out.value

    def indent_first(self):
        """How much further in than the rest the first line of the paragraph the caret is in goes, in pixels."""
        _lib.press_rich_text_indent_first.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_indent_first(self.handle, ctypes.byref(out)))
        return out.value

    def select(self, from_, to):
        """Select from one place to another, each a byte offset into the plain text: `to` is where the caret goes, and is scrolled into view. The same place twice puts the caret there. Made by the program, it does not raise selected."""
        _lib.press_rich_text_select.argtypes = [ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32]
        _check(_lib.press_rich_text_select(self.handle, from_, to))

    def selection_start(self):
        """Where the selection begins; with nothing selected, where the caret is."""
        _lib.press_rich_text_selection_start.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_selection_start(self.handle, ctypes.byref(out)))
        return out.value

    def selection_end(self):
        """Where the selection ends; with nothing selected, where the caret is."""
        _lib.press_rich_text_selection_end.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_selection_end(self.handle, ctypes.byref(out)))
        return out.value

    def select_all(self):
        """Select the whole document."""
        _lib.press_rich_text_select_all.argtypes = [ctypes.c_uint64]
        _check(_lib.press_rich_text_select_all(self.handle))

    def insert(self, text):
        """Put text in place of the selection, as if typed, in the look the selection has. Line breaks in it start paragraphs. It can be undone. It does not raise changed."""
        _lib.press_rich_text_insert.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_rich_text_insert(self.handle, *_s(text)))

    def copy(self):
        """Copy the selection to the clipboard, as RTF and as plain text. With nothing selected the clipboard is left alone."""
        _lib.press_rich_text_copy.argtypes = [ctypes.c_uint64]
        _check(_lib.press_rich_text_copy(self.handle))

    def cut(self):
        """Copy the selection, then take it out. Raises changed."""
        _lib.press_rich_text_cut.argtypes = [ctypes.c_uint64]
        _check(_lib.press_rich_text_cut(self.handle))

    def paste(self):
        """Put the clipboard's words in place of the selection: its RTF if it has any, otherwise its plain text. Raises changed; on a display the words may arrive a moment after this returns."""
        _lib.press_rich_text_paste.argtypes = [ctypes.c_uint64]
        _check(_lib.press_rich_text_paste(self.handle))

    def undo(self):
        """Undo the last change, if there is one. It does not raise changed."""
        _lib.press_rich_text_undo.argtypes = [ctypes.c_uint64]
        _check(_lib.press_rich_text_undo(self.handle))

    def redo(self):
        """Do again what was last undone, if anything was. It does not raise changed."""
        _lib.press_rich_text_redo.argtypes = [ctypes.c_uint64]
        _check(_lib.press_rich_text_redo(self.handle))

    def can_undo(self):
        """Whether there is a change to undo."""
        _lib.press_rich_text_can_undo.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_can_undo(self.handle, ctypes.byref(out)))
        return out.value

    def can_redo(self):
        """Whether there is a change undone to do again."""
        _lib.press_rich_text_can_redo.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_can_redo(self.handle, ctypes.byref(out)))
        return out.value

    def set_rtf(self, value):
        """The whole document as RTF. Setting it reads RTF (or, if it is not RTF, plain text) in place of what was there, forgets what could be undone, and does not raise changed."""
        _lib.press_rich_text_set_rtf.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_rich_text_set_rtf(self.handle, *_s(value)))

    def get_rtf(self):
        """The whole document as RTF. Setting it reads RTF (or, if it is not RTF, plain text) in place of what was there, forgets what could be undone, and does not raise changed."""
        _lib.press_rich_text_get_rtf.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
        size = ctypes.c_size_t()
        _check(_lib.press_rich_text_get_rtf(self.handle, None, 0, ctypes.byref(size)))
        buffer = ctypes.create_string_buffer(size.value + 1)
        _check(_lib.press_rich_text_get_rtf(self.handle, buffer, len(buffer), ctypes.byref(size)))
        return buffer.raw[:min(size.value, len(buffer) - 1)].decode("utf-8")

    def set_text(self, value):
        """The document's words, as plain text, paragraphs parted by line breaks. Setting it puts plain text in place of what was there, in the look the first words had, and does not raise changed."""
        _lib.press_rich_text_set_text.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_rich_text_set_text(self.handle, *_s(value)))

    def get_text(self):
        """The document's words, as plain text, paragraphs parted by line breaks. Setting it puts plain text in place of what was there, in the look the first words had, and does not raise changed."""
        _lib.press_rich_text_get_text.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
        size = ctypes.c_size_t()
        _check(_lib.press_rich_text_get_text(self.handle, None, 0, ctypes.byref(size)))
        buffer = ctypes.create_string_buffer(size.value + 1)
        _check(_lib.press_rich_text_get_text(self.handle, buffer, len(buffer), ctypes.byref(size)))
        return buffer.raw[:min(size.value, len(buffer) - 1)].decode("utf-8")

    def set_wrap_width(self, value):
        """Break lines at this many pixels, as on a page, whatever the widget's width; 0, the default, breaks them to fit the widget."""
        _lib.press_rich_text_set_wrap_width.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_rich_text_set_wrap_width(self.handle, value))

    def set_bold(self, value):
        """Whether the selection is bold: as its first character is. Setting it makes the selection bold or not, or with nothing selected what is typed next."""
        _lib.press_rich_text_set_bold.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_rich_text_set_bold(self.handle, value))

    def get_bold(self):
        """Whether the selection is bold: as its first character is. Setting it makes the selection bold or not, or with nothing selected what is typed next."""
        _lib.press_rich_text_get_bold.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_get_bold(self.handle, ctypes.byref(out)))
        return out.value

    def set_italic(self, value):
        """Whether the selection is italic; setting it works as bold does."""
        _lib.press_rich_text_set_italic.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_rich_text_set_italic(self.handle, value))

    def get_italic(self):
        """Whether the selection is italic; setting it works as bold does."""
        _lib.press_rich_text_get_italic.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_get_italic(self.handle, ctypes.byref(out)))
        return out.value

    def set_underline(self, value):
        """Whether the selection is underlined; setting it works as bold does."""
        _lib.press_rich_text_set_underline.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_rich_text_set_underline(self.handle, value))

    def get_underline(self):
        """Whether the selection is underlined; setting it works as bold does."""
        _lib.press_rich_text_get_underline.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_get_underline(self.handle, ctypes.byref(out)))
        return out.value

    def set_strikeout(self, value):
        """Whether the selection is struck through; setting it works as bold does."""
        _lib.press_rich_text_set_strikeout.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_rich_text_set_strikeout(self.handle, value))

    def get_strikeout(self):
        """Whether the selection is struck through; setting it works as bold does."""
        _lib.press_rich_text_get_strikeout.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_get_strikeout(self.handle, ctypes.byref(out)))
        return out.value

    def set_font(self, value):
        """The family of the selection's font, as fontconfig knows it; setting it works as bold does."""
        _lib.press_rich_text_set_font.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t]
        _check(_lib.press_rich_text_set_font(self.handle, *_s(value)))

    def get_font(self):
        """The family of the selection's font, as fontconfig knows it; setting it works as bold does."""
        _lib.press_rich_text_get_font.argtypes = [ctypes.c_uint64, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
        size = ctypes.c_size_t()
        _check(_lib.press_rich_text_get_font(self.handle, None, 0, ctypes.byref(size)))
        buffer = ctypes.create_string_buffer(size.value + 1)
        _check(_lib.press_rich_text_get_font(self.handle, buffer, len(buffer), ctypes.byref(size)))
        return buffer.raw[:min(size.value, len(buffer) - 1)].decode("utf-8")

    def set_size(self, value):
        """The size of the selection's type, in points; setting it works as bold does."""
        _lib.press_rich_text_set_size.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_rich_text_set_size(self.handle, value))

    def get_size(self):
        """The size of the selection's type, in points; setting it works as bold does."""
        _lib.press_rich_text_get_size.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_get_size(self.handle, ctypes.byref(out)))
        return out.value

    def set_color(self, value):
        """The colour of the selection, 0xRRGGBB; setting it works as bold does."""
        _lib.press_rich_text_set_color.argtypes = [ctypes.c_uint64, ctypes.c_uint32]
        _check(_lib.press_rich_text_set_color(self.handle, value))

    def get_color(self):
        """The colour of the selection, 0xRRGGBB; setting it works as bold does."""
        _lib.press_rich_text_get_color.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint32)]
        out = ctypes.c_uint32()
        _check(_lib.press_rich_text_get_color(self.handle, ctypes.byref(out)))
        return out.value

    def set_justify(self, value):
        """Where the lines of the paragraph the caret is in sit: Start at the left, Center in the middle, End at the right, Fill with both edges straight. Setting it sets every paragraph the selection touches."""
        _lib.press_rich_text_set_justify.argtypes = [ctypes.c_uint64, ctypes.c_int32]
        _check(_lib.press_rich_text_set_justify(self.handle, int(value)))

    def get_justify(self):
        """Where the lines of the paragraph the caret is in sit: Start at the left, Center in the middle, End at the right, Fill with both edges straight. Setting it sets every paragraph the selection touches."""
        _lib.press_rich_text_get_justify.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_int32)]
        out = ctypes.c_int32()
        _check(_lib.press_rich_text_get_justify(self.handle, ctypes.byref(out)))
        return Align(out.value)

    def set_bullet(self, value):
        """Whether the paragraph the caret is in has a bullet. Setting it sets every paragraph the selection touches."""
        _lib.press_rich_text_set_bullet.argtypes = [ctypes.c_uint64, ctypes.c_bool]
        _check(_lib.press_rich_text_set_bullet(self.handle, value))

    def get_bullet(self):
        """Whether the paragraph the caret is in has a bullet. Setting it sets every paragraph the selection touches."""
        _lib.press_rich_text_get_bullet.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_bool)]
        out = ctypes.c_bool()
        _check(_lib.press_rich_text_get_bullet(self.handle, ctypes.byref(out)))
        return out.value

    def on_changed(self, fn):
        """The user changed the document: by typing, the keys, a cut, a paste, or undoing. The handler receives (); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_void_p)
        connector = _lib.press_rich_text_on_changed
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("changed", connector, cfunc, fn, lambda f: lambda _this, _userdata: f())

    def on_selected(self, fn):
        """The user moved the caret or changed the selection. The handler is given where the selection begins and ends, the start first; equal, they are the caret. It comes after changed when both happen. The handler receives (start, end); pass None to disconnect."""
        cfunc = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_int32, ctypes.c_int32, ctypes.c_void_p)
        connector = _lib.press_rich_text_on_selected
        connector.argtypes = [ctypes.c_uint64, cfunc, ctypes.c_void_p, _DESTROY]
        self._connect("selected", connector, cfunc, fn, lambda f: lambda _this, start, end, _userdata: f(start, end))
