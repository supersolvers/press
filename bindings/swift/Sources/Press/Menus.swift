// SPDX-License-Identifier: GPL-3.0-or-later
// Menus. The items of a menu are written as views (buttons, toggles,
// pickers, dividers, menus inside menus) but they are not widgets in a
// window, so they are not kept as nodes: each time a menu is brought up to
// date its views are read off into a plain description, and the menu's
// items are made to agree with it.
import Foundation
import PressCore

// MARK: - Describing

/// What a view comes to when it is read as an item of a menu, a choice of a
/// picker or a row of a list.
struct ItemSpec {
    enum Kind {
        case text(String)
        case button(String, () -> Void)
        case toggle(String, Binding<Bool>)
        case divider
        case menu(String, [ItemSpec])
    }

    var kind: Kind
    var tag: AnyHashable?
    var enabled: Bool
    var shortcut: KeyboardShortcut?
    var checked = false
    var icon = Icon.none

    init(_ kind: Kind, _ context: DescribeContext) {
        self.kind = kind
        self.tag = context.tag
        self.enabled = context.env.isEnabled
        self.shortcut = context.env.keyboardShortcut
    }

    /// The words of the item, if it has any.
    var title: String? {
        switch kind {
        case .text(let title), .button(let title, _), .toggle(let title, _), .menu(let title, _): return title
        case .divider: return nil
        }
    }
}

/// What has been said, on the way down, about the views being described.
struct DescribeContext {
    var env = EnvironmentValues()
    var tag: AnyHashable?
}

/// A view that can be read as items.
@MainActor
protocol _Describable {
    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext)
}

/// Read a view as items. A view of the program's own is asked for its body;
/// what it keeps with `@State` is not kept here, so state belongs in the
/// view that holds the menu.
@MainActor
func describe(_ view: any View, _ out: inout [ItemSpec], _ context: DescribeContext) {
    if let describable = view as? any _Describable {
        describable._describe(&out, context)
    } else if !(view is any _NodeView) {
        for child in Mirror(reflecting: view).children {
            (child.value as? EnvironmentReader)?.read(context.env)
        }
        describe(bodyOf(view), &out, context)
    }
}

@MainActor
func described(_ view: any View, _ context: DescribeContext = DescribeContext()) -> [ItemSpec] {
    var out: [ItemSpec] = []
    describe(view, &out, context)
    return out
}

/// The first words to be found in a view: a button's caption, say.
@MainActor
func firstText(_ view: any View) -> String? {
    described(view).lazy.compactMap(\.title).first
}

/// An `@Environment` property, which a described view may also read.
protocol EnvironmentReader {
    func read(_ values: EnvironmentValues)
}

extension Environment: EnvironmentReader {
    func read(_ values: EnvironmentValues) {
        box.values = values
    }
}

// MARK: - Menus as views

/// A menu inside a menu: an item that opens more items beside it.
public struct Menu<Content: View>: View, _NodeView, _Describable {
    public typealias Body = Never
    let title: String
    let content: Content

    public init(_ title: some StringProtocol, @ViewBuilder content: () -> Content) {
        self.title = String(title)
        self.content = content()
    }

    /// In a window a menu is nothing: press has menus in the menu bar and
    /// on the right button, and no button that pulls one down.
    func _makeNode(_ env: EnvironmentValues) -> Node { Node(self, env) }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        var inner = context
        inner.tag = nil
        out.append(ItemSpec(.menu(title, described(content, inner)), context))
    }
}

// MARK: - Keeping a menu in step

/// The widgets of some menus, kept agreeing with a description.
@MainActor
final class MenuTree {
    private(set) var menus: [PressCore.Menu] = []
    private var items: [PressCore.MenuItem] = []
    private var actions: [() -> Void] = []
    private var shape = ""

    /// The outline of a description: what kinds of item, in what order. Two
    /// descriptions with the same outline differ only in what can be
    /// changed in place.
    private static func outline(_ specs: [ItemSpec]) -> String {
        specs.map { spec -> String in
            switch spec.kind {
            case .divider: return "-"
            case .menu(_, let items): return "(" + outline(items) + ")"
            default: return "i"
            }
        }.joined()
    }

    /// Make the menus agree with a description whose top level is all
    /// menus. Returns false if they had to be made afresh, in which case
    /// the old ones are gone and the caller must put the new ones in place.
    func update(_ specs: [ItemSpec]) -> Bool {
        let wanted = Self.outline(specs)
        let kept = wanted == shape && !menus.isEmpty
        if !kept {
            for menu in menus {
                try? menu.destroy()
            }
            (menus, items, roots, shape) = ([], [], [], wanted)
        }
        actions = []
        var (menuIndex, itemIndex) = (0, 0)
        attempt {
            for spec in specs {
                guard case .menu(let title, let children) = spec.kind else { continue }
                let menu = try walk(title, children, spec.enabled, &menuIndex, &itemIndex, fresh: !kept)
                if !kept {
                    roots.append(menu)
                }
            }
        }
        return kept
    }

    /// The top-level menus of the last description, in order.
    private(set) var roots: [PressCore.Menu] = []

    private func walk(
        _ title: String, _ children: [ItemSpec], _ enabled: Bool, _ menuIndex: inout Int, _ itemIndex: inout Int,
        fresh: Bool
    ) throws -> PressCore.Menu {
        let menu: PressCore.Menu
        if fresh {
            menu = try PressCore.Menu(title)
            menus.append(menu)
        } else {
            menu = menus[menuIndex]
            try menu.setTitle(title)
        }
        menuIndex += 1
        try menu.setEnabled(enabled)
        for child in children {
            if case .divider = child.kind {
                if fresh {
                    try menu.addSeparator()
                }
                continue
            }
            let item: PressCore.MenuItem
            let index = itemIndex
            itemIndex += 1
            if fresh {
                item = try PressCore.MenuItem(child.title ?? "")
                try item.onActivated(
                    onMain { [weak self] in
                        guard let self, self.actions.indices.contains(index) else { return }
                        let action = self.actions[index]
                        Runtime.shared.event(action)
                    })
                try menu.add(item)
                items.append(item)
            } else {
                item = items[index]
                try item.setLabel(child.title ?? "")
            }
            try item.setEnabled(child.enabled)
            try item.setChecked(child.checked)
            switch child.kind {
            case .button(_, let action):
                actions.append(action)
                try item.setShortcut(child.shortcut?.menuCharacter ?? "")
            case .toggle(_, let binding):
                actions.append { binding.wrappedValue.toggle() }
                try item.setShortcut(child.shortcut?.menuCharacter ?? "")
            case .menu(let title, let inner):
                actions.append {}
                let submenu = try walk(title, inner, child.enabled, &menuIndex, &itemIndex, fresh: fresh)
                if fresh {
                    try item.setSubmenu(submenu)
                }
            case .text, .divider:
                actions.append {}
            }
        }
        return menu
    }

    func destroy() {
        for menu in menus {
            try? menu.destroy()
        }
        (menus, items, actions, shape, roots) = ([], [], [], "", [])
    }
}

// MARK: - Context menus

/// Gives a view a menu on the right button.
public struct _ContextMenuModifier<Content: View, MenuItems: View>: View, _WrapperView {
    public typealias Body = Never
    let content: Content
    let items: MenuItems

    var _content: any View { content }

    func _updated(_ node: WrapperNode, first: Bool) {
        attach(node)
    }

    func _contentChanged(_ node: WrapperNode) {
        attach(node)
    }

    private struct Memory {
        let tree: MenuTree
        var owner: UInt64?
    }

    /// A menu belongs to one widget, so the first of the content's widgets
    /// carries it.
    private func attach(_ node: WrapperNode) {
        var memory = node.memory as? Memory ?? Memory(tree: MenuTree())
        let specs = observing(node) { described(items, DescribeContext(env: node.env)) }
        let kept = memory.tree.update([ItemSpec(.menu("", specs), DescribeContext(env: node.env))])
        let widget = node.items.first?.widget
        if let widget, !kept || memory.owner != widget.handle, let menu = memory.tree.roots.first {
            attempt { try widget.setContextMenu(menu) }
            memory.owner = widget.handle
        }
        node.memory = memory
    }

    func _teardown(_ node: WrapperNode) {
        (node.memory as? Memory)?.tree.destroy()
    }
}

extension View {
    /// Give the view a menu that pops up on the right button, or on the
    /// context-menu key while the view has the keyboard.
    public func contextMenu<MenuItems: View>(@ViewBuilder menuItems: () -> MenuItems) -> some View {
        _ContextMenuModifier(content: self, items: menuItems())
    }
}

// MARK: - Commands

/// The menus of the menu bar.
@MainActor
public protocol Commands {
    associatedtype Body: Commands
    @CommandsBuilder var body: Body { get }
}

extension Never: Commands {}

extension Commands where Body == Never {
    public var body: Never { fatalError() }
}

/// One thing said about the menu bar.
enum CommandSpec {
    case menu(String, any View)
    case group(CommandGroupPlacement, CommandGroupMode, any View)
}

enum CommandGroupMode {
    case before, after, replacing
}

@MainActor
protocol _CommandList {
    func _commands(_ out: inout [CommandSpec])
}

@MainActor
func collectCommands(_ commands: any Commands, _ out: inout [CommandSpec]) {
    if let list = commands as? any _CommandList {
        list._commands(&out)
    } else {
        collectCommands(commandBody(commands), &out)
    }
}

@MainActor
private func commandBody<C: Commands>(_ commands: C) -> any Commands {
    commands.body
}

public struct EmptyCommands: Commands, _CommandList {
    public typealias Body = Never
    public init() {}
    func _commands(_ out: inout [CommandSpec]) {}
}

public struct _TupleCommands<each C: Commands>: Commands, _CommandList {
    public typealias Body = Never
    let value: (repeat each C)

    func _commands(_ out: inout [CommandSpec]) {
        repeat collectCommands(each value, &out)
    }
}

@MainActor
@resultBuilder
public enum CommandsBuilder {
    public static func buildBlock() -> EmptyCommands { EmptyCommands() }
    public static func buildBlock<C: Commands>(_ commands: C) -> C { commands }
    public static func buildBlock<each C: Commands>(_ commands: repeat each C) -> _TupleCommands<repeat each C> {
        _TupleCommands(value: (repeat each commands))
    }

    /// Commands under an `if`: there only while it holds.
    public static func buildOptional<C: Commands>(_ commands: C?) -> _OptionalCommands<C> {
        _OptionalCommands(value: commands)
    }

    public static func buildEither<T: Commands, F: Commands>(first: T) -> _EitherCommands<T, F> {
        _EitherCommands(first: first, second: nil)
    }

    public static func buildEither<T: Commands, F: Commands>(second: F) -> _EitherCommands<T, F> {
        _EitherCommands(first: nil, second: second)
    }
}

public struct _OptionalCommands<C: Commands>: Commands, _CommandList {
    public typealias Body = Never
    let value: C?

    func _commands(_ out: inout [CommandSpec]) {
        if let value { collectCommands(value, &out) }
    }
}

public struct _EitherCommands<T: Commands, F: Commands>: Commands, _CommandList {
    public typealias Body = Never
    let first: T?
    let second: F?

    func _commands(_ out: inout [CommandSpec]) {
        if let first { collectCommands(first, &out) }
        if let second { collectCommands(second, &out) }
    }
}

/// A menu of the program's own, put in the bar after Edit.
public struct CommandMenu<Content: View>: Commands, _CommandList {
    public typealias Body = Never
    let name: String
    let content: Content

    public init(_ name: some StringProtocol, @ViewBuilder content: () -> Content) {
        self.name = String(name)
        self.content = content()
    }

    func _commands(_ out: inout [CommandSpec]) {
        out.append(.menu(name, content))
    }
}

/// A place in the standard menus. press gives every window a File menu
/// with Quit, and an Edit menu that cuts, copies and pastes in whichever
/// field has the keyboard.
public struct CommandGroupPlacement: Hashable, Sendable {
    let menu: String
    let order: Int

    // File
    public static let newItem = CommandGroupPlacement(menu: "File", order: 0)
    public static let saveItem = CommandGroupPlacement(menu: "File", order: 1)
    public static let importExport = CommandGroupPlacement(menu: "File", order: 2)
    public static let printItem = CommandGroupPlacement(menu: "File", order: 3)
    public static let appSettings = CommandGroupPlacement(menu: "File", order: 4)
    /// Quit.
    public static let appTermination = CommandGroupPlacement(menu: "File", order: 5)
    // Edit
    public static let undoRedo = CommandGroupPlacement(menu: "Edit", order: 0)
    /// Cut, Copy and Paste.
    public static let pasteboard = CommandGroupPlacement(menu: "Edit", order: 1)
    /// Select All.
    public static let textEditing = CommandGroupPlacement(menu: "Edit", order: 2)
    // View
    public static let toolbar = CommandGroupPlacement(menu: "View", order: 0)
    public static let sidebar = CommandGroupPlacement(menu: "View", order: 1)
    // Window
    public static let windowSize = CommandGroupPlacement(menu: "Window", order: 0)
    public static let windowArrangement = CommandGroupPlacement(menu: "Window", order: 1)
    // Help
    public static let help = CommandGroupPlacement(menu: "Help", order: 0)
    public static let appInfo = CommandGroupPlacement(menu: "Help", order: 1)
}

/// Commands put before, after or in place of a group of the standard ones.
public struct CommandGroup<Content: View>: Commands, _CommandList {
    public typealias Body = Never
    let placement: CommandGroupPlacement
    let mode: CommandGroupMode
    let content: Content

    public init(before group: CommandGroupPlacement, @ViewBuilder addition: () -> Content) {
        (placement, mode, content) = (group, .before, addition())
    }

    public init(after group: CommandGroupPlacement, @ViewBuilder addition: () -> Content) {
        (placement, mode, content) = (group, .after, addition())
    }

    public init(replacing group: CommandGroupPlacement, @ViewBuilder addition: () -> Content) {
        (placement, mode, content) = (group, .replacing, addition())
    }

    func _commands(_ out: inout [CommandSpec]) {
        out.append(.group(placement, mode, content))
    }
}

/// What a text field can be asked to do from the Edit menu.
enum EditAction {
    case cut, copy, paste, selectAll
}

/// The description of a window's whole menu bar: the standard menus, with
/// what the program said about them, and the program's own between Edit
/// and Window.
@MainActor
func menuBar(_ commands: [CommandSpec], env: EnvironmentValues, edit: @escaping (EditAction) -> Void, quit: @escaping () -> Void) -> [ItemSpec] {
    let context = DescribeContext(env: env)
    func command(_ title: String, _ key: KeyEquivalent, _ action: @escaping () -> Void) -> ItemSpec {
        var spec = ItemSpec(.button(title, action), context)
        spec.shortcut = KeyboardShortcut(key)
        return spec
    }
    var groups: [CommandGroupPlacement: [ItemSpec]] = [
        .appTermination: [command("Quit", "q", quit)],
        .pasteboard: [
            command("Cut", "x") { edit(.cut) }, command("Copy", "c") { edit(.copy) }, command("Paste", "v") { edit(.paste) },
        ],
        .textEditing: [command("Select All", "a") { edit(.selectAll) }],
    ]
    var custom: [ItemSpec] = []
    for spec in commands {
        switch spec {
        case .menu(let name, let content):
            custom.append(ItemSpec(.menu(name, described(content, context)), context))
        case .group(let placement, let mode, let content):
            let items = described(content, context)
            switch mode {
            case .before: groups[placement] = items + (groups[placement] ?? [])
            case .after: groups[placement] = (groups[placement] ?? []) + items
            case .replacing: groups[placement] = items
            }
        }
    }
    func standard(_ name: String) -> [ItemSpec] {
        let parts = groups.filter { $0.key.menu == name && !$0.value.isEmpty }.sorted { $0.key.order < $1.key.order }
        guard !parts.isEmpty else { return [] }
        var items: [ItemSpec] = []
        for part in parts {
            if !items.isEmpty {
                items.append(ItemSpec(.divider, context))
            }
            items.append(contentsOf: part.value)
        }
        return [ItemSpec(.menu(name, items), context)]
    }
    return standard("File") + standard("Edit") + standard("View") + custom + standard("Window") + standard("Help")
}
