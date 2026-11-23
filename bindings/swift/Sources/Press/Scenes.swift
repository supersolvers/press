// SPDX-License-Identifier: GPL-3.0-or-later
// A program and its windows: `App`, `Scene`, `WindowGroup`, and the loop
// that keeps them going.
import Dispatch
import Foundation
import PressCore

// MARK: - Scenes

/// A part of a program that is shown in windows.
@MainActor
public protocol Scene {
    associatedtype Body: Scene
    @SceneBuilder var body: Body { get }
}

extension Never: Scene {}

extension Scene where Body == Never {
    public var body: Never { fatalError() }
}

/// How a window may be resized.
public struct WindowResizability: Hashable, Sendable {
    let resizable: Bool

    public static let automatic = WindowResizability(resizable: true)
    public static let contentMinSize = WindowResizability(resizable: true)
    /// The window is the size of its content and stays so.
    public static let contentSize = WindowResizability(resizable: false)
}

/// What a scene comes to: a window that can be opened.
struct WindowSpec {
    var title: String
    var id: String?
    var content: any View
    /// Whether more than one may be open at once.
    var many: Bool
    var size: (Int, Int)?
    var resizable = true
    var tag: String?
    var commands: [CommandSpec] = []
}

@MainActor
protocol _SceneList {
    func _scenes(_ out: inout [WindowSpec])
}

@MainActor
func collectScenes(_ scene: any Scene, _ out: inout [WindowSpec]) {
    if let list = scene as? any _SceneList {
        list._scenes(&out)
    } else {
        collectScenes(sceneBody(scene), &out)
    }
}

@MainActor
private func sceneBody<S: Scene>(_ scene: S) -> any Scene {
    scene.body
}

@MainActor
private func sceneOf<A: App>(_ app: A) -> any Scene {
    app.body
}

/// Windows of one kind. The first scene of a program is opened when it
/// starts; the others wait for `openWindow`.
public struct WindowGroup<Content: View>: Scene, _SceneList {
    public typealias Body = Never
    let title: String
    let id: String?
    let content: Content

    public init(_ title: some StringProtocol, id: String? = nil, @ViewBuilder content: () -> Content) {
        self.title = String(title)
        self.id = id
        self.content = content()
    }

    public init(id: String? = nil, @ViewBuilder content: () -> Content) {
        self.title = ProcessInfo.processInfo.processName
        self.id = id
        self.content = content()
    }

    func _scenes(_ out: inout [WindowSpec]) {
        out.append(WindowSpec(title: title, id: id, content: content, many: true))
    }
}

/// A window of which there is only ever one.
public struct Window<Content: View>: Scene, _SceneList {
    public typealias Body = Never
    let title: String
    let id: String
    let content: Content

    public init(_ title: some StringProtocol, id: String, @ViewBuilder content: () -> Content) {
        self.title = String(title)
        self.id = id
        self.content = content()
    }

    func _scenes(_ out: inout [WindowSpec]) {
        out.append(WindowSpec(title: title, id: id, content: content, many: false))
    }
}

public struct _TupleScene<each S: Scene>: Scene, _SceneList {
    public typealias Body = Never
    let value: (repeat each S)

    func _scenes(_ out: inout [WindowSpec]) {
        repeat collectScenes(each value, &out)
    }
}

/// A scene with something said about its windows.
public struct _ModifiedScene<Content: Scene>: Scene, _SceneList {
    public typealias Body = Never
    let content: Content
    let change: @MainActor (inout WindowSpec) -> Void

    func _scenes(_ out: inout [WindowSpec]) {
        var mine: [WindowSpec] = []
        collectScenes(content, &mine)
        for index in mine.indices {
            change(&mine[index])
        }
        out.append(contentsOf: mine)
    }
}

@MainActor
@resultBuilder
public enum SceneBuilder {
    public static func buildBlock<S: Scene>(_ scene: S) -> S { scene }
    public static func buildBlock<each S: Scene>(_ scene: repeat each S) -> _TupleScene<repeat each S> {
        _TupleScene(value: (repeat each scene))
    }
}

extension Scene {
    /// Add to the menu bar. The menus are those of every window.
    public func commands<C: Commands>(@CommandsBuilder content: () -> C) -> some Scene {
        var specs: [CommandSpec] = []
        collectCommands(content(), &specs)
        return _ModifiedScene(content: self) { $0.commands += specs }
    }

    /// The size of the window's content when it opens, in pixels. Without
    /// this a window opens at the size its content needs.
    public func defaultSize(width: CGFloat, height: CGFloat) -> some Scene {
        _ModifiedScene(content: self) { $0.size = (max(pixels(width), 1), max(pixels(height), 1)) }
    }

    public func defaultSize(_ size: CGSize) -> some Scene {
        defaultSize(width: size.width, height: size.height)
    }

    public func windowResizability(_ resizability: WindowResizability) -> some Scene {
        _ModifiedScene(content: self) { $0.resizable = resizability.resizable }
    }

    /// Say what kind of window it is, for the desktop: a tag starting
    /// `dialog:` marks a box, which a desktop may frame and place as one
    /// (with only a close button, in the middle of the screen), and which
    /// has no menu bar. Not in SwiftUI.
    public func windowTag(_ tag: String) -> some Scene {
        _ModifiedScene(content: self) { $0.tag = tag }
    }
}

// MARK: - Opening and closing from a view

/// Closes the window a view is in: `@Environment(\.dismiss) var dismiss`,
/// then `dismiss()`.
@MainActor
public struct DismissAction {
    let action: @MainActor () -> Void

    public func callAsFunction() {
        action()
    }
}

/// Opens a window of the scene with an id:
/// `@Environment(\.openWindow) var openWindow`, then `openWindow(id: "about")`.
@MainActor
public struct OpenWindowAction {
    public func callAsFunction(id: String) {
        Runtime.shared.app?.open(id: id)
    }
}

private enum DismissKey: EnvironmentKey {
    static var defaultValue: DismissAction { MainActor.assumeIsolated { DismissAction(action: {}) } }
}

extension EnvironmentValues {
    public var dismiss: DismissAction {
        get { self[DismissKey.self] }
        set { self[DismissKey.self] = newValue }
    }

    public var openWindow: OpenWindowAction {
        MainActor.assumeIsolated { OpenWindowAction() }
    }
}

// MARK: - Windows

/// A window and everything in it.
@MainActor
final class WindowNode: Node {
    var spec: WindowSpec
    let index: Int
    let window: PressCore.Window
    private let host: StackHost
    private let menus = MenuTree()
    private var bar: PressCore.MenuBar?
    private var title: String

    init(_ spec: WindowSpec, index: Int) throws {
        self.spec = spec
        self.index = index
        self.title = spec.title
        window = try PressCore.Window(title: spec.title, width: spec.size?.0 ?? 64, height: spec.size?.1 ?? 32)
        host = try StackHost(.vertical)
        try host.stack.setAlign(.fill)
        try window.setContent(host.stack)
        try window.setResizable(spec.resizable)
        if let tag = spec.tag { try window.setTag(tag) }
        super.init(EmptyView(), EnvironmentValues())
        try window.onCloseRequested(onMain { [weak self] in self?.close() })
        try window.onFullScreenChanged(onMain { [weak self] (on: Bool) in self?.fullScreenChanged(on) })
    }

    /// What to tell when the desktop puts the window into full screen or
    /// takes it out: the binding of a `.fullScreen(isOn:)` inside.
    private var fullScreenHeard: ((Bool) -> Void)?

    /// Have the window fill its screen, or not, as a view inside asks.
    func setFullScreen(_ on: Bool, heard: @escaping (Bool) -> Void) {
        fullScreenHeard = heard
        if (try? window.fullScreen) != on {
            attempt { try window.setFullScreen(on) }
        }
    }

    private func fullScreenChanged(_ on: Bool) {
        guard alive, let heard = fullScreenHeard else { return }
        Runtime.shared.event { heard(on) }
    }

    override func update() {
        var inner = env
        inner.dismiss = DismissAction { [weak self] in self?.close() }
        reconcileChildren([spec.content], inner)
        contentChanged()
        updateMenus()
    }

    override func contentChanged() {
        var inside: [Item] = []
        for child in children {
            child.collect(&inside)
        }
        host.sync(inside)
    }

    override func collect(_ out: inout [Item]) {}

    private func updateMenus() {
        // a box has no menus of its own
        if spec.tag?.hasPrefix("dialog:") == true { return }
        let commands = (parent as? AppHost)?.commands ?? spec.commands
        let specs = observing(self) {
            menuBar(
                commands, env: env,
                edit: { [weak self] in self?.edit($0) },
                quit: { try? PressCore.App.quit() })
        }
        if !menus.update(specs) {
            attempt {
                let fresh = try PressCore.MenuBar()
                for menu in menus.roots {
                    try fresh.add(menu)
                }
                try window.setMenuBar(fresh)
                try? bar?.destroy()
                bar = fresh
            }
        }
    }

    /// The Edit menu works on whichever text field has the keyboard.
    private func edit(_ action: EditAction) {
        guard let focused = try? window.focused() else { return }
        let field = PressCore.TextField(handle: focused.handle)
        switch action {
        case .cut: try? field.cut()
        case .copy: try? field.copy()
        case .paste: try? field.paste()
        case .selectAll: try? field.selectAll()
        }
    }

    func setTitle(_ title: String) {
        if title != self.title {
            self.title = title
            attempt { try window.setTitle(title) }
        }
    }

    func show() {
        attempt {
            if spec.size == nil {
                try window.fit()
            }
            try window.show()
        }
    }

    func close() {
        guard alive else { return }
        Runtime.shared.event {
            let app = parent as? AppHost
            teardown()
            app?.closed(self)
        }
    }

    override func teardown() {
        super.teardown()
        menus.destroy()
        try? window.destroy()
    }
}

// MARK: - The program

/// A program. Mark one type with `@main`:
///
///     @main
///     struct Hello: App {
///         var body: some Scene {
///             WindowGroup("Hello") { Text("Hello, world.").padding() }
///         }
///     }
@MainActor
public protocol App {
    associatedtype Body: Scene
    @SceneBuilder var body: Body { get }
    init()
    /// How the program looks: its controls, its type, its windows' frames
    /// and where its menus are. A program that says nothing looks as the
    /// desktop it is on would have it.
    ///
    ///     static var style: Style { .twoThousandOne }
    static var style: Style { get }
}

/// A look for a program: one of the styles that come with PRESS, or
/// another by its name or the path of its library.
///
/// A style with a frame of its own has the program's windows draw their
/// title bars and edges themselves, on any desktop that will let them, so
/// the whole window is the program's. `PRESS_STYLE` in the program's
/// environment, which is the user saying otherwise for one run, takes the
/// place of what the program says.
public struct Style: Hashable, Sendable {
    /// The name the style is looked for by; nil for the desktop's choice.
    public let name: String?

    /// A style by its name (looked for as `libpress_style_NAME.so` where
    /// styles are kept) or by the path of its library.
    public static func named(_ name: String) -> Style { Style(name: name) }

    /// Whatever the desktop has chosen for every program.
    public static let desktop = Style(name: nil)
    /// Black and white, in the manner of the first Macintosh, of 1984.
    public static let eightyFour = Style(name: "eightyfour")
    /// Four colours, blue, white, black and orange, in the manner of the
    /// home computer workbench of 1985.
    public static let eightyFive = Style(name: "eightyfive")
    /// Lozenge buttons, a lift for a scroll bar and brackets at the corners
    /// of a window, in the manner of the open workstation look of 1988.
    public static let eightyEight = Style(name: "eightyeight")
    /// Four greys and black title bars, in the manner of the workstation of 1989.
    public static let eightyNine = Style(name: "eightynine")
    /// Grey buttons on white windows, in the manner of the office desktops of 1990.
    public static let ninety = Style(name: "ninety")
    /// Flat pale grey in thin black lines, with a cream title bar, in the
    /// manner of the desktop of 1991 that came in a computer's ROM.
    public static let ninetyOne = Style(name: "ninetyone")
    /// Heavy bevels in slate blue and a frame in rose, in the manner of the
    /// common desktop of the workstations of 1993.
    public static let ninetyThree = Style(name: "ninetythree")
    /// Grey and bevelled, in the manner of the middle 1990s.
    public static let ninetyFive = Style(name: "ninetyfive")
    /// Quiet grey, with the title a yellow tab on the window's corner, in
    /// the manner of the media desktop of 1998.
    public static let ninetyEight = Style(name: "ninetyeight")
    /// Platinum, in the manner of the end of the 1990s.
    public static let ninetyNine = Style(name: "ninetynine")
    /// In the manner of 2000: Ninety-Five in a warmer grey, with smooth type.
    public static let twoThousand = Style(name: "twothousand")
    /// In the manner of 2001: rounded, shaded, blue.
    public static let twoThousandOne = Style(name: "twothousandone")
    /// Parchment, tan and orange, in the manner of the free desktop of 2004.
    public static let twoThousandFour = Style(name: "twothousandfour")
    /// In the manner of 2005: gel, stripes and soft shadows.
    public static let twoThousandFive = Style(name: "twothousandfive")
    /// Lit gel and glass orbs on plain pale ground: a future as 2001 saw it.
    public static let twoThousandTwentySeven = Style(name: "twothousandtwentyseven")
}

extension App {
    public static var style: Style { .desktop }
}

extension App {
    /// Start press, open the first scene's window, and run until the last
    /// window is closed or the program quits.
    public static func main() {
        Runtime.shared.run(Self.self)
    }
}

/// Stands for the program: it keeps the state of the `App` value, and its
/// children are the windows that are open.
@MainActor
final class AppHost: CompositeNode {
    private let scenes: () -> [WindowSpec]
    private let properties: Any?
    private(set) var specs: [WindowSpec] = []
    private(set) var commands: [CommandSpec] = []
    private var launched = false

    init<A: App>(_ app: A) {
        properties = app
        scenes = {
            var out: [WindowSpec] = []
            collectScenes(sceneOf(app), &out)
            return out
        }
        super.init(EmptyView(), EnvironmentValues())
    }

    /// A program of one window holding a view, with no `App` behind it.
    init(_ title: String, _ view: any View) {
        properties = nil
        scenes = { [WindowSpec(title: title, id: nil, content: view, many: true)] }
        super.init(EmptyView(), EnvironmentValues())
    }

    var windows: [WindowNode] { children.compactMap { $0 as? WindowNode } }

    override func update() {
        if let properties {
            var slot = 0
            link(properties, &slot, depth: 0)
        }
        specs = observing(self) { scenes() }
        commands = specs.flatMap(\.commands)
        if !launched {
            launched = true
            if !specs.isEmpty {
                open(0)
            }
            return
        }
        for window in windows where specs.indices.contains(window.index) {
            window.spec = specs[window.index]
            window.refresh()
        }
    }

    private func open(_ index: Int) {
        attempt {
            let node = try WindowNode(specs[index], index: index)
            node.parent = self
            children.append(node)
            node.refresh()
            node.show()
        }
    }

    func open(id: String) {
        guard let index = specs.firstIndex(where: { $0.id == id }) else {
            FileHandle.standardError.write(Data("Press: no scene has the id \"\(id)\"\n".utf8))
            return
        }
        if !specs[index].many, windows.contains(where: { $0.index == index }) {
            return
        }
        Runtime.shared.event { open(index) }
    }

    func closed(_ window: WindowNode) {
        children.removeAll { $0 === window }
        if children.isEmpty {
            try? PressCore.App.quit()
        }
    }
}

extension Runtime {
    /// Bring a program's first window into being, without running it.
    func launch(_ host: AppHost) {
        app = host
        event { host.refresh() }
    }

    /// Forget the program, for a test that wants to start another.
    func reset() {
        app?.teardown()
        app = nil
    }

    func run<A: App>(_ type: A.Type) {
        do {
            try PressCore.App.initialize()
            if let name = type.style.name {
                // a style that cannot be had is said so by the engine, and the program looks as it did
                try? PressCore.App.setStyle(name)
            }
            let host = AppHost(A())
            launch(host)
            // For looking at a program without a screen: draw the first
            // window to a PNG file and stop.
            if let path = ProcessInfo.processInfo.environment["PRESS_SNAPSHOT"], !path.isEmpty {
                try host.windows.first?.window.snapshot(path, scale: 1)
                try PressCore.App.shutdown()
                exit(0)
            }
            // With no display there is nobody to close the last window, so
            // the program is over once its windows are made: as the engine's
            // own loop, press_app_run, returns at once without one.
            if ProcessInfo.processInfo.environment["PRESS_BACKEND"] == "headless" {
                try PressCore.App.shutdown()
                exit(0)
            }
            // press is asked for the descriptor that says it has work, and
            // the main queue waits on that, so that tasks on the main actor
            // and press's events take turns in one loop.
            try PressCore.App.start()
            let source = DispatchSource.makeReadSource(fileDescriptor: Int32(try PressCore.App.fd()), queue: .main)
            source.setEventHandler { MainActor.assumeIsolated { Runtime.shared.turn() } }
            loop = source
            source.resume()
            turn()
            // Not dispatchMain(): on Linux that gives the main queue to
            // another thread, and press answers only the thread it was
            // started on. The run loop serves the main queue from this one.
            while true {
                RunLoop.main.run()
            }
        } catch {
            FileHandle.standardError.write(Data("Press: \(error)\n".utf8))
            exit(1)
        }
    }

    /// Let press do what is ready. It says when the program is over: the
    /// last window has closed, or something asked to quit.
    private func turn() {
        do {
            if try PressCore.App.dispatch() {
                return
            }
        } catch {
            FileHandle.standardError.write(Data("Press: \(error)\n".utf8))
            exit(1)
        }
        loop?.cancel()
        try? PressCore.App.shutdown()
        exit(0)
    }
}
