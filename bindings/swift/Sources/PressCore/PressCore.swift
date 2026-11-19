// Generated from idl/press.toml by press-idl. Do not edit.
// SPDX-License-Identifier: GPL-3.0-or-later
//
// press: a one-bit user interface toolkit with a flat C ABI.

import CPress

/// The ABI version this binding was generated for. `App.abiVersion()` is that
/// of the library that was loaded.
public let pressABIVersion: UInt32 = 1

/// What a call throws when it does not return `Status.ok`.
public struct PressError: Error, Equatable, CustomStringConvertible {
    /// The status as the library returned it.
    public let code: Int32

    public init(code: Int32) {
        self.code = code
    }

    public init(_ status: Status) {
        self.code = status.rawValue
    }

    /// The status, or nil if the library is newer than this binding and
    /// returned one it does not know.
    public var status: Status? {
        Status(rawValue: code)
    }

    public var description: String {
        status.map { "\($0): \($0.message)" } ?? "unknown status \(code)"
    }
}

/// Wraps a handle. Letting go of the wrapper does not destroy the object;
/// `destroy` does, and so does destroying whatever the object is inside.
/// Two wrappers of the same handle are equal.
open class PressObject: Hashable {
    public let handle: UInt64

    /// Wrap a handle that is already known, as a class of your choosing. The
    /// library checks the class at each call and refuses the wrong one.
    public init(handle: UInt64) {
        self.handle = handle
    }

    public static func == (a: PressObject, b: PressObject) -> Bool {
        a.handle == b.handle
    }

    public func hash(into hasher: inout Hasher) {
        hasher.combine(handle)
    }
}

@inline(__always)
private func _check(_ code: Int32) throws {
    if code != 0 {
        throw PressError(code: code)
    }
}

/// Fetch a string result: ask for its length, then for the string itself in
/// a buffer that fits.
private func _string(_ fetch: (UnsafeMutablePointer<CChar>?, Int, UnsafeMutablePointer<Int>?) -> Int32) throws -> String {
    var size = 0
    try _check(fetch(nil, 0, &size))
    var buffer = [CChar](repeating: 0, count: size + 1)
    let capacity = buffer.count
    try _check(buffer.withUnsafeMutableBufferPointer { fetch($0.baseAddress, capacity, &size) })
    return String(decoding: buffer.prefix(min(size, capacity - 1)).map { UInt8(bitPattern: $0) }, as: UTF8.self)
}

/// Copy a string lent to a handler.
private func _text(_ pointer: UnsafePointer<CChar>?, _ length: Int) -> String {
    guard let pointer, length > 0 else { return "" }
    return pointer.withMemoryRebound(to: UInt8.self, capacity: length) {
        String(decoding: UnsafeBufferPointer(start: $0, count: length), as: UTF8.self)
    }
}

/// Copy bytes lent to a handler.
private func _bytes(_ pointer: UnsafePointer<UInt8>?, _ length: Int) -> [UInt8] {
    guard let pointer, length > 0 else { return [] }
    return Array(UnsafeBufferPointer(start: pointer, count: length))
}

/// Keeps a closure alive while the library holds it as userdata.
private final class _Box<T> {
    let value: T

    init(_ value: T) {
        self.value = value
    }
}

/// The closure behind a userdata pointer. The caller holds its own reference
/// to it for the length of the call, so a handler may replace or disconnect
/// itself while it runs.
private func _unbox<T>(_ userdata: UnsafeMutableRawPointer?, as type: T.Type) -> T {
    Unmanaged<_Box<T>>.fromOpaque(userdata!).takeUnretainedValue().value
}

/// The destroy notifier of every closure handed to the library.
private func _release(_ userdata: UnsafeMutableRawPointer?) {
    if let userdata {
        Unmanaged<AnyObject>.fromOpaque(userdata).release()
    }
}

/// Runs a closure handed over as a piece of work.
private func _runWork(_ userdata: UnsafeMutableRawPointer?) {
    _unbox(userdata, as: (() -> Void).self)()
}

/// Result of every call. Zero is success.
public enum Status: Int32, CaseIterable, Sendable {
    /// success
    case ok = 0
    /// press_app_init has not been called on this thread
    case notInitialized = 1
    /// the handle is null, stale or already destroyed
    case invalidHandle = 2
    /// the handle refers to an object of another class
    case wrongType = 3
    /// an argument is out of range, null or not valid UTF-8
    case invalidArgument = 4
    /// the call is not allowed in the current state
    case invalidState = 5
    /// the display backend failed
    case backend = 6
    /// a file could not be read or written
    case io = 7
    /// an internal error; this is a bug in press
    case `internal` = 8

    /// A short description of the status.
    public var message: String {
        switch self {
        case .ok: return "success"
        case .notInitialized: return "press_app_init has not been called on this thread"
        case .invalidHandle: return "the handle is null, stale or already destroyed"
        case .wrongType: return "the handle refers to an object of another class"
        case .invalidArgument: return "an argument is out of range, null or not valid UTF-8"
        case .invalidState: return "the call is not allowed in the current state"
        case .backend: return "the display backend failed"
        case .io: return "a file could not be read or written"
        case .`internal`: return "an internal error; this is a bug in press"
        }
    }
}

/// Direction in which a Stack lays out its children.
public enum Orientation: Int32, CaseIterable, Sendable {
    /// top to bottom
    case vertical = 0
    /// left to right
    case horizontal = 1
}

/// Placement of children across a Stack's axis.
public enum Align: Int32, CaseIterable, Sendable {
    /// left or top
    case start = 0
    /// centred
    case center = 1
    /// right or bottom
    case end = 2
    /// stretched to the full width or height
    case fill = 3
}

/// A key that does not type a character.
public enum Key: Int32, CaseIterable, Sendable {
    /// move the focus on; with Shift, back
    case tab = 1
    /// press the default button
    case `return` = 2
    /// press the cancel button
    case escape = 3
    /// delete backwards
    case backspace = 4
    /// delete forwards
    case delete = 5
    /// left arrow
    case left = 6
    /// right arrow
    case right = 7
    /// up arrow
    case up = 8
    /// down arrow
    case down = 9
    /// start of the line
    case home = 10
    /// end of the line
    case end = 11
    /// pull down or put away the menu bar (F10)
    case menu = 12
    /// pop up the context menu of whatever has the keyboard (the Menu key, or Shift+F10)
    case context = 13
    /// a page up; told only to a picture that hears every key
    case pageUp = 14
    /// a page down; told only to a picture that hears every key
    case pageDown = 15
    /// Insert; told only to a picture that hears every key
    case insert = 16
    /// a function key, told only to a picture that hears every key; F2 is 22 and so on
    case f1 = 21
    /// F2
    case f2 = 22
    /// F3
    case f3 = 23
    /// F4
    case f4 = 24
    /// F5
    case f5 = 25
    /// F6
    case f6 = 26
    /// F7
    case f7 = 27
    /// F8
    case f8 = 28
    /// F9
    case f9 = 29
    /// F10, which otherwise works the menu bar
    case f10 = 30
    /// F11
    case f11 = 31
    /// F12
    case f12 = 32
}

/// Modifier keys held during a key press. Combine with bitwise OR.
public struct Modifier: OptionSet, Hashable, Sendable {
    public let rawValue: UInt32

    public init(rawValue: UInt32) {
        self.rawValue = rawValue
    }

    /// extends the selection; reverses Tab
    public static let shift = Modifier(rawValue: 1)
    /// the command key: moves and deletes by word
    public static let control = Modifier(rawValue: 2)
    /// reserved
    public static let alt = Modifier(rawValue: 4)
}

/// A small picture shown before an item's label in a list.
public enum Icon: Int32, CaseIterable, Sendable {
    /// no picture
    case none = 0
    /// a folder
    case folder = 1
    /// a sheet of paper with a corner turned down
    case document = 2
    /// a diamond
    case application = 3
}

/// How a line is drawn on a Picture.
public enum LineStyle: Int32, CaseIterable, Sendable {
    /// every pixel
    case solid = 0
    /// every other pixel
    case dotted = 1
    /// four pixels on and three off
    case dashed = 2
}

/// One of the bundled typefaces, each drawn in pixels for the one size it is used at. Every one has Western, Greek, Cyrillic and Hebrew letters of its own and the Chinese, Japanese and Korean characters they all share.
public enum Typeface: Int32, CaseIterable, Sendable {
    /// heavy and plain, 12 pixels: the face of the interface
    case cicero = 0
    /// small and light, 11 pixels, for a lot of words in a little room
    case agate = 1
    /// Agate made heavy
    case agateBold = 2
    /// light and airy, 12 pixels, for reading
    case brevier = 3
    /// every character the same width, 12 pixels, for columns and program text
    case pica = 4
    /// the same, heavy and larger, 16 pixels, in the manner of a terminal
    case elite = 5
}

/// Process-wide state. A process has one UI thread, the one that called init, and every press call must come from it, except post.
public enum App {
    /// Start press and make the calling thread the UI thread. Safe to call again from that thread; refused from any other until it has shut down.
    public static func initialize() throws {
        try _check(press_app_init())
    }

    /// Show the windows and process events until quit is called or the last window closes. With the headless backend this returns at once.
    public static func run() throws {
        try _check(press_app_run())
    }

    /// For programs with an event loop of their own, in place of run: connect to the display without waiting. Then wait on the descriptor from fd and call dispatch whenever it is readable.
    public static func start() throws {
        try _check(press_app_start())
    }

    /// A file descriptor that becomes readable whenever dispatch has work: input, an animation or key repeat coming due, or a change made through this API that needs drawing. Only poll it; never read from it. Valid from start until stop.
    public static func fd() throws -> Int {
        var out = Int32(0)
        try _check(press_app_fd(&out))
        return Int(out)
    }

    /// Do everything that is ready, without waiting: handle input, call handlers, redraw. Returns false once quit has been called or no window is left showing. Not allowed from inside a handler.
    public static func dispatch() throws -> Bool {
        var out = false
        try _check(press_app_dispatch(&out))
        return out
    }

    /// Disconnect from the display after start. Objects survive and can be shown again with start.
    public static func stop() throws {
        try _check(press_app_stop())
    }

    /// Hand over work to be done on the UI thread. This is the one function that may be called from any thread. The work is called from the next dispatch (or by run), in the order posted, and may call any press function. If this returns an error nothing was taken: neither the work nor its destroy notifier will be called.
    public static func post(_ work: @escaping @Sendable () -> Void) throws {
        let box = Unmanaged.passRetained(_Box<() -> Void>(work))
        let status = press_app_post(_runWork, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// Put something other than text on the clipboard: a picture, say. `kind` says what it is, as a media type such as image/png, and `data` is the thing itself, as the bytes of a file of that type. It takes the place of whatever was on the clipboard, and any program that understands the kind can paste it. Text fields look after copying text themselves.
    public static func copyData(_ kind: String, data: [UInt8]) throws {
        try _check(press_app_copy_data(kind, kind.utf8.count, data, data.count))
    }

    /// Add another kind to what this program last copied, for programs that understand that one and not the other: a picture and, with it, its caption as text/plain;charset=utf-8, say. Whoever pastes takes the kind it likes best. A kind given twice keeps the later data. It may follow copy_data or text copied from a text field; if the clipboard has since passed to another program it starts afresh, as copy_data does.
    public static func copyAlso(_ kind: String, data: [UInt8]) throws {
        try _check(press_app_copy_also(kind, kind.utf8.count, data, data.count))
    }

    /// Ask run to return, or dispatch to report false.
    public static func quit() throws {
        try _check(press_app_quit())
    }

    /// Destroy every object and release all callbacks. Posted work that has not been done is dropped, and its destroy notifier is called.
    public static func shutdown() throws {
        try _check(press_app_shutdown())
    }

    /// The ABI version of the loaded library.
    public static func abiVersion() throws -> UInt32 {
        var out = UInt32(0)
        try _check(press_app_abi_version(&out))
        return out
    }

    /// Choose how the program looks: the name of a style (ninetyfive, say, looked for as libpress_style_ninetyfive.so where styles are kept), the path of a style's library, or eightyfour for black and white. A style with a frame of its own has the program's windows draw their title bars and edges themselves, and one that keeps its menus in the window keeps them there on any desktop. Call it after init and before making windows. It takes the place of the desktop's choice of style; PRESS_STYLE in the program's environment, which is the user saying otherwise for one run, takes the place of this. A style that cannot be had is refused with INVALID_ARGUMENT, and the program looks as it did.
    public static func setStyle(_ name: String) throws {
        try _check(press_app_set_style(name, name.utf8.count))
    }
}

/// Base of everything that can be placed in a window, menus included. Its functions accept any handle.
public class Widget: PressObject {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Destroy the widget and everything inside it. Its handle becomes invalid.
    public func destroy() throws {
        try _check(press_widget_destroy(handle))
    }

    /// Give the widget a context menu: it pops up when the widget, or anything inside it, is clicked with the right button, or with the context-menu key while it has the keyboard. A window can carry one for everything in it. The menu belongs to the widget from then on. A text field without one of its own pops up Cut, Copy, Paste and Select All.
    public func setContextMenu(_ menu: Menu?) throws {
        try _check(press_widget_set_context_menu(handle, menu?.handle ?? 0))
    }

    /// Set the widget's text in another of the bundled typefaces. Labels, buttons, checkboxes, text fields, text areas and pictures have one; other widgets refuse it as the wrong type. A widget is the size its text needs in its typeface.
    public func setTypeface(_ face: Typeface) throws {
        try _check(press_widget_set_typeface(handle, face.rawValue))
    }

    /// Say how small the widget may be, at the least, in pixels. It is still as large as its kind makes it where that is larger, so this can make a button taller but not narrower than its label needs. Nought either way leaves that way to the widget.
    public func setMinSize(_ width: Int, height: Int) throws {
        try _check(press_widget_set_min_size(handle, Int32(clamping: width), Int32(clamping: height)))
    }

    /// Say whether the widget takes up spare room when its window is larger than its content needs. Without this it goes by kind: lists, tables, tree views, text areas, scroll views and tab views grow both ways; one-line text fields and progress bars grow across; sliders and scroll bars grow along their length; a stack grows if anything in it does; the rest keep their natural size. A widget told to grow does so along the stack it is in, so an empty label makes a spring that pushes its neighbours apart; one told not to keeps its natural size both ways.
    public func setStretch(_ value: Bool) throws {
        try _check(press_widget_set_stretch(handle, value))
    }

    /// Give the widget the keyboard. Only text fields, buttons and checkboxes that are enabled and in a window can take it.
    public func focus() throws {
        try _check(press_widget_focus(handle))
    }

    /// Disabled widgets are drawn greyed and ignore the pointer and the keyboard.
    public func setEnabled(_ value: Bool) throws {
        try _check(press_widget_set_enabled(handle, value))
    }

    /// Disabled widgets are drawn greyed and ignore the pointer and the keyboard.
    public var enabled: Bool {
        get throws {
            var out = false
            try _check(press_widget_get_enabled(handle, &out))
            return out
        }
    }

    /// A word of help, shown beside the pointer when it has rested on the widget a moment, and put away when it moves on or is pressed. An empty string takes it away.
    public func setTooltip(_ value: String) throws {
        try _check(press_widget_set_tooltip(handle, value, value.utf8.count))
    }
}

/// A top-level window. The desktop draws its title bar and frame.
public class Window: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a hidden window with the given content size in pixels.
    public init(title: String, width: Int, height: Int) throws {
        var out = UInt64(0)
        try _check(press_window_new(title, title.utf8.count, Int32(clamping: width), Int32(clamping: height), &out))
        super.init(handle: out)
    }

    /// Place a widget in the window; it fills the content area.
    public func setContent(_ content: Widget?) throws {
        try _check(press_window_set_content(handle, content?.handle ?? 0))
    }

    /// Give the window a menu bar. It is drawn across the top and takes its height from the content area.
    public func setMenuBar(_ bar: MenuBar?) throws {
        try _check(press_window_set_menu_bar(handle, bar?.handle ?? 0))
    }

    /// Make the window the size its content asks for, with room for its menu bar, in place of the size it was created with. Call it once the content and the menu bar are in place and before the window is first shown; a window that has been shown refuses.
    public func fit() throws {
        try _check(press_window_fit(handle))
    }

    /// Make the window visible.
    public func show() throws {
        try _check(press_window_show(handle))
    }

    /// The widget in the window that has the keyboard, or none (a handle of 0) when nothing has. A window that has not been typed into yet answers with its first text field, which is where typing will go.
    public func focused() throws -> Widget? {
        var out = UInt64(0)
        try _check(press_window_focused(handle, &out))
        return out == 0 ? nil : Widget(handle: out)
    }

    /// Render the window content to a PNG file, magnified by an integer scale: one bit a pixel, or in colour when the window shows a photo. An open menu is included, and the picture grows to fit it. Works with every backend.
    public func snapshot(_ path: String, scale: Int) throws {
        try _check(press_window_snapshot(handle, path, path.utf8.count, Int32(clamping: scale)))
    }

    /// Test hook: press and release the pointer at a point in the content area. An open menu is reached at the place it hangs, even outside the window.
    public func clickAt(_ x: Int, y: Int) throws {
        try _check(press_window_click_at(handle, Int32(clamping: x), Int32(clamping: y)))
    }

    /// Test hook: move the pointer to a point in the content area and leave it there for a number of milliseconds, as far as the toolkit's clock is concerned: long enough, and the word of help of what is under it shows.
    public func hoverAt(_ x: Int, y: Int, milliseconds: Int) throws {
        try _check(press_window_hover_at(handle, Int32(clamping: x), Int32(clamping: y), Int32(clamping: milliseconds)))
    }

    /// Test hook: press the pointer at one point in the content area, move it to another in four steps, and let it go there.
    public func dragAt(_ x0: Int, y0: Int, x1: Int, y1: Int) throws {
        try _check(press_window_drag_at(handle, Int32(clamping: x0), Int32(clamping: y0), Int32(clamping: x1), Int32(clamping: y1)))
    }

    /// Pop a menu up with its corner at a point in the content area. The menu may be free-standing or a context menu, not one in a menu bar or a submenu. Its items raise activated as usual.
    public func popupMenu(_ menu: Menu?, x: Int, y: Int) throws {
        try _check(press_window_popup_menu(handle, menu?.handle ?? 0, Int32(clamping: x), Int32(clamping: y)))
    }

    /// Test hook: press the right button at a point in the content area.
    public func contextClickAt(_ x: Int, y: Int) throws {
        try _check(press_window_context_click_at(handle, Int32(clamping: x), Int32(clamping: y)))
    }

    /// Test hook: press the pointer's back button, or its forward button, over the window.
    public func sideClick(_ forward: Bool) throws {
        try _check(press_window_side_click(handle, forward))
    }

    /// Test hook: say which Modifier keys are held during the clicks that follow. Pass 0 to let them go.
    public func holdModifiers(_ modifiers: UInt32) throws {
        try _check(press_window_hold_modifiers(handle, modifiers))
    }

    /// Test hook: press a key in the window, with any combination of Modifier bits.
    public func pressKey(_ key: Key, modifiers: UInt32) throws {
        try _check(press_window_press_key(handle, key.rawValue, modifiers))
    }

    /// Test hook: type text into the window as if from the keyboard.
    public func typeText(_ text: String) throws {
        try _check(press_window_type_text(handle, text, text.utf8.count))
    }

    /// Ask for what is on the clipboard as a given kind (a media type such as image/png). The answer arrives a moment later as data_pasted, with nothing in it if the clipboard holds nothing of that kind.
    public func pasteData(_ kind: String) throws {
        try _check(press_window_paste_data(handle, kind, kind.utf8.count))
    }

    /// Say that things of a given kind (a media type such as image/png) may be dragged from other programs and dropped on the window. Call it once for each kind, the one liked best first. What is dropped arrives as data_dropped.
    public func acceptDrops(_ kind: String) throws {
        try _check(press_window_accept_drops(handle, kind, kind.utf8.count))
    }

    /// Text shown in the title bar.
    public func setTitle(_ value: String) throws {
        try _check(press_window_set_title(handle, value, value.utf8.count))
    }

    /// Whether the user may change the window's size. Windows can be resized unless this is turned off, which suits a small tool that is all of a piece. Set it before the window is first shown.
    public func setResizable(_ value: Bool) throws {
        try _check(press_window_set_resizable(handle, value))
    }

    /// Whether the window is waiting on something the program is doing. While it is, the pointer over it is an hourglass; the window still answers the pointer and the keyboard.
    public func setBusy(_ value: Bool) throws {
        try _check(press_window_set_busy(handle, value))
    }

    /// A name by which the desktop can know this window again another day, so as to put it back where it was, at the size it was. It is not shown to anyone, so it is not translated, and it should be the same each time the program shows the same thing: a document's path, say, or "preferences". Set it before the window is first shown. Desktops that do not remember windows ignore it.
    public func setTag(_ value: String) throws {
        try _check(press_window_set_tag(handle, value, value.utf8.count))
    }

    /// Whether the window fills its screen, with no title bar or frame and in front of the menu bar, as a film or a slide show does. Turning it on asks the desktop, which may refuse; reading it says what the window last asked for or was told. While it is on the window draws no menu bar of its own, though its shortcuts still work. Turned off, the window goes back to where it was and its size. full_screen_changed says when the desktop changes it, as when the user leaves full screen by its own means.
    public func setFullScreen(_ value: Bool) throws {
        try _check(press_window_set_full_screen(handle, value))
    }

    /// Whether the window fills its screen, with no title bar or frame and in front of the menu bar, as a film or a slide show does. Turning it on asks the desktop, which may refuse; reading it says what the window last asked for or was told. While it is on the window draws no menu bar of its own, though its shortcuts still work. Turned off, the window goes back to where it was and its size. full_screen_changed says when the desktop changes it, as when the user leaves full screen by its own means.
    public var fullScreen: Bool {
        get throws {
            var out = false
            try _check(press_window_get_full_screen(handle, &out))
            return out
        }
    }

    /// The desktop has put the window into full screen, or taken it out, whether the program asked or not: whether it is now. The handler receives (full_screen). A new handler takes the place of the last; pass nil to disconnect.
    public func onFullScreenChanged(_ handler: ((Bool) -> Void)?) throws {
        guard let handler else {
            return try _check(press_window_on_full_screen_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Bool) -> Void>(handler))
        let status = press_window_on_full_screen_changed(handle, { _, fullScreen, userdata in
            _unbox(userdata, as: ((Bool) -> Void).self)(fullScreen)
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// Files were dragged from another program, or from a list whose items stand for files, and dropped on the window. The handler is given their paths, one per line, and whether the user asked for copies (by holding Control, on most desktops) rather than for the files to be moved. What is done with them is up to the handler. Without a handler the window does not accept files. The handler receives (paths, copy). A new handler takes the place of the last; pass nil to disconnect.
    public func onFilesDropped(_ handler: ((String, Bool) -> Void)?) throws {
        guard let handler else {
            return try _check(press_window_on_files_dropped(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(String, Bool) -> Void>(handler))
        let status = press_window_on_files_dropped(handle, { _, paths, pathsLength, copy, userdata in
            _unbox(userdata, as: ((String, Bool) -> Void).self)(_text(paths, pathsLength), copy)
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The answer to paste_data: the kind asked for, and what the clipboard held of that kind, which is nothing if it held none. The data is lent for the length of the call. The handler receives (kind, data). A new handler takes the place of the last; pass nil to disconnect.
    public func onDataPasted(_ handler: ((String, [UInt8]) -> Void)?) throws {
        guard let handler else {
            return try _check(press_window_on_data_pasted(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(String, [UInt8]) -> Void>(handler))
        let status = press_window_on_data_pasted(handle, { _, kind, kindLength, data, dataLength, userdata in
            _unbox(userdata, as: ((String, [UInt8]) -> Void).self)(_text(kind, kindLength), _bytes(data, dataLength))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// Something of a kind named with accept_drops was dragged from another program and dropped on the window: its kind, the thing itself, and where in the window's content it was let go. The data is lent for the length of the call. The handler receives (kind, data, x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onDataDropped(_ handler: ((String, [UInt8], Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_window_on_data_dropped(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(String, [UInt8], Int, Int) -> Void>(handler))
        let status = press_window_on_data_dropped(handle, { _, kind, kindLength, data, dataLength, x, y, userdata in
            _unbox(userdata, as: ((String, [UInt8], Int, Int) -> Void).self)(_text(kind, kindLength), _bytes(data, dataLength), Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The pointer's back or forward button, on the side of many a mouse, was pressed over the window: forward says which. What it does is the program's to say; without a handler nothing happens. The handler receives (forward). A new handler takes the place of the last; pass nil to disconnect.
    public func onNavigate(_ handler: ((Bool) -> Void)?) throws {
        guard let handler else {
            return try _check(press_window_on_navigate(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Bool) -> Void>(handler))
        let status = press_window_on_navigate(handle, { _, forward, userdata in
            _unbox(userdata, as: ((Bool) -> Void).self)(forward)
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user asked to close the window. Without a handler the application quits. A new handler takes the place of the last; pass nil to disconnect.
    public func onCloseRequested(_ handler: (() -> Void)?) throws {
        guard let handler else {
            return try _check(press_window_on_close_requested(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<() -> Void>(handler))
        let status = press_window_on_close_requested(handle, { _, userdata in
            _unbox(userdata, as: (() -> Void).self)()
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// Lays out its children in a row or a column.
public class Stack: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty stack.
    public init(_ orientation: Orientation) throws {
        var out = UInt64(0)
        try _check(press_stack_new(orientation.rawValue, &out))
        super.init(handle: out)
    }

    /// Append a child. A widget can have only one parent.
    public func add(_ child: Widget?) throws {
        try _check(press_stack_add(handle, child?.handle ?? 0))
    }

    /// Put a child at a place among the stack's children, counting from 0, with those after it moving along; a place past the end is the end. A child already in this stack is moved to the place, which is how children are put in another order.
    public func insert(_ child: Widget?, index: Int) throws {
        try _check(press_stack_insert(handle, child?.handle ?? 0, Int32(clamping: index)))
    }

    /// Gap between children, in pixels.
    public func setSpacing(_ value: Int) throws {
        try _check(press_stack_set_spacing(handle, Int32(clamping: value)))
    }

    /// Margin around the children, in pixels.
    public func setPadding(_ value: Int) throws {
        try _check(press_stack_set_padding(handle, Int32(clamping: value)))
    }

    /// Placement of children across the axis.
    public func setAlign(_ value: Align) throws {
        try _check(press_stack_set_align(handle, value.rawValue))
    }
}

/// A line of text.
public class Label: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a label.
    public init(_ text: String) throws {
        var out = UInt64(0)
        try _check(press_label_new(text, text.utf8.count, &out))
        super.init(handle: out)
    }

    /// The text shown.
    public func setText(_ value: String) throws {
        try _check(press_label_set_text(handle, value, value.utf8.count))
    }
}

/// A push button.
public class Button: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a button.
    public init(_ label: String) throws {
        var out = UInt64(0)
        try _check(press_button_new(label, label.utf8.count, &out))
        super.init(handle: out)
    }

    /// Give the button a picture, in colour, before its label: the bytes of a PNG file, up to 256 pixels each way, with see-through parts if it has them. A button with a picture and an empty label is the picture alone. No bytes take the picture away.
    public func setImage(_ png: [UInt8]) throws {
        try _check(press_button_set_image(handle, png, png.count))
    }

    /// The button's caption.
    public func setLabel(_ value: String) throws {
        try _check(press_button_set_label(handle, value, value.utf8.count))
    }

    /// Mark the default button of a dialog: it gets the heavy outline and answers the Return key.
    public func setIsDefault(_ value: Bool) throws {
        try _check(press_button_set_is_default(handle, value))
    }

    /// Mark the button that answers the Escape key.
    public func setIsCancel(_ value: Bool) throws {
        try _check(press_button_set_is_cancel(handle, value))
    }

    /// Whether the button lies flat, with no edge of its own until the pointer is over it or down on it, and is no wider than its label needs: a tool in a toolbar.
    public func setFlat(_ value: Bool) throws {
        try _check(press_button_set_flat(handle, value))
    }

    /// The button was pressed, with the pointer or from the keyboard. A new handler takes the place of the last; pass nil to disconnect.
    public func onClicked(_ handler: (() -> Void)?) throws {
        guard let handler else {
            return try _check(press_button_on_clicked(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<() -> Void>(handler))
        let status = press_button_on_clicked(handle, { _, userdata in
            _unbox(userdata, as: (() -> Void).self)()
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A box that is either checked or not, with a caption.
public class Checkbox: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an unchecked checkbox.
    public init(_ label: String) throws {
        var out = UInt64(0)
        try _check(press_checkbox_new(label, label.utf8.count, &out))
        super.init(handle: out)
    }

    /// The box's caption.
    public func setLabel(_ value: String) throws {
        try _check(press_checkbox_set_label(handle, value, value.utf8.count))
    }

    /// Whether the box is checked.
    public func setChecked(_ value: Bool) throws {
        try _check(press_checkbox_set_checked(handle, value))
    }

    /// Whether the box is checked.
    public var checked: Bool {
        get throws {
            var out = false
            try _check(press_checkbox_get_checked(handle, &out))
            return out
        }
    }

    /// The user changed the box. The handler receives (checked). A new handler takes the place of the last; pass nil to disconnect.
    public func onToggled(_ handler: ((Bool) -> Void)?) throws {
        guard let handler else {
            return try _check(press_checkbox_on_toggled(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Bool) -> Void>(handler))
        let status = press_checkbox_on_toggled(handle, { _, checked, userdata in
            _unbox(userdata, as: ((Bool) -> Void).self)(checked)
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// One line of editable text.
public class TextField: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a text field holding the given text.
    public init(_ text: String) throws {
        var out = UInt64(0)
        try _check(press_text_field_new(text, text.utf8.count, &out))
        super.init(handle: out)
    }

    /// Select all of the text.
    public func selectAll() throws {
        try _check(press_text_field_select_all(handle))
    }

    /// Select from one place in the text to another, each a byte offset into its UTF-8: `to` is where the caret goes, and is scrolled into view, so a selection made backwards has `to` first. The same place twice puts the caret there with nothing selected. A place inside a character moves to its start, and one past the end to the end. Made by the program, it does not raise selected.
    public func select(_ from: Int, to: Int) throws {
        try _check(press_text_field_select(handle, Int32(clamping: from), Int32(clamping: to)))
    }

    /// Where the selection begins, as a byte offset into the text's UTF-8; with nothing selected, where the caret is.
    public func selectionStart() throws -> Int {
        var out = Int32(0)
        try _check(press_text_field_selection_start(handle, &out))
        return Int(out)
    }

    /// Where the selection ends, as a byte offset into the text's UTF-8; with nothing selected, where the caret is.
    public func selectionEnd() throws -> Int {
        var out = Int32(0)
        try _check(press_text_field_selection_end(handle, &out))
        return Int(out)
    }

    /// Copy the selected text to the clipboard. With nothing selected the clipboard is left alone.
    public func copy() throws {
        try _check(press_text_field_copy(handle))
    }

    /// Copy the selected text to the clipboard and delete it. Raises changed.
    public func cut() throws {
        try _check(press_text_field_cut(handle))
    }

    /// Replace the selection with the clipboard's text. Raises changed; on a display the text may arrive a moment after this returns.
    public func paste() throws {
        try _check(press_text_field_paste(handle))
    }

    /// The text in the field. Setting it does not raise changed.
    public func setText(_ value: String) throws {
        try _check(press_text_field_set_text(handle, value, value.utf8.count))
    }

    /// The text in the field. Setting it does not raise changed.
    public var text: String {
        get throws {
            return try _string { press_text_field_get_text(handle, $0, $1, $2) }
        }
    }

    /// Greyed text shown while the field is empty.
    public func setPlaceholder(_ value: String) throws {
        try _check(press_text_field_set_placeholder(handle, value, value.utf8.count))
    }

    /// The narrowest the field may be laid out, in pixels.
    public func setMinWidth(_ value: Int) throws {
        try _check(press_text_field_set_min_width(handle, Int32(clamping: value)))
    }

    /// Show a bullet in place of each character. A password cannot be copied, cut or dragged out, input methods are kept away from it, and assistive technology is not told what it holds. Not available on a text area.
    public func setPassword(_ value: Bool) throws {
        try _check(press_text_field_set_password(handle, value))
    }

    /// The user edited the text. The handler is given the text as it now stands. The handler receives (text). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((String) -> Void)?) throws {
        guard let handler else {
            return try _check(press_text_field_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(String) -> Void>(handler))
        let status = press_text_field_on_changed(handle, { _, text, textLength, userdata in
            _unbox(userdata, as: ((String) -> Void).self)(_text(text, textLength))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user pressed Return in the field. The handler is given the text. The default button, if any, is pressed as well. The handler receives (text). A new handler takes the place of the last; pass nil to disconnect.
    public func onSubmitted(_ handler: ((String) -> Void)?) throws {
        guard let handler else {
            return try _check(press_text_field_on_submitted(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(String) -> Void>(handler))
        let status = press_text_field_on_submitted(handle, { _, text, textLength, userdata in
            _unbox(userdata, as: ((String) -> Void).self)(_text(text, textLength))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user moved the caret or changed the selection: by typing, the keys, the pointer or a paste. The handler is given where the selection begins and ends, as byte offsets into the text's UTF-8, the start first; equal, they are the caret. It comes after changed when both happen. A password field does not raise it. The handler receives (start, end). A new handler takes the place of the last; pass nil to disconnect.
    public func onSelected(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_text_field_on_selected(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_text_field_on_selected(handle, { _, start, end, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(start), Int(end))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// Several lines of editable text, wrapped at word gaps to the width of the area. It keeps tabs, each four spaces wide, and holds up to two megabytes, where a text field holds 64 kilobytes. It is a TextField: every TextField function and event works on it, except that Return starts a new line, so it never raises submitted and does not press the default button.
public class TextArea: TextField {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a text area holding the given text. Line breaks in the text are kept.
    public override init(_ text: String) throws {
        var out = UInt64(0)
        try _check(press_text_area_new(text, text.utf8.count, &out))
        super.init(handle: out)
    }

    /// The height of the area, in lines of text. Longer text scrolls.
    public func setRows(_ value: Int) throws {
        try _check(press_text_area_set_rows(handle, Int32(clamping: value)))
    }
}

/// One of a group of choices, of which one at most is chosen. Radio buttons that share a parent (the same Stack) are a group. The arrow keys move through the group, choosing as they go.
public class RadioButton: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an unchosen radio button.
    public init(_ label: String) throws {
        var out = UInt64(0)
        try _check(press_radio_button_new(label, label.utf8.count, &out))
        super.init(handle: out)
    }

    /// The button's caption.
    public func setLabel(_ value: String) throws {
        try _check(press_radio_button_set_label(handle, value, value.utf8.count))
    }

    /// Whether this is the chosen one. Setting it true unchooses the others in its group. Setting it does not raise chosen.
    public func setChecked(_ value: Bool) throws {
        try _check(press_radio_button_set_checked(handle, value))
    }

    /// Whether this is the chosen one. Setting it true unchooses the others in its group. Setting it does not raise chosen.
    public var checked: Bool {
        get throws {
            var out = false
            try _check(press_radio_button_get_checked(handle, &out))
            return out
        }
    }

    /// The user chose this button. Choosing the one already chosen raises nothing. A new handler takes the place of the last; pass nil to disconnect.
    public func onChosen(_ handler: (() -> Void)?) throws {
        guard let handler else {
            return try _check(press_radio_button_on_chosen(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<() -> Void>(handler))
        let status = press_radio_button_on_chosen(handle, { _, userdata in
            _unbox(userdata, as: (() -> Void).self)()
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A number picked by position: a knob that slides along a groove. Its value runs from 0 to max; a vertical slider has its largest value at the top.
public class Slider: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a slider with a value of 0 and a max of 100.
    public init(_ orientation: Orientation) throws {
        var out = UInt64(0)
        try _check(press_slider_new(orientation.rawValue, &out))
        super.init(handle: out)
    }

    /// Where the knob is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range.
    public func setValue(_ value: Int) throws {
        try _check(press_slider_set_value(handle, Int32(clamping: value)))
    }

    /// Where the knob is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range.
    public var value: Int {
        get throws {
            var out = Int32(0)
            try _check(press_slider_get_value(handle, &out))
            return Int(out)
        }
    }

    /// The largest value, at least 1.
    public func setMax(_ value: Int) throws {
        try _check(press_slider_set_max(handle, Int32(clamping: value)))
    }

    /// The largest value, at least 1.
    public var max: Int {
        get throws {
            var out = Int32(0)
            try _check(press_slider_get_max(handle, &out))
            return Int(out)
        }
    }

    /// How far an arrow key or a notch of the wheel moves the value.
    public func setStep(_ value: Int) throws {
        try _check(press_slider_set_step(handle, Int32(clamping: value)))
    }

    /// The length of the slider in pixels. It is always 16 pixels thick.
    public func setLength(_ value: Int) throws {
        try _check(press_slider_set_length(handle, Int32(clamping: value)))
    }

    /// The user moved the knob. The handler receives (value). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_slider_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_slider_on_changed(handle, { _, value, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(value))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// Rows of text, one of which may be selected. It scrolls when there are more rows than fit. The arrow keys move the selection, and typing a letter jumps to the next row that starts with it.
public class ListBox: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty list box, five rows high.
    public init() throws {
        var out = UInt64(0)
        try _check(press_list_box_new(&out))
        super.init(handle: out)
    }

    /// Append a row.
    public func add(_ label: String) throws {
        try _check(press_list_box_add(handle, label, label.utf8.count))
    }

    /// Remove every row.
    public func clear() throws {
        try _check(press_list_box_clear(handle))
    }

    /// Whether an item is selected. With multiple selection several may be.
    public func isSelected(_ index: Int) throws -> Bool {
        var out = false
        try _check(press_list_box_is_selected(handle, Int32(clamping: index), &out))
        return out
    }

    /// Select or unselect one item, leaving the others as they are where several may be selected. Does not raise changed.
    public func setItemSelected(_ index: Int, selected: Bool) throws {
        try _check(press_list_box_set_item_selected(handle, Int32(clamping: index), selected))
    }

    /// Give an item a small picture before its label. Once any item has one, every label moves over so that they line up.
    public func setIcon(_ index: Int, icon: Icon) throws {
        try _check(press_list_box_set_icon(handle, Int32(clamping: index), icon.rawValue))
    }

    /// Say that an item stands for a file, by its full path; an empty path takes that away. Such an item can be dragged out of its list, even one whose rows cannot be rearranged, and dropped on another window or program, which is offered the file. A window with a files_dropped handler hears of it that way.
    public func setFile(_ index: Int, path: String) throws {
        try _check(press_list_box_set_file(handle, Int32(clamping: index), path, path.utf8.count))
    }

    /// Give an item something to carry when it is dragged out of its list: `kind` says what it is, as a media type such as image/png, and `data` is the thing itself. Such an item can be dragged out even of a list whose rows cannot be rearranged, and a program that accepts the kind is given the data where it is dropped. An empty kind takes it away.
    public func setDragData(_ index: Int, kind: String, data: [UInt8]) throws {
        try _check(press_list_box_set_drag_data(handle, Int32(clamping: index), kind, kind.utf8.count, data, data.count))
    }

    /// Give an item another kind to carry when it is dragged out, besides what set_drag_data gave it: where it is dropped takes the kind it likes best. A kind given twice keeps the later data.
    public func addDragData(_ index: Int, kind: String, data: [UInt8]) throws {
        try _check(press_list_box_add_drag_data(handle, Int32(clamping: index), kind, kind.utf8.count, data, data.count))
    }

    /// Give an item a picture of your own in place of a stock icon. The picture is drawn in text, one line for each row of pixels: X or # is black, a full stop is white, and anything else lets the row show through. Up to 14 pixels each way. An empty string takes the picture away.
    public func setPicture(_ index: Int, art: String) throws {
        try _check(press_list_box_set_picture(handle, Int32(clamping: index), art, art.utf8.count))
    }

    /// Give an item a picture read from a PNG file of up to 14 pixels each way. It is reduced to one bit: dark pixels become black, light ones white, and transparent ones let the row show through.
    public func setPictureFile(_ index: Int, path: String) throws {
        try _check(press_list_box_set_picture_file(handle, Int32(clamping: index), path, path.utf8.count))
    }

    /// Remove an item. Items after it move down by one index. In a tree view, whatever hangs from it goes too. Does not raise removed.
    public func remove(_ index: Int) throws {
        try _check(press_list_box_remove(handle, Int32(clamping: index)))
    }

    /// Move an item to a place among the rows as shown, counting from 0. A sorted table stops being sorted. Does not raise moved.
    public func moveItem(_ index: Int, position: Int) throws {
        try _check(press_list_box_move_item(handle, Int32(clamping: index), Int32(clamping: position)))
    }

    /// Where an item is among the rows as shown, counting from 0; -1 if it is folded away in a tree.
    public func positionOf(_ index: Int) throws -> Int {
        var out = Int32(0)
        try _check(press_list_box_position_of(handle, Int32(clamping: index), &out))
        return Int(out)
    }

    /// Start editing an item's text in place, whether or not the list is editable. In a table, column says which cell; elsewhere pass 0.
    public func edit(_ index: Int, column: Int) throws {
        try _check(press_list_box_edit(handle, Int32(clamping: index), Int32(clamping: column)))
    }

    /// Give an item a picture in colour, in place of its icon: the bytes of a PNG file, up to 16 pixels each way, with see-through parts if it has them. No bytes take the picture away.
    public func setImage(_ index: Int, png: [UInt8]) throws {
        try _check(press_list_box_set_image(handle, Int32(clamping: index), png, png.count))
    }

    /// The index of the selected row, counting from 0; -1 when none is. Setting it scrolls the row into view and does not raise changed.
    public func setSelected(_ value: Int) throws {
        try _check(press_list_box_set_selected(handle, Int32(clamping: value)))
    }

    /// The index of the selected row, counting from 0; -1 when none is. Setting it scrolls the row into view and does not raise changed.
    public var selected: Int {
        get throws {
            var out = Int32(0)
            try _check(press_list_box_get_selected(handle, &out))
            return Int(out)
        }
    }

    /// Let several items be selected at once: Control with a click adds or removes one, Shift with a click or an arrow stretches the selection, a drag stretches it, and Control+A takes everything. selected then gives the item last selected.
    public func setMultiple(_ value: Bool) throws {
        try _check(press_list_box_set_multiple(handle, value))
    }

    /// Let the user edit an item's text in place. Return then edits the current item, as renaming a file does, where it would otherwise open it; a double click still opens it.
    public func setEditable(_ value: Bool) throws {
        try _check(press_list_box_set_editable(handle, value))
    }

    /// Let rows dragged out of another list be dropped here: from the same window, another window of the program, or a list in another press program. The rows leave that list, which must be reorderable, and join this one, keeping their text and pictures. A tree view can take rows too: dropped onto an item they hang from it, dropped between two they go there. An item dragged out of a tree takes its whole branch: into another tree the branch keeps its shape, into a plain list it is laid out flat.
    public func setAcceptsRows(_ value: Bool) throws {
        try _check(press_list_box_set_accepts_rows(handle, value))
    }

    /// Let the user drag a row to a new place: press it and pull it up or down, and a line shows where it will land. Where several rows are selected, pulling one moves them all. Items keep their indices; only the order they are shown in changes. In a tree view an item can also be dropped onto another, to hang from it, and takes along whatever hangs from it. Rows dragged onto something that takes text, here or in another program, arrive as their labels.
    public func setReorderable(_ value: Bool) throws {
        try _check(press_list_box_set_reorderable(handle, value))
    }

    /// The height of the list, in rows.
    public func setRows(_ value: Int) throws {
        try _check(press_list_box_set_rows(handle, Int32(clamping: value)))
    }

    /// The narrowest the list may be laid out, scroll bar included, in pixels.
    public func setMinWidth(_ value: Int) throws {
        try _check(press_list_box_set_min_width(handle, Int32(clamping: value)))
    }

    /// The user selected a different row. The handler receives (index). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_list_box_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_list_box_on_changed(handle, { _, index, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(index))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user opened a row: a double click, or Return. The default button, if any, is pressed as well after Return. The handler receives (index). A new handler takes the place of the last; pass nil to disconnect.
    public func onActivated(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_list_box_on_activated(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_list_box_on_activated(handle, { _, index, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(index))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user dragged an item to a new place. index is the item, unchanged; position is where it now is among the rows. The handler receives (index, position). A new handler takes the place of the last; pass nil to disconnect.
    public func onMoved(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_list_box_on_moved(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_list_box_on_moved(handle, { _, index, position, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(index), Int(position))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user dragged an item out of this list into another. index is the index it had; items after it have moved down by one. When several leave together the highest index is reported first, so each index is right as it is reported. The handler receives (index). A new handler takes the place of the last; pass nil to disconnect.
    public func onRemoved(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_list_box_on_removed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_list_box_on_removed(handle, { _, index, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(index))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user dropped an item here from another list. index is its index in this list, which is the last; position is where it is among the rows. The handler receives (index, position). A new handler takes the place of the last; pass nil to disconnect.
    public func onReceived(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_list_box_on_received(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_list_box_on_received(handle, { _, index, position, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(index), Int(position))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The user edited an item's text in place and kept the change. The list already holds the new text; column is 0 except in a table. The handler receives (index, column, text). A new handler takes the place of the last; pass nil to disconnect.
    public func onEdited(_ handler: ((Int, Int, String) -> Void)?) throws {
        guard let handler else {
            return try _check(press_list_box_on_edited(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int, String) -> Void>(handler))
        let status = press_list_box_on_edited(handle, { _, index, column, text, textLength, userdata in
            _unbox(userdata, as: ((Int, Int, String) -> Void).self)(Int(index), Int(column), _text(text, textLength))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A list box with columns under a row of headings. It is a ListBox: rows are added with ListBox.add, with the cells of a row separated by tab characters, and every ListBox function and event works on it.
public class Table: ListBox {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty table with no columns.
    public override init() throws {
        var out = UInt64(0)
        try _check(press_table_new(&out))
        super.init(handle: out)
    }

    /// Add a column: its heading and its width in pixels. A cell too long for its column is cut short. The user can drag the edge between two headings to change a width.
    public func addColumn(_ title: String, width: Int) throws {
        try _check(press_table_add_column(handle, title, title.utf8.count, Int32(clamping: width)))
    }

    /// Sort by a column, smallest first or largest first, or with -1 show the rows in the order they were added.
    public func sortBy(_ column: Int, ascending: Bool) throws {
        try _check(press_table_sort_by(handle, Int32(clamping: column), ascending))
    }

    /// Let the user sort the rows by clicking a column's heading; a second click turns the order round. Cells that start with numbers are ordered as numbers. Sorting changes the order rows are shown in, never their indices.
    public func setSortable(_ value: Bool) throws {
        try _check(press_table_set_sortable(handle, value))
    }
}

/// A list box that shows its items as large pictures with their names beneath, in rows that fill its width, as a folder's window does. It is a ListBox: every ListBox function and event works on it. An item's picture goes by its icon (a folder, a document or an application), or is the program's own picture drawn double size. In a label with tabs only what comes before the first is shown. The arrow keys go from icon to icon. With set_reorderable on, the user can put icons where they like.
public class IconView: ListBox {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty icon view, three rows of icons high.
    public override init() throws {
        var out = UInt64(0)
        try _check(press_icon_view_new(&out))
        super.init(handle: out)
    }

    /// Put an item's icon at a place in the view, measured in pixels from the top left of all the icons (not of the view, once it has been scrolled). A negative place puts it back in line with the rest. Clearing the view forgets every place.
    public func setPosition(_ index: Int, x: Int, y: Int) throws {
        try _check(press_icon_view_set_position(handle, Int32(clamping: index), Int32(clamping: x), Int32(clamping: y)))
    }

    /// The user dragged an icon to a new place in the view, which they can do once ListBox.set_reorderable has been turned on for it. index is the item; x and y are where it now is, as for set_position. Several selected icons dragged together are each reported. The order of the items does not change. The handler receives (index, x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onPlaced(_ handler: ((Int, Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_icon_view_on_placed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int, Int) -> Void>(handler))
        let status = press_icon_view_on_placed(handle, { _, index, x, y, userdata in
            _unbox(userdata, as: ((Int, Int, Int) -> Void).self)(Int(index), Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A list box whose rows hang from one another and can be folded away. It is a ListBox: an item is known by its index, in the order items were added, and every ListBox function and event works on it. ListBox.add adds an item at the top level. Right and Left unfold and fold from the keyboard.
public class TreeView: ListBox {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty tree view.
    public override init() throws {
        var out = UInt64(0)
        try _check(press_tree_view_new(&out))
        super.init(handle: out)
    }

    /// Add an item under another, given by its index, or at the top level with -1. Returns the new item's index. Items start folded.
    public func addChild(_ parent: Int, label: String) throws -> Int {
        var out = Int32(0)
        try _check(press_tree_view_add_child(handle, Int32(clamping: parent), label, label.utf8.count, &out))
        return Int(out)
    }

    /// The index of the item an item hangs from, or -1 at the top level. It changes when the user drags the item elsewhere in a reorderable tree.
    public func parentOf(_ index: Int) throws -> Int {
        var out = Int32(0)
        try _check(press_tree_view_parent_of(handle, Int32(clamping: index), &out))
        return Int(out)
    }

    /// Unfold or fold an item to show or hide what hangs from it.
    public func setExpanded(_ index: Int, expanded: Bool) throws {
        try _check(press_tree_view_set_expanded(handle, Int32(clamping: index), expanded))
    }
}

/// Shows how far along some work is. It fills from the left as value goes from 0 to max.
public class ProgressBar: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a progress bar with a value of 0 and a max of 100.
    public init() throws {
        var out = UInt64(0)
        try _check(press_progress_bar_new(&out))
        super.init(handle: out)
    }

    /// How much is done, from 0 to max. A value out of range is brought into range.
    public func setValue(_ value: Int) throws {
        try _check(press_progress_bar_set_value(handle, Int32(clamping: value)))
    }

    /// How much is done, from 0 to max. A value out of range is brought into range.
    public var value: Int {
        get throws {
            var out = Int32(0)
            try _check(press_progress_bar_get_value(handle, &out))
            return Int(out)
        }
    }

    /// The value that means finished, at least 1.
    public func setMax(_ value: Int) throws {
        try _check(press_progress_bar_set_max(handle, Int32(clamping: value)))
    }

    /// The value that means finished, at least 1.
    public var max: Int {
        get throws {
            var out = Int32(0)
            try _check(press_progress_bar_get_max(handle, &out))
            return Int(out)
        }
    }

    /// Show moving stripes in place of an amount, for work whose length is not known.
    public func setBusy(_ value: Bool) throws {
        try _check(press_progress_bar_set_busy(handle, value))
    }

    /// The length of the bar in pixels.
    public func setLength(_ value: Int) throws {
        try _check(press_progress_bar_set_length(handle, Int32(clamping: value)))
    }
}

/// A white box of a size the program chooses, for the program to draw on with lines, boxes and words: a graph, a chart, a diagram. What is drawn is kept, and drawn again whenever the window is, so draw once and again only when something changes. Places are in the picture's own pixels, from its top left corner; what falls outside is not shown. The pointer pressed, dragged and let go in it, and the wheel turned over it, are told with the place. Words are drawn in the picture's typeface, which set_typeface changes.
public class Picture: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a blank picture of the given size in pixels, from 1 to 4096 each way.
    public init(width: Int, height: Int) throws {
        var out = UInt64(0)
        try _check(press_picture_new(Int32(clamping: width), Int32(clamping: height), &out))
        super.init(handle: out)
    }

    /// Rub out everything drawn, leaving it white.
    public func clear() throws {
        try _check(press_picture_clear(handle))
    }

    /// Draw a line one pixel thick from one point to another, both ends included.
    public func line(_ x0: Int, y0: Int, x1: Int, y1: Int, style: LineStyle) throws {
        try _check(press_picture_line(handle, Int32(clamping: x0), Int32(clamping: y0), Int32(clamping: x1), Int32(clamping: y1), style.rawValue))
    }

    /// Draw a box with its top left corner at a point: an outline one pixel thick, or filled in black.
    public func box(_ x: Int, y: Int, width: Int, height: Int, filled: Bool) throws {
        try _check(press_picture_box(handle, Int32(clamping: x), Int32(clamping: y), Int32(clamping: width), Int32(clamping: height), filled))
    }

    /// Fill a box with a pattern of black and white: `pattern` is eight bytes, a row of an 8 by 8 tile each, the leftmost pixel in the top bit, a set bit black and a clear one white. It covers what was there, so eight noughts rub the box out and eight 255s black it in; 170 and 85 by turns are a half-tone grey. The tile is laid from the picture's corner, so boxes side by side in one pattern join without a seam. Lines, outlines and words are always drawn over patterns, whichever was asked for first.
    public func fill(_ x: Int, y: Int, width: Int, height: Int, pattern: [UInt8]) throws {
        try _check(press_picture_fill(handle, Int32(clamping: x), Int32(clamping: y), Int32(clamping: width), Int32(clamping: height), pattern, pattern.count))
    }

    /// Lay down a picture of your own making, a pixel a bit, with its top left corner at a point: `bits` is rows of `width` pixels, each row padded to whole bytes, the leftmost pixel in the top bit of a byte, set for black. It covers what was there, as fill does. For a scene drawn pixel by pixel, such as shading or a game's world, this is one call where dots would be thousands.
    public func image(_ x: Int, y: Int, width: Int, height: Int, bits: [UInt8]) throws {
        try _check(press_picture_image(handle, Int32(clamping: x), Int32(clamping: y), Int32(clamping: width), Int32(clamping: height), bits, bits.count))
    }

    /// Draw what follows in a colour, 0xRRGGBB: lines, boxes outlined and filled, and words. A picture draws in black until told otherwise, and again once it is cleared.
    public func setColor(_ rgb: UInt32) throws {
        try _check(press_picture_set_color(handle, rgb))
    }

    /// Fill a box with one colour, 0xRRGGBB. It covers what was there, as fill does; lines, boxes and words lie on it wherever in the order they were drawn.
    public func fillColor(_ x: Int, y: Int, width: Int, height: Int, rgb: UInt32) throws {
        try _check(press_picture_fill_color(handle, Int32(clamping: x), Int32(clamping: y), Int32(clamping: width), Int32(clamping: height), rgb))
    }

    /// Lay down a picture in colour with its top left corner at a point: `rgb` is rows of `width` pixels, three bytes each, red, green and blue. press is black and white, and this is its one exception, for what is a photograph by nature, such as a camera's picture; offer it in black and white too. It covers what was there, as image does.
    public func photo(_ x: Int, y: Int, width: Int, height: Int, rgb: [UInt8]) throws {
        try _check(press_picture_photo(handle, Int32(clamping: x), Int32(clamping: y), Int32(clamping: width), Int32(clamping: height), rgb, rgb.count))
    }

    /// Draw words in black with the left end of their line at x and its top at y.
    public func text(_ x: Int, y: Int, text: String) throws {
        try _check(press_picture_text(handle, Int32(clamping: x), Int32(clamping: y), text, text.utf8.count))
    }

    /// How wide words would be drawn, in pixels, so that they can be placed: centred under a mark, say, or kept clear of an edge.
    public func textWidth(_ text: String) throws -> Int {
        var out = Int32(0)
        try _check(press_picture_text_width(handle, text, text.utf8.count, &out))
        return Int(out)
    }

    /// How tall a line of words is, in pixels.
    public func lineHeight() throws -> Int {
        var out = Int32(0)
        try _check(press_picture_line_height(handle, &out))
        return Int(out)
    }

    /// Draw words in black in a typeface of their own, whatever the picture's, with the left end of their line at x and its top at y: a heading over plain words, say, or program text in an even face.
    public func textIn(_ x: Int, y: Int, text: String, face: Typeface) throws {
        try _check(press_picture_text_in(handle, Int32(clamping: x), Int32(clamping: y), text, text.utf8.count, face.rawValue))
    }

    /// How wide words would be drawn in a given typeface, in pixels.
    public func textWidthIn(_ text: String, face: Typeface) throws -> Int {
        var out = Int32(0)
        try _check(press_picture_text_width_in(handle, text, text.utf8.count, face.rawValue, &out))
        return Int(out)
    }

    /// How tall a line of words is in a given typeface, in pixels.
    public func lineHeightIn(_ face: Typeface) throws -> Int {
        var out = Int32(0)
        try _check(press_picture_line_height_in(handle, face.rawValue, &out))
        return Int(out)
    }

    /// Change the picture's size, from 1 to 4096 pixels each way. What is drawn is kept; what falls outside is not shown.
    public func setSize(_ width: Int, height: Int) throws {
        try _check(press_picture_set_size(handle, Int32(clamping: width), Int32(clamping: height)))
    }

    /// What the picture shows, in words, for those who cannot see it: "a graph of y = sin x from -10 to 10", say. Screen readers read it out.
    public func setDescription(_ value: String) throws {
        try _check(press_picture_set_description(handle, value, value.utf8.count))
    }

    /// Let the picture have the keyboard, for a game or a drawing worked by keys: Tab stops at it, a press in it takes the keyboard there, and it is ringed while it has it. The arrow keys and their kin are then told by key_pressed, and what is typed by typed.
    public func setTakesKeys(_ value: Bool) throws {
        try _check(press_picture_set_takes_keys(handle, value))
    }

    /// Let the picture hear every key, for a terminal or an emulator of another machine: it takes the keyboard as takes_keys does, and each key is told by key_down with the modifiers held, Tab, Return, Escape, F1 to F12, Page Up and Down, Insert, and Control or Alt with a character among them. Control and Shift with a character still choose a menu entry with that shortcut. What is typed without Control or Alt still comes by typed, and key_pressed is not raised.
    public func setRawKeys(_ value: Bool) throws {
        try _check(press_picture_set_raw_keys(handle, value))
    }

    /// Let the picture hear the right button, for a remote desktop or a game: pressed in the picture, it raises right_pressed, and no context menu pops up there. Where the pointer goes while it is held is told by dragged, and letting it go by right_released.
    public func setRightButton(_ value: Bool) throws {
        try _check(press_picture_set_right_button(handle, value))
    }

    /// Let the picture hear the middle button, as right_button does the right: middle_pressed, where the pointer goes while it is held by dragged, and middle_released.
    public func setMiddleButton(_ value: Bool) throws {
        try _check(press_picture_set_middle_button(handle, value))
    }

    /// The right button was pressed in a picture that hears it (right_button): where, in the picture's pixels. Until it is let go the picture hears where the pointer goes, by dragged, even outside. The handler receives (x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onRightPressed(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_right_pressed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_right_pressed(handle, { _, x, y, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The right button, pressed in the picture, was let go: where, in the picture's pixels, which may be outside it. The handler receives (x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onRightReleased(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_right_released(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_right_released(handle, { _, x, y, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The middle button was pressed in a picture that hears it (middle_button): where, in the picture's pixels. Until it is let go the picture hears where the pointer goes, by dragged, even outside. The handler receives (x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onMiddlePressed(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_middle_pressed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_middle_pressed(handle, { _, x, y, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The middle button, pressed in the picture, was let go: where, in the picture's pixels, which may be outside it. The handler receives (x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onMiddleReleased(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_middle_released(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_middle_released(handle, { _, x, y, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// A key was pressed while a picture that hears every key had the keyboard: its Key value, or 0 for a character key; the character, as a Unicode code point, when Control or Alt was held with one (Control gives it in lower case), and otherwise 0; and the Modifier bits held. The handler receives (key, character, modifiers). A new handler takes the place of the last; pass nil to disconnect.
    public func onKeyDown(_ handler: ((Int, Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_key_down(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int, Int) -> Void>(handler))
        let status = press_picture_on_key_down(handle, { _, key, character, modifiers, userdata in
            _unbox(userdata, as: ((Int, Int, Int) -> Void).self)(Int(key), Int(character), Int(modifiers))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// A picture told to stretch (with set_stretch) was given room of another size by its window, the first time it is laid out included: the width and height it now fills, in pixels. Draw it again for that size; its own size, from new or set_size, is the least it asks for. The handler receives (width, height). A new handler takes the place of the last; pass nil to disconnect.
    public func onResized(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_resized(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_resized(handle, { _, width, height, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(width), Int(height))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// A key that types nothing was pressed while the picture had the keyboard: an arrow, Home, End, Backspace or Delete, as its Key value. Tab, Return and Escape keep their usual work. The handler receives (key). A new handler takes the place of the last; pass nil to disconnect.
    public func onKeyPressed(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_key_pressed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_picture_on_key_pressed(handle, { _, key, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(key))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// Something was typed while the picture had the keyboard: a letter, a figure, a space, or whatever an input method made of the keys. The handler receives (text). A new handler takes the place of the last; pass nil to disconnect.
    public func onTyped(_ handler: ((String) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_typed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(String) -> Void>(handler))
        let status = press_picture_on_typed(handle, { _, text, textLength, userdata in
            _unbox(userdata, as: ((String) -> Void).self)(_text(text, textLength))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The pointer was pressed in the picture: where, and 1 for a single click, 2 for the second of a double click, and so on. Until it is let go the picture hears where it moves, even outside. The handler receives (x, y, clicks). A new handler takes the place of the last; pass nil to disconnect.
    public func onPressed(_ handler: ((Int, Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_pressed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int, Int) -> Void>(handler))
        let status = press_picture_on_pressed(handle, { _, x, y, clicks, userdata in
            _unbox(userdata, as: ((Int, Int, Int) -> Void).self)(Int(x), Int(y), Int(clicks))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The pointer moved while held after a press in the picture. The place may be outside it. The handler receives (x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onDragged(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_dragged(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_dragged(handle, { _, x, y, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The pointer pressed in the picture was let go. The handler receives (x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onReleased(_ handler: ((Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_released(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int) -> Void>(handler))
        let status = press_picture_on_released(handle, { _, x, y, userdata in
            _unbox(userdata, as: ((Int, Int) -> Void).self)(Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }

    /// The wheel was turned over the picture: lines down, or up when negative (three to a notch of an ordinary wheel), and where the pointer was. The handler receives (lines, x, y). A new handler takes the place of the last; pass nil to disconnect.
    public func onScrolled(_ handler: ((Int, Int, Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_picture_on_scrolled(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int, Int, Int) -> Void>(handler))
        let status = press_picture_on_scrolled(handle, { _, lines, x, y, userdata in
            _unbox(userdata, as: ((Int, Int, Int) -> Void).self)(Int(lines), Int(x), Int(y))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// Several pages of widgets in the space of one, with a row of tabs along the top to choose between them. Left and Right turn the pages from the keyboard.
public class TabView: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a tab view with no pages.
    public init() throws {
        var out = UInt64(0)
        try _check(press_tab_view_new(&out))
        super.init(handle: out)
    }

    /// Add a page: the word on its tab, and the widget (usually a Stack) shown when it is chosen.
    public func add(_ title: String, content: Widget?) throws {
        try _check(press_tab_view_add(handle, title, title.utf8.count, content?.handle ?? 0))
    }

    /// The index of the page showing; -1 when there are no pages. Setting it does not raise changed.
    public func setSelected(_ value: Int) throws {
        try _check(press_tab_view_set_selected(handle, Int32(clamping: value)))
    }

    /// The index of the page showing; -1 when there are no pages. Setting it does not raise changed.
    public var selected: Int {
        get throws {
            var out = Int32(0)
            try _check(press_tab_view_get_selected(handle, &out))
            return Int(out)
        }
    }

    /// The user turned to a different page. The handler receives (index). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_tab_view_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_tab_view_on_changed(handle, { _, index, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(index))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A box showing one of a list of choices. Pressing it pops the whole list up over it, with the current choice under the pointer: drag to another and let go, or click and then click. From the keyboard, Space pops the list up.
public class PopupButton: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a pop-up button with no choices.
    public init() throws {
        var out = UInt64(0)
        try _check(press_popup_button_new(&out))
        super.init(handle: out)
    }

    /// Append a choice. The first one added starts as the current choice.
    public func add(_ label: String) throws {
        try _check(press_popup_button_add(handle, label, label.utf8.count))
    }

    /// Remove every choice.
    public func clear() throws {
        try _check(press_popup_button_clear(handle))
    }

    /// The index of the current choice, counting from 0; -1 when there are no choices. Setting it does not raise changed.
    public func setSelected(_ value: Int) throws {
        try _check(press_popup_button_set_selected(handle, Int32(clamping: value)))
    }

    /// The index of the current choice, counting from 0; -1 when there are no choices. Setting it does not raise changed.
    public var selected: Int {
        get throws {
            var out = Int32(0)
            try _check(press_popup_button_get_selected(handle, &out))
            return Int(out)
        }
    }

    /// The user picked a different choice. Picking the current one again raises nothing. The handler receives (index). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_popup_button_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_popup_button_on_changed(handle, { _, index, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(index))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A framed opening onto a widget that may be larger than the space it is given. It has a scroll bar down its right-hand side, and one along the bottom when the content is too wide. A widget inside that takes the keyboard is scrolled into view.
public class ScrollView: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty scroll view, 200 by 120 pixels.
    public init() throws {
        var out = UInt64(0)
        try _check(press_scroll_view_new(&out))
        super.init(handle: out)
    }

    /// Put a widget in the view; usually a Stack of others. It is laid out at the size it asks for, or the size of the opening if that is larger.
    public func setContent(_ content: Widget?) throws {
        try _check(press_scroll_view_set_content(handle, content?.handle ?? 0))
    }

    /// The width the view asks for, frame and scroll bar included, in pixels.
    public func setWidth(_ value: Int) throws {
        try _check(press_scroll_view_set_width(handle, Int32(clamping: value)))
    }

    /// The height the view asks for, frame included, in pixels.
    public func setHeight(_ value: Int) throws {
        try _check(press_scroll_view_set_height(handle, Int32(clamping: value)))
    }

    /// How far the content is scrolled to the left, in pixels. A value past the end is brought back.
    public func setScrollX(_ value: Int) throws {
        try _check(press_scroll_view_set_scroll_x(handle, Int32(clamping: value)))
    }

    /// How far the content is scrolled to the left, in pixels. A value past the end is brought back.
    public var scrollX: Int {
        get throws {
            var out = Int32(0)
            try _check(press_scroll_view_get_scroll_x(handle, &out))
            return Int(out)
        }
    }

    /// How far the content is scrolled up, in pixels. A value past the end is brought back.
    public func setScrollY(_ value: Int) throws {
        try _check(press_scroll_view_set_scroll_y(handle, Int32(clamping: value)))
    }

    /// How far the content is scrolled up, in pixels. A value past the end is brought back.
    public var scrollY: Int {
        get throws {
            var out = Int32(0)
            try _check(press_scroll_view_get_scroll_y(handle, &out))
            return Int(out)
        }
    }
}

/// A scroll bar: an arrow at each end and a box that slides between them. Its value runs from 0 to max. It scrolls nothing by itself; connect changed and move your content. A text area has one of its own.
public class ScrollBar: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a scroll bar with a value of 0, a max of 100 and a page of 10.
    public init(_ orientation: Orientation) throws {
        var out = UInt64(0)
        try _check(press_scroll_bar_new(orientation.rawValue, &out))
        super.init(handle: out)
    }

    /// Where the box is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range.
    public func setValue(_ value: Int) throws {
        try _check(press_scroll_bar_set_value(handle, Int32(clamping: value)))
    }

    /// Where the box is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range.
    public var value: Int {
        get throws {
            var out = Int32(0)
            try _check(press_scroll_bar_get_value(handle, &out))
            return Int(out)
        }
    }

    /// The largest value: how far the content can scroll. With a max of 0 there is nothing to scroll and the bar goes blank.
    public func setMax(_ value: Int) throws {
        try _check(press_scroll_bar_set_max(handle, Int32(clamping: value)))
    }

    /// The largest value: how far the content can scroll. With a max of 0 there is nothing to scroll and the bar goes blank.
    public var max: Int {
        get throws {
            var out = Int32(0)
            try _check(press_scroll_bar_get_max(handle, &out))
            return Int(out)
        }
    }

    /// How much is in view at once, in the units of value. It sets the size of the box, and a click in the track moves by a page less a step.
    public func setPage(_ value: Int) throws {
        try _check(press_scroll_bar_set_page(handle, Int32(clamping: value)))
    }

    /// How far an arrow, or one notch of the wheel's three, moves the value.
    public func setStep(_ value: Int) throws {
        try _check(press_scroll_bar_set_step(handle, Int32(clamping: value)))
    }

    /// The length of the bar in pixels. It is always 16 pixels thick.
    public func setLength(_ value: Int) throws {
        try _check(press_scroll_bar_set_length(handle, Int32(clamping: value)))
    }

    /// The user moved the box, with an arrow, the track, a drag or the wheel. The handler receives (value). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_scroll_bar_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_scroll_bar_on_changed(handle, { _, value, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(value))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// The row of menu titles across the top of a window. Attach it with Window.set_menu_bar.
public class MenuBar: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty menu bar.
    public init() throws {
        var out = UInt64(0)
        try _check(press_menu_bar_new(&out))
        super.init(handle: out)
    }

    /// Append a menu. A menu can be in only one bar.
    public func add(_ menu: Menu?) throws {
        try _check(press_menu_bar_add(handle, menu?.handle ?? 0))
    }
}

/// A pull-down menu. A disabled menu has a greyed title and cannot be pulled down.
public class Menu: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty menu.
    public init(_ title: String) throws {
        var out = UInt64(0)
        try _check(press_menu_new(title, title.utf8.count, &out))
        super.init(handle: out)
    }

    /// Append an item. An item can be in only one menu.
    public func add(_ item: MenuItem?) throws {
        try _check(press_menu_add(handle, item?.handle ?? 0))
    }

    /// Append a dividing line.
    public func addSeparator() throws {
        try _check(press_menu_add_separator(handle))
    }

    /// The word shown in the menu bar.
    public func setTitle(_ value: String) throws {
        try _check(press_menu_set_title(handle, value, value.utf8.count))
    }
}

/// A command in a menu. A disabled item is greyed and cannot be chosen.
public class MenuItem: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a menu item.
    public init(_ label: String) throws {
        var out = UInt64(0)
        try _check(press_menu_item_new(label, label.utf8.count, &out))
        super.init(handle: out)
    }

    /// Hang a submenu off the item. It opens beside the item when the item is pointed at, and the item no longer raises activated. The submenu's own title is not shown. A menu can hang in only one place.
    public func setSubmenu(_ menu: Menu?) throws {
        try _check(press_menu_item_set_submenu(handle, menu?.handle ?? 0))
    }

    /// The item's text.
    public func setLabel(_ value: String) throws {
        try _check(press_menu_item_set_label(handle, value, value.utf8.count))
    }

    /// Show a check mark beside the item. Choosing the item does not change it.
    public func setChecked(_ value: Bool) throws {
        try _check(press_menu_item_set_checked(handle, value))
    }

    /// Show a check mark beside the item. Choosing the item does not change it.
    public var checked: Bool {
        get throws {
            var out = false
            try _check(press_menu_item_get_checked(handle, &out))
            return out
        }
    }

    /// One character that, typed with Control, chooses the item; empty for none. In a text field, Control with A, C, X or V edits the text instead.
    public func setShortcut(_ value: String) throws {
        try _check(press_menu_item_set_shortcut(handle, value, value.utf8.count))
    }

    /// The user chose the item, from the menu or with its shortcut. A new handler takes the place of the last; pass nil to disconnect.
    public func onActivated(_ handler: (() -> Void)?) throws {
        guard let handler else {
            return try _check(press_menu_item_on_activated(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<() -> Void>(handler))
        let status = press_menu_item_on_activated(handle, { _, userdata in
            _unbox(userdata, as: (() -> Void).self)()
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A box with a title, round things that belong together. It is a Stack: every Stack function works on it, and its children are laid out inside the box.
public class GroupBox: Stack {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty box. With an empty title it is a box and no more.
    public init(title: String, orientation: Orientation) throws {
        var out = UInt64(0)
        try _check(press_group_box_new(title, title.utf8.count, orientation.rawValue, &out))
        super.init(handle: out)
    }

    /// The words at the top of the box.
    public func setTitle(_ value: String) throws {
        try _check(press_group_box_set_title(handle, value, value.utf8.count))
    }
}

/// A bar of tools, for the top of a window: a Stack that lays its children out in a row, close together, on a bar drawn by the style. Buttons in it are usually flat.
public class Toolbar: Stack {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty toolbar.
    public init() throws {
        var out = UInt64(0)
        try _check(press_toolbar_new(&out))
        super.init(handle: out)
    }
}

/// A bar of news, for the foot of a window: a Stack that lays its children out in a row on a bar drawn by the style. Labels are what it usually holds.
public class StatusBar: Stack {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty status bar.
    public init() throws {
        var out = UInt64(0)
        try _check(press_status_bar_new(&out))
        super.init(handle: out)
    }

    /// Whether each child is shown in a cell of its own, the last reaching the end of the bar. At first they are all on one strip.
    public func setPanels(_ value: Bool) throws {
        try _check(press_status_bar_set_panels(handle, value))
    }
}

/// The row of buttons at the foot of a dialog: a Stack that lays its children out in a row at its far end, in the order the style of the program has them. Add the button that agrees first, then the one that calls it off, then any others; where the style has OK before Cancel they are shown as added, and where it has Cancel before OK, last first.
public class ButtonRow: Stack {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create an empty row of buttons.
    public init() throws {
        var out = UInt64(0)
        try _check(press_button_row_new(&out))
        super.init(handle: out)
    }
}

/// A whole number with a pair of arrows beside it, one over the other, to step it up and down. An arrow held down steps again and again. The arrow keys step it too when it has the keyboard.
public class Stepper: Widget {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a stepper at 0, which goes from 0 to 100 by ones.
    public init() throws {
        var out = UInt64(0)
        try _check(press_stepper_new(&out))
        super.init(handle: out)
    }

    /// The number. Setting it does not raise changed; a value out of range is brought into range.
    public func setValue(_ value: Int) throws {
        try _check(press_stepper_set_value(handle, Int32(clamping: value)))
    }

    /// The number. Setting it does not raise changed; a value out of range is brought into range.
    public var value: Int {
        get throws {
            var out = Int32(0)
            try _check(press_stepper_get_value(handle, &out))
            return Int(out)
        }
    }

    /// The least the number may be.
    public func setMin(_ value: Int) throws {
        try _check(press_stepper_set_min(handle, Int32(clamping: value)))
    }

    /// The most the number may be.
    public func setMax(_ value: Int) throws {
        try _check(press_stepper_set_max(handle, Int32(clamping: value)))
    }

    /// How far one press of an arrow moves the number, at least 1.
    public func setStep(_ value: Int) throws {
        try _check(press_stepper_set_step(handle, Int32(clamping: value)))
    }

    /// The user stepped the number. The handler receives (value). A new handler takes the place of the last; pass nil to disconnect.
    public func onChanged(_ handler: ((Int) -> Void)?) throws {
        guard let handler else {
            return try _check(press_stepper_on_changed(handle, nil, nil, nil))
        }
        let box = Unmanaged.passRetained(_Box<(Int) -> Void>(handler))
        let status = press_stepper_on_changed(handle, { _, value, userdata in
            _unbox(userdata, as: ((Int) -> Void).self)(Int(value))
        }, box.toOpaque(), _release)
        if status != 0 {
            box.release()
        }
        try _check(status)
    }
}

/// A text field with a list of things it might say, dropped from a button at its end. Choosing one puts it in the field and raises changed; anything else may be typed. It is a TextField: every TextField function and event works on it.
public class ComboBox: TextField {
    public override init(handle: UInt64) {
        super.init(handle: handle)
    }

    /// Create a combo box holding the given text, with no choices yet.
    public override init(_ text: String) throws {
        var out = UInt64(0)
        try _check(press_combo_box_new(text, text.utf8.count, &out))
        super.init(handle: out)
    }

    /// Append a choice to the list.
    public func add(_ choice: String) throws {
        try _check(press_combo_box_add(handle, choice, choice.utf8.count))
    }

    /// Take every choice away. The text in the field stays.
    public func clear() throws {
        try _check(press_combo_box_clear(handle))
    }
}
