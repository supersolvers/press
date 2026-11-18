// SPDX-License-Identifier: GPL-3.0-or-later
// What can be said about a view after it: `.padding()`, `.disabled(true)`,
// `.font(.caption)`, `.onAppear { }` and the rest. Each wraps the view in
// another, as in SwiftUI.
import Foundation
import PressCore
import Observation

// MARK: - The machinery

/// A view that wraps one other and changes something about it.
@MainActor
protocol _WrapperView: View, _NodeView {
    var _content: any View { get }
    /// Change what is handed down to the content.
    func _environment(_ env: inout EnvironmentValues)
    /// Say something about each widget the content gives.
    func _item(_ item: inout Item)
    /// The content has just been brought up to date.
    func _updated(_ node: WrapperNode, first: Bool)
    /// Something inside gives different widgets from before.
    func _contentChanged(_ node: WrapperNode)
    func _teardown(_ node: WrapperNode)
}

extension _WrapperView {
    func _makeNode(_ env: EnvironmentValues) -> Node { WrapperNode(self, env) }
    func _environment(_ env: inout EnvironmentValues) {}
    func _item(_ item: inout Item) {}
    func _updated(_ node: WrapperNode, first: Bool) {}
    func _contentChanged(_ node: WrapperNode) {}
    func _teardown(_ node: WrapperNode) {}
}

@MainActor
final class WrapperNode: Node {
    /// Whatever the wrapper needs to remember.
    var memory: Any?
    private var first = true

    private var wrapper: any _WrapperView { view as! any _WrapperView }

    override func update() {
        var inner = env
        wrapper._environment(&inner)
        reconcileChildren([wrapper._content], inner)
        wrapper._updated(self, first: first)
        first = false
    }

    override func collect(_ out: inout [Item]) {
        var mine: [Item] = []
        super.collect(&mine)
        for index in mine.indices {
            wrapper._item(&mine[index])
        }
        out.append(contentsOf: mine)
    }

    override func contentChanged() {
        wrapper._contentChanged(self)
        super.contentChanged()
    }

    override func teardown() {
        wrapper._teardown(self)
        super.teardown()
    }
}

// MARK: - Environment

/// Changes what is handed down to a view.
public struct _EnvironmentModifier<Content: View>: View, _WrapperView, _Describable {
    public typealias Body = Never
    let content: Content
    let change: (inout EnvironmentValues) -> Void

    var _content: any View { content }

    func _environment(_ env: inout EnvironmentValues) {
        change(&env)
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        var context = context
        change(&context.env)
        describe(content, &out, context)
    }
}

extension View {
    func transformingEnvironment(_ change: @escaping (inout EnvironmentValues) -> Void) -> _EnvironmentModifier<Self> {
        _EnvironmentModifier(content: self, change: change)
    }

    /// Hand a value down to this view and everything inside it.
    public func environment<V>(_ keyPath: WritableKeyPath<EnvironmentValues, V>, _ value: V) -> some View {
        transformingEnvironment { $0[keyPath: keyPath] = value }
    }

    /// Hand an observable object down, to be read with `@Environment(T.self)`.
    public func environment<T: AnyObject & Observable>(_ object: T?) -> some View {
        transformingEnvironment { $0[object: T.self] = object }
    }

    /// Grey the view and everything inside it, and have it ignore the user.
    public func disabled(_ disabled: Bool) -> some View {
        transformingEnvironment { $0.isEnabled = $0.isEnabled && !disabled }
    }

    /// Set the words of this view and everything inside it in a typeface.
    public func font(_ font: Font?) -> some View {
        transformingEnvironment { $0.font = font }
    }

    /// Set the words in the heavy cut of their typeface, where it has one.
    public func bold(_ isActive: Bool = true) -> some View {
        transformingEnvironment { if isActive { $0.font = ($0.font ?? .body).bold() } }
    }

    /// Set the words in the typeface whose characters are all one width.
    public func monospaced(_ isActive: Bool = true) -> some View {
        transformingEnvironment { if isActive { $0.font = ($0.font ?? .body).monospaced() } }
    }

    /// Give the buttons inside a key: `.defaultAction` is Return and
    /// `.cancelAction` is Escape. On a button in a menu, the character is
    /// typed with Control.
    public func keyboardShortcut(_ shortcut: KeyboardShortcut?) -> some View {
        transformingEnvironment { $0.keyboardShortcut = shortcut }
    }

    public func keyboardShortcut(_ key: KeyEquivalent, modifiers: EventModifiers = .command) -> some View {
        keyboardShortcut(KeyboardShortcut(key, modifiers: modifiers))
    }

    /// Do something when the user presses Return in a text field inside, or
    /// opens a row of a list.
    public func onSubmit(_ action: @escaping () -> Void) -> some View {
        transformingEnvironment { env in
            let outer = env.submitAction
            env.submitAction = {
                action()
                outer?()
            }
        }
    }

    /// How the pickers inside are shown.
    public func pickerStyle(_ style: PickerStyle) -> some View {
        transformingEnvironment { $0.pickerStyle = style }
    }
}

private enum IsEnabledKey: EnvironmentKey { static let defaultValue = true }
private enum FontKey: EnvironmentKey { static let defaultValue: Font? = nil }
private enum ShortcutKey: EnvironmentKey { static let defaultValue: KeyboardShortcut? = nil }
private enum SubmitKey: EnvironmentKey { static let defaultValue: (() -> Void)? = nil }
private enum PickerStyleKey: EnvironmentKey { static let defaultValue = PickerStyle.menu }

extension EnvironmentValues {
    /// Whether the view answers the user.
    public var isEnabled: Bool {
        get { self[IsEnabledKey.self] }
        set { self[IsEnabledKey.self] = newValue }
    }

    public var font: Font? {
        get { self[FontKey.self] }
        set { self[FontKey.self] = newValue }
    }

    var keyboardShortcut: KeyboardShortcut? {
        get { self[ShortcutKey.self] }
        set { self[ShortcutKey.self] = newValue }
    }

    var submitAction: (() -> Void)? {
        get { self[SubmitKey.self] }
        set { self[SubmitKey.self] = newValue }
    }

    var pickerStyle: PickerStyle {
        get { self[PickerStyleKey.self] }
        set { self[PickerStyleKey.self] = newValue }
    }
}

// MARK: - Typefaces

/// A typeface. press has six, each drawn for one size, so the styles of
/// SwiftUI are shared out among them.
public struct Font: Hashable, Sendable {
    let typeface: Typeface

    /// One of press's own typefaces by name.
    public static func press(_ typeface: Typeface) -> Font { Font(typeface: typeface) }

    /// Cicero: heavy and plain, the face of the interface.
    public static let body = Font(typeface: .cicero)
    public static let headline = Font(typeface: .cicero)
    public static let title3 = Font(typeface: .cicero)
    /// Elite: heavy and large, every character one width.
    public static let largeTitle = Font(typeface: .elite)
    public static let title = Font(typeface: .elite)
    public static let title2 = Font(typeface: .elite)
    /// Brevier: light and airy, for reading.
    public static let callout = Font(typeface: .brevier)
    /// Agate made heavy.
    public static let subheadline = Font(typeface: .agateBold)
    /// Agate: small and light, for a lot of words in a little room.
    public static let footnote = Font(typeface: .agate)
    public static let caption = Font(typeface: .agate)
    public static let caption2 = Font(typeface: .agate)

    public enum TextStyle: Sendable {
        case largeTitle, title, title2, title3, headline, subheadline, body, callout, footnote, caption, caption2
    }

    public enum Design: Sendable {
        case `default`, serif, rounded, monospaced
    }

    public static func system(_ style: TextStyle = .body, design: Design? = nil) -> Font {
        let font: Font
        switch style {
        case .largeTitle, .title, .title2: font = .title
        case .title3, .headline, .body: font = .body
        case .subheadline: font = .subheadline
        case .callout: font = .callout
        case .footnote, .caption, .caption2: font = .caption
        }
        return design == .monospaced ? font.monospaced() : font
    }

    /// Pica, or Elite for a title: every character one width.
    public func monospaced() -> Font {
        typeface == .elite ? self : Font(typeface: .pica)
    }

    public func bold() -> Font {
        switch typeface {
        case .agate: return Font(typeface: .agateBold)
        case .brevier: return Font(typeface: .cicero)
        case .pica: return Font(typeface: .elite)
        default: return self
        }
    }
}

// MARK: - Keys

/// A key that with Control chooses a command.
public struct KeyEquivalent: Hashable, Sendable, ExpressibleByExtendedGraphemeClusterLiteral {
    public let character: Character

    public init(_ character: Character) {
        self.character = character
    }

    public init(extendedGraphemeClusterLiteral value: Character) {
        self.character = value
    }

    public static let `return` = KeyEquivalent("\r")
    public static let escape = KeyEquivalent("\u{1B}")
}

/// Keys held with another. press has one command key, Control, and menu
/// shortcuts are always typed with it, so these are accepted and not used.
public struct EventModifiers: OptionSet, Hashable, Sendable {
    public let rawValue: Int

    public init(rawValue: Int) {
        self.rawValue = rawValue
    }

    public static let shift = EventModifiers(rawValue: 1)
    public static let control = EventModifiers(rawValue: 2)
    public static let option = EventModifiers(rawValue: 4)
    public static let command = EventModifiers(rawValue: 8)
}

public struct KeyboardShortcut: Hashable, Sendable {
    public let key: KeyEquivalent
    public let modifiers: EventModifiers

    public init(_ key: KeyEquivalent, modifiers: EventModifiers = .command) {
        self.key = key
        self.modifiers = modifiers
    }

    /// Return: the button with the heavy outline.
    public static let defaultAction = KeyboardShortcut(.return, modifiers: [])
    /// Escape.
    public static let cancelAction = KeyboardShortcut(.escape, modifiers: [])

    /// What a menu item is told: one character, shown in capitals.
    var menuCharacter: String {
        self == .defaultAction || self == .cancelAction ? "" : String(key.character).uppercased()
    }
}

// MARK: - Tags and tabs

/// Says something about the widgets of a view, for whatever holds them.
public struct _TraitModifier<Content: View>: View, _WrapperView, _Describable {
    public typealias Body = Never
    let content: Content
    let tag: AnyHashable?
    let tabTitle: String?

    var _content: any View { content }

    func _item(_ item: inout Item) {
        if let tag {
            item.tag = tag
        }
        if let tabTitle {
            item.tabTitle = tabTitle
        }
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        var context = context
        if let tag {
            context.tag = tag
        }
        describe(content, &out, context)
    }
}

extension View {
    /// The value a picker, a list or a tab view takes when this is chosen.
    public func tag<V: Hashable>(_ tag: V) -> some View {
        _TraitModifier(content: self, tag: hashable(tag), tabTitle: nil)
    }

    /// The word on this view's tab in a tab view. Only its words are used.
    public func tabItem<Label: View>(@ViewBuilder _ label: () -> Label) -> some View {
        _TraitModifier(content: self, tag: nil, tabTitle: firstText(label()) ?? "")
    }
}

/// A value as a tag, with an optional taken out of its wrapping so that a
/// selection of `Int?` matches a tag of `Int`.
func hashable<V: Hashable>(_ value: V) -> AnyHashable? {
    if let optional = value as? any _AnyOptional {
        return optional._unwrapped
    }
    return AnyHashable(value)
}

protocol _AnyOptional {
    var _unwrapped: AnyHashable? { get }
}

extension Optional: _AnyOptional where Wrapped: Hashable {
    var _unwrapped: AnyHashable? { map { AnyHashable($0) } }
}

// MARK: - Size

/// Asks the widgets of a view to be a size.
public struct _FrameModifier<Content: View>: View, _WrapperView {
    public typealias Body = Never
    let content: Content
    let frame: FrameSpec

    var _content: any View { content }

    func _updated(_ node: WrapperNode, first: Bool) {
        node.children.first?.applyFrame(frame)
    }

    func _contentChanged(_ node: WrapperNode) {
        node.children.first?.applyFrame(frame)
    }
}

extension View {
    /// Ask for a size in pixels. A widget is never smaller than what it
    /// shows needs, so this can make one larger and not smaller.
    public func frame(width: CGFloat? = nil, height: CGFloat? = nil) -> some View {
        _FrameModifier(content: self, frame: FrameSpec(width: width.map(pixels), height: height.map(pixels)))
    }

    /// Ask for a least size, and with a `maxWidth` or `maxHeight` of
    /// `.infinity` for the view to take up spare room in its stack.
    public func frame(
        minWidth: CGFloat? = nil, maxWidth: CGFloat? = nil, minHeight: CGFloat? = nil, maxHeight: CGFloat? = nil
    ) -> some View {
        let grows = maxWidth == .infinity || maxHeight == .infinity
        return _FrameModifier(
            content: self,
            frame: FrameSpec(minWidth: minWidth.map(pixels), minHeight: minHeight.map(pixels), stretch: grows ? true : nil))
    }

    /// Keep the view at the size it needs, however much room there is.
    public func fixedSize() -> some View {
        _FrameModifier(content: self, frame: FrameSpec(stretch: false))
    }

    /// Put a margin round the view, the same on every side.
    public func padding(_ length: CGFloat = 16) -> some View {
        _PaddingView(content: self, padding: pixels(length))
    }
}

func pixels(_ value: CGFloat) -> Int {
    value.isFinite ? Int(value.rounded()) : 0
}

// MARK: - Coming and going

/// Does something when a view arrives, leaves, or sees a value change.
/// Gives a view the keyboard when asked.
public struct _FocusModifier<Content: View>: View, _WrapperView {
    public typealias Body = Never
    let content: Content
    let wanted: Binding<Bool>

    var _content: any View { content }

    func _updated(_ node: WrapperNode, first: Bool) {
        guard wanted.wrappedValue else { return }
        var items: [Item] = []
        node.collect(&items)
        guard let widget = items.first?.widget else { return }
        let wanted = self.wanted
        // after this update, when a view just shown is in its window
        Task { @MainActor in
            try? widget.focus()
            Runtime.shared.event { wanted.wrappedValue = false }
        }
    }
}

extension View {
    /// Give this view the keyboard whenever `isFocused` is set true; it is
    /// set back to false once the view has it. A find field shown by a
    /// command, say, takes the typing that follows at once.
    public func focused(_ isFocused: Binding<Bool>) -> some View {
        _FocusModifier(content: self, wanted: isFocused)
    }
}

public struct _LifecycleModifier<Content: View>: View, _WrapperView {
    public typealias Body = Never
    let content: Content
    var appear: (() -> Void)?
    var disappear: (() -> Void)?
    var task: (@MainActor @Sendable () async -> Void)?
    /// The value watched, and what to do when it is no longer the same.
    var watched: AnyHashable?
    var changed: ((AnyHashable, AnyHashable) -> Void)?
    var initial = false

    var _content: any View { content }

    func _updated(_ node: WrapperNode, first: Bool) {
        if first {
            appear?()
            if let task {
                node.memory = Task { @MainActor in await task() }
            }
        }
        if let changed, let watched {
            let before = node.memory as? AnyHashable
            node.memory = watched
            if let before, before != watched {
                changed(before, watched)
            } else if before == nil, initial {
                changed(watched, watched)
            }
        }
    }

    func _teardown(_ node: WrapperNode) {
        (node.memory as? Task<Void, Never>)?.cancel()
        disappear?()
    }
}

extension View {
    /// Do something when the view first comes into a window.
    public func onAppear(perform action: (() -> Void)? = nil) -> some View {
        _LifecycleModifier(content: self, appear: action)
    }

    /// Do something when the view leaves its window.
    public func onDisappear(perform action: (() -> Void)? = nil) -> some View {
        _LifecycleModifier(content: self, disappear: action)
    }

    /// Start a piece of asynchronous work when the view comes into a
    /// window, and cancel it when the view leaves. It runs on the main
    /// actor, so it may change the view's state as a handler does.
    public func task(_ action: @escaping @MainActor @Sendable () async -> Void) -> some View {
        _LifecycleModifier(content: self, task: action)
    }

    /// Do something when a value is no longer what it was.
    public func onChange<V: Hashable>(of value: V, initial: Bool = false, _ action: @escaping (V, V) -> Void) -> some View {
        _LifecycleModifier(
            content: self, watched: AnyHashable(value),
            changed: { old, new in
                if let old = old.base as? V, let new = new.base as? V {
                    action(old, new)
                }
            }, initial: initial)
    }

    public func onChange<V: Hashable>(of value: V, initial: Bool = false, _ action: @escaping () -> Void) -> some View {
        onChange(of: value, initial: initial) { _, _ in action() }
    }
}

// MARK: - The window's title

/// Names the window a view is in.
public struct _TitleModifier<Content: View>: View, _WrapperView {
    public typealias Body = Never
    let content: Content
    let title: String

    var _content: any View { content }

    func _updated(_ node: WrapperNode, first: Bool) {
        node.windowNode?.setTitle(title)
    }
}

extension View {
    /// The title of the window this view is in.
    public func navigationTitle(_ title: some StringProtocol) -> some View {
        _TitleModifier(content: self, title: String(title))
    }
}

public struct _FullScreenModifier<Content: View>: View, _WrapperView {
    public typealias Body = Never
    let content: Content
    let isOn: Binding<Bool>

    var _content: any View { content }

    func _updated(_ node: WrapperNode, first: Bool) {
        let isOn = self.isOn
        // read where it is watched, so that the window follows the binding
        let wanted = observing(node) { isOn.wrappedValue }
        node.windowNode?.setFullScreen(wanted) { on in
            if isOn.wrappedValue != on { isOn.wrappedValue = on }
        }
    }
}

extension View {
    /// Whether the window this view is in fills its screen, with no title
    /// bar or frame, as a film does. Setting the binding asks the desktop;
    /// it is set back if the desktop refuses, or when the user leaves full
    /// screen by the desktop's own means.
    public func fullScreen(isOn: Binding<Bool>) -> some View {
        _FullScreenModifier(content: self, isOn: isOn)
    }
}

// MARK: - Modifiers of a program's own

/// A change to a view that a program writes once and uses on many.
@MainActor
public protocol ViewModifier {
    associatedtype Body: View
    typealias Content = _ViewModifier_Content<Self>
    @ViewBuilder func body(content: Content) -> Body
}

/// The view a modifier is being used on.
public struct _ViewModifier_Content<Modifier: ViewModifier>: View, _NodeView, _Describable {
    public typealias Body = Never
    let view: any View

    func _makeNode(_ env: EnvironmentValues) -> Node { ListNode(self, env) { [($0 as! Self).view] } }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        describe(view, &out, context)
    }
}

/// A view with a modifier used on it.
public struct ModifiedContent<Content: View, Modifier: ViewModifier>: View {
    public var content: Content
    public var modifier: Modifier

    public init(content: Content, modifier: Modifier) {
        self.content = content
        self.modifier = modifier
    }

    public var body: some View {
        modifier.body(content: _ViewModifier_Content(view: content))
    }
}

extension View {
    public func modifier<M: ViewModifier>(_ modifier: M) -> ModifiedContent<Self, M> {
        ModifiedContent(content: self, modifier: modifier)
    }
}
