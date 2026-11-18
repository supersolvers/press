// SPDX-License-Identifier: GPL-3.0-or-later
// Drawing: a canvas is a picture the program draws on with lines, boxes
// and words: in black and white and the greys made of them, or in colour.
import Foundation
import PressCore

/// Black, white, or a grey made of the two: half and half, or light. Or a
/// colour, for a program that is not keeping to black and white.
public struct Color: Hashable, Sendable {
    /// An 8 by 8 tile, a byte a row, or nil for solid black.
    let pattern: [UInt8]?
    /// A colour, 0xRRGGBB, drawn as it is in place of black or a pattern.
    var rgb: UInt32? = nil

    /// A colour from how much red, green and blue it has, each from 0 to 1.
    public init(red: Double, green: Double, blue: Double) {
        let part = { (v: Double) in UInt32((min(max(v, 0), 1) * 255).rounded()) }
        self.init(pattern: nil, rgb: part(red) << 16 | part(green) << 8 | part(blue))
    }

    init(pattern: [UInt8]?, rgb: UInt32? = nil) {
        self.pattern = pattern
        self.rgb = rgb
    }

    public static let red = Color(pattern: nil, rgb: 0xff3b30)
    public static let orange = Color(pattern: nil, rgb: 0xff9500)
    public static let yellow = Color(pattern: nil, rgb: 0xffcc00)
    public static let green = Color(pattern: nil, rgb: 0x34c759)
    public static let mint = Color(pattern: nil, rgb: 0x00c7be)
    public static let teal = Color(pattern: nil, rgb: 0x30b0c7)
    public static let cyan = Color(pattern: nil, rgb: 0x32ade6)
    public static let blue = Color(pattern: nil, rgb: 0x007aff)
    public static let indigo = Color(pattern: nil, rgb: 0x5856d6)
    public static let purple = Color(pattern: nil, rgb: 0xaf52de)
    public static let pink = Color(pattern: nil, rgb: 0xff2d55)
    public static let brown = Color(pattern: nil, rgb: 0xa2845e)

    public static let black = Color(pattern: nil)
    public static let primary = Color(pattern: nil)
    public static let white = Color(pattern: [UInt8](repeating: 0, count: 8))
    public static let clear = Color(pattern: [UInt8](repeating: 0, count: 8))
    public static let gray = Color(pattern: [170, 85, 170, 85, 170, 85, 170, 85])
    public static let secondary = Color(pattern: [170, 85, 170, 85, 170, 85, 170, 85])
    /// A light grey: one pixel in four black, spaced out.
    public static let lightGray = Color(pattern: [136, 0, 34, 0, 136, 0, 34, 0])
}

/// Lines and boxes to be drawn.
public struct Path: Sendable {
    enum Element: Sendable {
        case move(CGPoint)
        case line(CGPoint)
        case rect(CGRect)
        case close
    }

    var elements: [Element] = []

    public init() {}

    public init(_ rect: CGRect) {
        elements = [.rect(rect)]
    }

    public init(_ build: (inout Path) -> Void) {
        build(&self)
    }

    public mutating func move(to point: CGPoint) {
        elements.append(.move(point))
    }

    public mutating func addLine(to point: CGPoint) {
        elements.append(.line(point))
    }

    public mutating func addLines(_ points: [CGPoint]) {
        for (index, point) in points.enumerated() {
            elements.append(index == 0 ? .move(point) : .line(point))
        }
    }

    public mutating func addRect(_ rect: CGRect) {
        elements.append(.rect(rect))
    }

    public mutating func closeSubpath() {
        elements.append(.close)
    }
}

public struct StrokeStyle: Sendable {
    public var lineWidth: CGFloat
    /// Empty for a solid line; a first length of 1 gives dots and anything
    /// longer gives dashes.
    public var dash: [CGFloat]

    public init(lineWidth: CGFloat = 1, dash: [CGFloat] = []) {
        self.lineWidth = lineWidth
        self.dash = dash
    }
}

/// A place in a box, as parts of its width and height.
public struct UnitPoint: Hashable, Sendable {
    public var x: CGFloat
    public var y: CGFloat

    public init(x: CGFloat, y: CGFloat) {
        self.x = x
        self.y = y
    }

    public static let topLeading = UnitPoint(x: 0, y: 0)
    public static let top = UnitPoint(x: 0.5, y: 0)
    public static let topTrailing = UnitPoint(x: 1, y: 0)
    public static let leading = UnitPoint(x: 0, y: 0.5)
    public static let center = UnitPoint(x: 0.5, y: 0.5)
    public static let trailing = UnitPoint(x: 1, y: 0.5)
    public static let bottomLeading = UnitPoint(x: 0, y: 1)
    public static let bottom = UnitPoint(x: 0.5, y: 1)
    public static let bottomTrailing = UnitPoint(x: 1, y: 1)
}

/// What a canvas's closure draws with.
@MainActor
public struct GraphicsContext {
    let picture: PressCore.Picture

    public struct Shading: Sendable {
        let color: Color

        public static func color(_ color: Color) -> Shading {
            Shading(color: color)
        }
    }

    /// Draw the lines of a path, one pixel thick: in the colour given if
    /// it is a colour, and otherwise in black.
    public func stroke(_ path: Path, with shading: Shading = .color(.black), lineWidth: CGFloat = 1) {
        stroke(path, with: shading, style: StrokeStyle(lineWidth: lineWidth))
    }

    public func stroke(_ path: Path, with shading: Shading = .color(.black), style: StrokeStyle) {
        let line: LineStyle = style.dash.isEmpty ? .solid : (style.dash[0] <= 1 ? .dotted : .dashed)
        var start: CGPoint?
        var current: CGPoint?
        if let rgb = shading.color.rgb {
            try? picture.setColor(rgb)
        }
        defer {
            if shading.color.rgb != nil {
                try? picture.setColor(0)
            }
        }
        func draw(_ from: CGPoint, _ to: CGPoint) {
            try? picture.line(pixels(from.x), y0: pixels(from.y), x1: pixels(to.x), y1: pixels(to.y), style: line)
        }
        for element in path.elements {
            switch element {
            case .move(let point):
                (start, current) = (point, point)
            case .line(let point):
                if let current {
                    draw(current, point)
                }
                current = point
                start = start ?? point
            case .rect(let rect):
                if line == .solid {
                    try? picture.box(
                        pixels(rect.minX), y: pixels(rect.minY), width: pixels(rect.width), height: pixels(rect.height),
                        filled: false)
                } else {
                    let corners = [
                        CGPoint(x: rect.minX, y: rect.minY), CGPoint(x: rect.maxX - 1, y: rect.minY),
                        CGPoint(x: rect.maxX - 1, y: rect.maxY - 1), CGPoint(x: rect.minX, y: rect.maxY - 1),
                    ]
                    for index in 0..<4 {
                        draw(corners[index], corners[(index + 1) % 4])
                    }
                }
            case .close:
                if let current, let start {
                    draw(current, start)
                }
                current = start
            }
        }
    }

    /// Fill the boxes of a path with black, white, a grey or a colour.
    /// Only boxes can be filled.
    public func fill(_ path: Path, with shading: Shading = .color(.black)) {
        for case .rect(let rect) in path.elements {
            let (x, y, width, height) = (pixels(rect.minX), pixels(rect.minY), pixels(rect.width), pixels(rect.height))
            if let rgb = shading.color.rgb {
                try? picture.fillColor(x, y: y, width: width, height: height, rgb: rgb)
            } else if let pattern = shading.color.pattern {
                try? picture.fill(x, y: y, width: width, height: height, pattern: pattern)
            } else {
                try? picture.box(x, y: y, width: width, height: height, filled: true)
            }
        }
    }

    /// Lay down a one-bit picture with its top left corner at a point.
    public func draw(_ image: Image, at point: CGPoint) {
        try? picture.image(pixels(point.x), y: pixels(point.y), width: image.width, height: image.height, bits: image.bits)
    }

    /// Draw words, with a place in the box they take up put at a point:
    /// their middle, unless another anchor is given.
    public func draw(_ text: Text, at point: CGPoint, anchor: UnitPoint = .center) {
        let size = measure(text)
        let (x, y) = (pixels(point.x - size.width * anchor.x), pixels(point.y - size.height * anchor.y))
        if let face = text.font?.typeface {
            try? picture.textIn(x, y: y, text: text.string, face: face)
        } else {
            try? picture.text(x, y: y, text: text.string)
        }
    }

    /// How much room words take when drawn: their width, and the height of
    /// their line, in their own typeface if they have one.
    public func measure(_ text: Text) -> CGSize {
        if let face = text.font?.typeface {
            return CGSize(width: (try? picture.textWidthIn(text.string, face: face)) ?? 0, height: (try? picture.lineHeightIn(face)) ?? 0)
        }
        return CGSize(width: (try? picture.textWidth(text.string)) ?? 0, height: (try? picture.lineHeight()) ?? 0)
    }
}

/// A picture the program draws. It is 200 by 120 pixels unless
/// `.frame(width:height:)` says otherwise, and is drawn again whenever the
/// views are brought up to date. Given a `maxWidth` or `maxHeight` of
/// `.infinity` it grows with its window, its least size the `minWidth` and
/// `minHeight`: the renderer is then given the room the canvas fills, and
/// `onResize` hears when that changes.
public struct Canvas: View, _NodeView {
    public typealias Body = Never
    let renderer: (inout GraphicsContext, CGSize) -> Void
    /// What a click does, by how many clicks it takes.
    var taps: [Int: (CGPoint) -> Void] = [:]
    var drag: DragGesture?
    var description = ""
    var typed: ((String) -> Void)?
    var keyPressed: ((PressCore.Key) -> Void)?
    var scrolled: ((Int, CGPoint) -> Void)?
    var keyDown: ((KeyDown) -> Void)?
    var right: ((ButtonEvent) -> Void)?
    var middle: ((ButtonEvent) -> Void)?
    var resized: ((CGSize) -> Void)?

    public init(renderer: @escaping (inout GraphicsContext, CGSize) -> Void) {
        self.renderer = renderer
    }

    /// Do something when the canvas is clicked, given where: once, or with
    /// a `count` of 2 when it is double-clicked.
    public func onTapGesture(count: Int = 1, perform action: @escaping (CGPoint) -> Void) -> Canvas {
        var copy = self
        copy.taps[count] = action
        return copy
    }

    /// Follow the pointer pressed and moved on the canvas: a drag's
    /// `onChanged` hears it as it goes and `onEnded` when it is let go.
    public func gesture(_ gesture: DragGesture) -> Canvas {
        var copy = self
        copy.drag = gesture
        return copy
    }

    /// What the canvas shows, in words, for those who cannot see it.
    public func accessibilityLabel(_ label: some StringProtocol) -> Canvas {
        var copy = self
        copy.description = String(label)
        return copy
    }

    /// Let the canvas have the keyboard, as for a game or a terminal: Tab
    /// stops at it and a click in it takes the keyboard there. What is
    /// typed goes to `typed`, and the keys that type nothing (the arrows,
    /// Home, End, Backspace, Delete) to `pressed`; Tab, Return and Escape
    /// keep their usual work.
    public func onKeyboard(typed: @escaping (String) -> Void, pressed: @escaping (PressCore.Key) -> Void = { _ in }) -> Canvas {
        var copy = self
        copy.typed = typed
        copy.keyPressed = pressed
        return copy
    }

    /// Let the canvas hear every key, as a terminal or a remote desktop must:
    /// what is typed goes to `typed`, and every other key to `down`, with
    /// the modifiers held: Tab, Return, Escape, the arrows and their kin,
    /// F1 to F12, Page Up and Down, Insert, and Control or Alt with a
    /// character. Control and Shift with a character still choose a menu
    /// entry that has that shortcut, and the Command key keeps the menus.
    public func onKeyboard(typed: @escaping (String) -> Void, down: @escaping (KeyDown) -> Void) -> Canvas {
        var copy = self
        copy.typed = typed
        copy.keyPressed = nil
        copy.keyDown = down
        return copy
    }

    /// Let the canvas have the right button, as a remote desktop or a game
    /// must: it is told when the button goes down, where the pointer goes
    /// while it is held, and when it comes up, and no context menu pops up
    /// over the canvas.
    public func onRightButton(perform action: @escaping (ButtonEvent) -> Void) -> Canvas {
        var copy = self
        copy.right = action
        return copy
    }

    /// Let the canvas have the middle button, as `onRightButton` does the
    /// right.
    public func onMiddleButton(perform action: @escaping (ButtonEvent) -> Void) -> Canvas {
        var copy = self
        copy.middle = action
        return copy
    }

    /// Do something when the wheel is turned over the canvas: given the
    /// lines down (up when negative, three to a notch) and where.
    public func onScroll(perform action: @escaping (Int, CGPoint) -> Void) -> Canvas {
        var copy = self
        copy.scrolled = action
        return copy
    }

    /// Do something when a canvas that grows with its window is given room
    /// of another size, the first time it is laid out included: given the
    /// size it now fills. It is drawn again at that size by itself.
    public func onResize(perform action: @escaping (CGSize) -> Void) -> Canvas {
        var copy = self
        copy.resized = action
        return copy
    }

    func _makeNode(_ env: EnvironmentValues) -> Node { CanvasNode(self, env) }
}

@MainActor
final class CanvasNode: Node {
    private var picture: PressCore.Picture?
    private var size = (200, 120)
    private var built: (Int, Int)?
    private var described = ""
    private var takesKeys = false
    private var hearsAll = false
    private var hearsRight = false
    private var hearsMiddle = false
    /// Whether the canvas grows with its window, as its frame says, and
    /// whether its picture has been told so.
    private var stretches = false
    private var toldStretch = false
    /// The room its window gives a canvas that grows, once known.
    private var room: (Int, Int)?
    /// The right or middle button is down, having gone down in the canvas.
    private var otherHeld: OtherButton?

    private enum OtherButton { case right, middle }

    private func handler(_ button: OtherButton) -> ((ButtonEvent) -> Void)? {
        button == .right ? canvas.right : canvas.middle
    }

    private var canvas: Canvas { view as! Canvas }

    override func update() {
        attempt {
            if picture == nil || built.map({ $0 != size }) ?? true {
                // a picture's size is fixed when it is made
                try? picture?.destroy()
                let fresh = try PressCore.Picture(width: size.0, height: size.1)
                try fresh.onPressed(onMain { [weak self] (x: Int, y: Int, clicks: Int) in self?.pressed(x, y, clicks) })
                try fresh.onDragged(onMain { [weak self] (x: Int, y: Int) in self?.dragged(x, y) })
                try fresh.onReleased(onMain { [weak self] (x: Int, y: Int) in self?.released(x, y) })
                try fresh.onTyped(onMain { [weak self] (text: String) in self?.typed(text) })
                try fresh.onKeyPressed(onMain { [weak self] (key: Int) in self?.keyPressed(key) })
                try fresh.onScrolled(onMain { [weak self] (lines: Int, x: Int, y: Int) in self?.scrolled(lines, x, y) })
                try fresh.onKeyDown(onMain { [weak self] (key: Int, character: Int, modifiers: Int) in self?.keyDown(key, character, modifiers) })
                try fresh.onRightPressed(onMain { [weak self] (x: Int, y: Int) in self?.otherPressed(.right, x, y) })
                try fresh.onRightReleased(onMain { [weak self] (x: Int, y: Int) in self?.otherReleased(.right, x, y) })
                try fresh.onMiddlePressed(onMain { [weak self] (x: Int, y: Int) in self?.otherPressed(.middle, x, y) })
                try fresh.onMiddleReleased(onMain { [weak self] (x: Int, y: Int) in self?.otherReleased(.middle, x, y) })
                try fresh.onResized(onMain { [weak self] (width: Int, height: Int) in self?.resized(width, height) })
                (picture, built, described, takesKeys, hearsAll, hearsRight, hearsMiddle, toldStretch) = (fresh, size, "", false, false, false, false, false)
            }
            guard let picture else { return }
            if toldStretch != stretches {
                toldStretch = stretches
                try picture.setStretch(stretches)
                if !stretches { room = nil }
            }
            let wanted = (keys: canvas.typed != nil || canvas.keyDown != nil, all: canvas.keyDown != nil)
            if (takesKeys, hearsAll) != wanted {
                (takesKeys, hearsAll) = wanted
                try picture.setRawKeys(wanted.all)
                try picture.setTakesKeys(wanted.keys)
            }
            if hearsRight != (canvas.right != nil) {
                hearsRight = canvas.right != nil
                try picture.setRightButton(hearsRight)
            }
            if hearsMiddle != (canvas.middle != nil) {
                hearsMiddle = canvas.middle != nil
                try picture.setMiddleButton(hearsMiddle)
            }
            if described != canvas.description {
                described = canvas.description
                try picture.setDescription(described)
            }
            try picture.clear()
            var context = GraphicsContext(picture: picture)
            // what the closure reads of observable objects is watched here
            let drawn = stretches ? room ?? size : size
            observing(self) { canvas.renderer(&context, CGSize(width: drawn.0, height: drawn.1)) }
        }
    }

    // MARK: The pointer

    /// Where the pointer was pressed, how many clicks it made, and whether
    /// it has moved far enough since to be a drag.
    private var press: (start: CGPoint, clicks: Int, dragging: Bool)?

    private func tap(for clicks: Int) -> ((CGPoint) -> Void)? {
        canvas.taps[clicks] ?? (clicks > 1 ? nil : canvas.taps[1])
    }

    private func pressed(_ x: Int, _ y: Int, _ clicks: Int) {
        guard alive else { return }
        let at = CGPoint(x: x, y: y)
        press = (at, clicks, false)
        // with no drag to tell it from, a click is a tap as soon as it is pressed
        guard let drag = canvas.drag else {
            if let tap = tap(for: clicks) {
                Runtime.shared.event { tap(at) }
            }
            return
        }
        if drag.minimumDistance <= 0 {
            press?.dragging = true
            let value = DragGesture.Value(startLocation: at, location: at, clicks: clicks)
            Runtime.shared.event { drag.changed?(value) }
        }
    }

    private func dragged(_ x: Int, _ y: Int) {
        if let held = otherHeld {
            if alive, let action = handler(held) {
                Runtime.shared.event { action(.moved(CGPoint(x: x, y: y))) }
            }
            return
        }
        guard alive, var held = press, let drag = canvas.drag else { return }
        let at = CGPoint(x: x, y: y)
        if !held.dragging {
            let moved = hypot(at.x - held.start.x, at.y - held.start.y)
            guard moved >= drag.minimumDistance else { return }
            held.dragging = true
            press = held
        }
        let value = DragGesture.Value(startLocation: held.start, location: at, clicks: held.clicks)
        Runtime.shared.event { drag.changed?(value) }
    }

    private func released(_ x: Int, _ y: Int) {
        guard alive, let held = press else { return }
        press = nil
        let at = CGPoint(x: x, y: y)
        guard let drag = canvas.drag else { return }
        if held.dragging {
            let value = DragGesture.Value(startLocation: held.start, location: at, clicks: held.clicks)
            Runtime.shared.event { drag.ended?(value) }
        } else if let tap = tap(for: held.clicks) {
            Runtime.shared.event { tap(held.start) }
        }
    }

    private func otherPressed(_ button: OtherButton, _ x: Int, _ y: Int) {
        guard alive, let action = handler(button) else { return }
        otherHeld = button
        Runtime.shared.event { action(.pressed(CGPoint(x: x, y: y))) }
    }

    private func otherReleased(_ button: OtherButton, _ x: Int, _ y: Int) {
        if otherHeld == button { otherHeld = nil }
        guard alive, let action = handler(button) else { return }
        Runtime.shared.event { action(.released(CGPoint(x: x, y: y))) }
    }

    // MARK: The keyboard and the wheel

    private func typed(_ text: String) {
        guard alive, let typed = canvas.typed else { return }
        Runtime.shared.event { typed(text) }
    }

    private func keyPressed(_ code: Int) {
        guard alive, let pressed = canvas.keyPressed, let key = PressCore.Key(rawValue: Int32(code)) else { return }
        Runtime.shared.event { pressed(key) }
    }

    private func keyDown(_ code: Int, _ character: Int, _ modifiers: Int) {
        guard alive, let down = canvas.keyDown else { return }
        let key = code == 0 ? nil : PressCore.Key(rawValue: Int32(code))
        let typed = character == 0 ? nil : Unicode.Scalar(UInt32(character)).map(Character.init)
        let event = KeyDown(key: key, character: typed, modifiers: PressCore.Modifier(rawValue: UInt32(modifiers)))
        Runtime.shared.event { down(event) }
    }

    private func resized(_ width: Int, _ height: Int) {
        guard alive, stretches, room.map({ $0 != (width, height) }) ?? true else { return }
        room = (width, height)
        update()
        if let resized = canvas.resized {
            Runtime.shared.event { resized(CGSize(width: width, height: height)) }
        }
    }

    private func scrolled(_ lines: Int, _ x: Int, _ y: Int) {
        guard alive, let scrolled = canvas.scrolled else { return }
        Runtime.shared.event { scrolled(lines, CGPoint(x: x, y: y)) }
    }

    override func collect(_ out: inout [Item]) {
        if let picture {
            out.append(Item(widget: picture))
        }
    }

    override func applyFrame(_ frame: FrameSpec) {
        let wanted = (frame.width ?? frame.minWidth ?? size.0, frame.height ?? frame.minHeight ?? size.1)
        let grows = frame.stretch == true
        if (wanted != size && wanted.0 > 0 && wanted.1 > 0) || grows != stretches {
            if wanted.0 > 0, wanted.1 > 0 { size = wanted }
            stretches = grows
            update()
        }
    }

    override func teardown() {
        super.teardown()
        try? picture?.destroy()
    }
}


/// A key pressed on a canvas that hears every key: a key that types
/// nothing, or a character typed with Control or Alt (in lower case with
/// Control), and the modifiers held.
public struct KeyDown: Hashable, Sendable {
    /// The key, or nil for a character key.
    public var key: PressCore.Key?
    public var character: Character?
    public var modifiers: PressCore.Modifier

    public init(key: PressCore.Key?, character: Character?, modifiers: PressCore.Modifier) {
        self.key = key
        self.character = character
        self.modifiers = modifiers
    }
}

/// The right or middle button on a canvas that has it: down, moved while
/// held, and up, each with where the pointer is in the canvas.
public enum ButtonEvent: Hashable, Sendable {
    case pressed(CGPoint)
    case moved(CGPoint)
    case released(CGPoint)
}

/// The name the right button's events had first.
public typealias RightButton = ButtonEvent

/// The pointer pressed and moved on a canvas.
public struct DragGesture {
    public struct Value: Sendable {
        public var startLocation: CGPoint
        public var location: CGPoint
        /// 1 for a drag begun with a single click, 2 begun with a double.
        public var clicks: Int

        public var translation: CGSize {
            CGSize(width: location.x - startLocation.x, height: location.y - startLocation.y)
        }
    }

    /// How far the pointer must move before it is a drag and not a click;
    /// nought for a drag from the moment it is pressed.
    public var minimumDistance: CGFloat
    var changed: ((Value) -> Void)?
    var ended: ((Value) -> Void)?

    public init(minimumDistance: CGFloat = 10) {
        self.minimumDistance = minimumDistance
    }

    public func onChanged(_ action: @escaping (Value) -> Void) -> DragGesture {
        var copy = self
        copy.changed = action
        return copy
    }

    public func onEnded(_ action: @escaping (Value) -> Void) -> DragGesture {
        var copy = self
        copy.ended = action
        return copy
    }
}

/// A one-bit picture for a canvas to draw: rows of `width` pixels, each row
/// padded to whole bytes, the leftmost pixel in the top bit, set for black.
public struct Image: Sendable {
    public let width: Int
    public let height: Int
    public let bits: [UInt8]

    public init(bitmap bits: [UInt8], width: Int, height: Int) {
        self.bits = bits
        self.width = width
        self.height = height
    }
}
