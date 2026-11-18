// SPDX-License-Identifier: GPL-3.0-or-later
// Press: press in the manner of SwiftUI. A program says what its windows
// hold as values, in `body`, and this module keeps the widgets of the PressCore
// module in step with them.
//
// This file is the middle of it: what a view is, the builder that gathers
// views, the state a view keeps, and the tree of nodes that stands between
// the views a program describes and the widgets on the screen.
import Foundation
import PressCore
import Observation

// MARK: - Views

/// A piece of a window, described by what it is made of.
@MainActor
public protocol View {
    associatedtype Body: View
    @ViewBuilder var body: Body { get }
}

extension Never: View {
    public var body: Never { fatalError() }
}

extension View where Body == Never {
    public var body: Never { fatalError("\(Self.self) is drawn by press and has no body") }
}

/// Nothing at all.
public struct EmptyView: View, _NodeView {
    public typealias Body = Never
    public init() {}
    func _makeNode(_ env: EnvironmentValues) -> Node { Node(self, env) }
}

/// Several views side by side in a builder.
public struct TupleView<each Content: View>: View, _NodeView, _Describable {
    public typealias Body = Never
    let value: (repeat each Content)

    init(_ value: (repeat each Content)) {
        self.value = value
    }

    var views: [any View] {
        var out: [any View] = []
        repeat out.append(each value)
        return out
    }

    func _makeNode(_ env: EnvironmentValues) -> Node { ListNode(self, env) { ($0 as! Self).views } }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        for view in views {
            describe(view, &out, context)
        }
    }
}

/// One of two views, as an `if` with an `else` chooses.
public struct _ConditionalContent<TrueContent: View, FalseContent: View>: View, _NodeView, _Describable {
    public typealias Body = Never
    enum Storage {
        case first(TrueContent)
        case second(FalseContent)
    }
    let storage: Storage

    var chosen: any View {
        switch storage {
        case .first(let view): return view
        case .second(let view): return view
        }
    }

    var isFirst: Bool {
        if case .first = storage { return true }
        return false
    }

    func _makeNode(_ env: EnvironmentValues) -> Node { ConditionalNode(self, env) }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        describe(chosen, &out, context)
    }
}

extension Optional: View where Wrapped: View {
    public typealias Body = Never
}

extension Optional: _NodeView where Wrapped: View {
    func _makeNode(_ env: EnvironmentValues) -> Node {
        ListNode(self, env) { ($0 as! Self).map { [$0] } ?? [] }
    }
}

extension Optional: _Describable where Wrapped: View {
    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        if let view = self {
            describe(view, &out, context)
        }
    }
}

/// A view whose kind is not said.
public struct AnyView: View, _NodeView, _Describable {
    public typealias Body = Never
    let wrapped: any View

    public init<V: View>(_ view: V) {
        self.wrapped = view
    }

    init(erasing view: any View) {
        self.wrapped = view
    }

    func _makeNode(_ env: EnvironmentValues) -> Node { ListNode(self, env) { [($0 as! Self).wrapped] } }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        describe(wrapped, &out, context)
    }
}

/// Views gathered so as to be treated as one.
public struct Group<Content: View>: View, _NodeView, _Describable {
    public typealias Body = Never
    let content: Content

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    func _makeNode(_ env: EnvironmentValues) -> Node { ListNode(self, env) { [($0 as! Self).content] } }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        describe(content, &out, context)
    }
}

/// A view for each of a collection of things. Each is known by its id, so
/// that what a view remembers stays with its thing when the order changes.
public struct ForEach<Data: RandomAccessCollection, ID: Hashable, Content: View>: View, _NodeView, _Describable {
    public typealias Body = Never
    let data: Data
    let id: (Data.Element) -> ID
    let content: (Data.Element) -> Content

    public init(_ data: Data, id: KeyPath<Data.Element, ID>, @ViewBuilder content: @escaping (Data.Element) -> Content) {
        self.data = data
        self.id = { $0[keyPath: id] }
        self.content = content
    }

    var keyed: [(AnyHashable, any View)] {
        data.map { (AnyHashable(id($0)), content($0)) }
    }

    func _makeNode(_ env: EnvironmentValues) -> Node { ForEachNode(self, env) { ($0 as! Self).keyed } }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        for (key, view) in keyed {
            // An element's id is its tag, unless its view gives another.
            var context = context
            context.tag = key
            describe(view, &out, context)
        }
    }
}

extension ForEach where Data.Element: Identifiable, ID == Data.Element.ID {
    public init(_ data: Data, @ViewBuilder content: @escaping (Data.Element) -> Content) {
        self.init(data, id: \.id, content: content)
    }
}

extension ForEach where Data == Range<Int>, ID == Int {
    public init(_ data: Range<Int>, @ViewBuilder content: @escaping (Int) -> Content) {
        self.init(data, id: \.self, content: content)
    }
}

/// Gathers the views written in a closure.
@MainActor
@resultBuilder
public enum ViewBuilder {
    public static func buildBlock() -> EmptyView { EmptyView() }
    public static func buildBlock<Content: View>(_ content: Content) -> Content { content }
    public static func buildBlock<each Content: View>(_ content: repeat each Content) -> TupleView<repeat each Content> {
        TupleView((repeat each content))
    }
    public static func buildExpression<Content: View>(_ content: Content) -> Content { content }
    public static func buildIf<Content: View>(_ content: Content?) -> Content? { content }
    public static func buildEither<T: View, F: View>(first: T) -> _ConditionalContent<T, F> {
        _ConditionalContent(storage: .first(first))
    }
    public static func buildEither<T: View, F: View>(second: F) -> _ConditionalContent<T, F> {
        _ConditionalContent(storage: .second(second))
    }
    public static func buildLimitedAvailability<Content: View>(_ content: Content) -> AnyView { AnyView(content) }
}

// MARK: - State

/// A property of a view that press looks after between one `body` and the
/// next.
public protocol DynamicProperty {}

@MainActor
protocol _LinkedProperty {
    /// Tie this property, found in a view about to be asked for its body,
    /// to what its node keeps.
    func _link(_ node: CompositeNode, _ slot: inout Int)
}

/// Where a state's value lives for as long as its view is in a window.
final class StateStorage {
    var value: Any
    let node: WeakNode

    init(_ value: Any, _ node: WeakNode) {
        self.value = value
        self.node = node
    }
}

/// What a `State` holds. A view is a value, made afresh whenever the view
/// above it is asked for its body; each time, its box is pointed at the
/// storage its node has kept.
final class StateBox<Value> {
    var initial: Value
    var storage: StateStorage?

    init(_ initial: Value) {
        self.initial = initial
    }

    var value: Value {
        get { (storage?.value as? Value) ?? initial }
        set {
            guard let storage else {
                initial = newValue
                return
            }
            storage.value = newValue
            Runtime.invalidate(storage.node)
        }
    }
}

/// A value a view keeps, and is drawn again for when it changes.
@propertyWrapper
public struct State<Value>: DynamicProperty, _LinkedProperty {
    let box: StateBox<Value>

    public init(wrappedValue: Value) {
        box = StateBox(wrappedValue)
    }

    public init(initialValue: Value) {
        box = StateBox(initialValue)
    }

    public var wrappedValue: Value {
        get { box.value }
        nonmutating set { box.value = newValue }
    }

    public var projectedValue: Binding<Value> {
        let box = self.box
        return Binding(get: { box.value }, set: { box.value = $0 })
    }

    func _link(_ node: CompositeNode, _ slot: inout Int) {
        if slot == node.storages.count {
            node.storages.append(StateStorage(box.initial, WeakNode(node)))
        }
        box.storage = node.storages[slot]
        slot += 1
    }
}

extension State where Value: ExpressibleByNilLiteral {
    public init() {
        self.init(wrappedValue: nil)
    }
}

/// A way to read and change a value kept somewhere else.
@propertyWrapper
@dynamicMemberLookup
public struct Binding<Value>: DynamicProperty {
    let getter: () -> Value
    let setter: (Value) -> Void

    public init(get: @escaping () -> Value, set: @escaping (Value) -> Void) {
        getter = get
        setter = set
    }

    public var wrappedValue: Value {
        get { getter() }
        nonmutating set { setter(newValue) }
    }

    public var projectedValue: Binding<Value> { self }

    /// A binding to a value that does not change.
    public static func constant(_ value: Value) -> Binding<Value> {
        Binding(get: { value }, set: { _ in })
    }

    public subscript<Subject>(dynamicMember keyPath: WritableKeyPath<Value, Subject>) -> Binding<Subject> {
        Binding<Subject>(
            get: { self.wrappedValue[keyPath: keyPath] },
            set: { newValue in
                var value = self.wrappedValue
                value[keyPath: keyPath] = newValue
                self.wrappedValue = value
            })
    }
}

/// Gives bindings to the properties of an observable object:
/// `@Bindable var model`, then `$model.name`.
@propertyWrapper
@dynamicMemberLookup
public struct Bindable<Value: AnyObject & Observable>: DynamicProperty {
    public var wrappedValue: Value

    public init(wrappedValue: Value) {
        self.wrappedValue = wrappedValue
    }

    public init(_ wrappedValue: Value) {
        self.wrappedValue = wrappedValue
    }

    public var projectedValue: Bindable<Value> { self }

    public subscript<Subject>(dynamicMember keyPath: ReferenceWritableKeyPath<Value, Subject>) -> Binding<Subject> {
        let object = wrappedValue
        return Binding<Subject>(get: { object[keyPath: keyPath] }, set: { object[keyPath: keyPath] = $0 })
    }
}

// MARK: - Environment

/// A value handed down from a view to everything inside it.
public protocol EnvironmentKey {
    associatedtype Value
    static var defaultValue: Value { get }
}

/// The values handed down to a view.
public struct EnvironmentValues {
    var values: [ObjectIdentifier: Any] = [:]

    public init() {}

    public subscript<K: EnvironmentKey>(key: K.Type) -> K.Value {
        get { (values[ObjectIdentifier(key)] as? K.Value) ?? K.defaultValue }
        set { values[ObjectIdentifier(key)] = newValue }
    }

    subscript<T: AnyObject>(object type: T.Type) -> T? {
        get { values[ObjectIdentifier(type)] as? T }
        set { values[ObjectIdentifier(type)] = newValue }
    }
}

final class EnvironmentBox {
    var values = EnvironmentValues()
}

/// Reads a value handed down to the view: `@Environment(\.isEnabled)`, or an
/// observable object by its type, `@Environment(Model.self)`.
@propertyWrapper
public struct Environment<Value>: DynamicProperty, _LinkedProperty {
    let read: (EnvironmentValues) -> Value
    let box = EnvironmentBox()

    public init(_ keyPath: KeyPath<EnvironmentValues, Value>) {
        read = { $0[keyPath: keyPath] }
    }

    public init(_ type: Value.Type) where Value: AnyObject & Observable {
        read = { values in
            guard let object = values[object: type] else {
                fatalError("No \(type) was put in the environment; use .environment(_:) on a view above this one")
            }
            return object
        }
    }

    public var wrappedValue: Value { read(box.values) }

    func _link(_ node: CompositeNode, _ slot: inout Int) {
        box.values = node.env
    }
}

// MARK: - The tree of nodes

/// A view that press knows how to keep a node for, where other views are
/// asked for their body.
@MainActor
protocol _NodeView {
    func _makeNode(_ env: EnvironmentValues) -> Node
}

/// A widget a node gives to whatever holds it, and what was said about it
/// on the way up.
struct Item {
    let widget: PressCore.Widget
    var tag: AnyHashable?
    var tabTitle: String?
}

/// A size a view was asked to be.
struct FrameSpec {
    var width: Int?
    var height: Int?
    var minWidth: Int?
    var minHeight: Int?
    var stretch: Bool?
}

final class WeakNode: @unchecked Sendable {
    weak var node: Node?

    init(_ node: Node) {
        self.node = node
    }
}

/// What stands for a view while it is in a window. Views are made again and
/// again; a node stays, holding the view's state and its widgets, for as
/// long as a view of the same kind is described in the same place.
@MainActor
class Node {
    weak var parent: Node?
    var view: any View
    var env: EnvironmentValues
    var children: [Node] = []
    var alive = true
    var dirty = false

    init(_ view: any View, _ env: EnvironmentValues) {
        self.view = view
        self.env = env
    }

    /// Bring the node's children and widgets in step with `view` and `env`.
    func update() {}

    /// The widgets this node gives to whatever holds it.
    func collect(_ out: inout [Item]) {
        for child in children {
            child.collect(&out)
        }
    }

    /// The node is leaving its window for good.
    func teardown() {
        alive = false
        for child in children {
            child.teardown()
        }
    }

    /// Something beneath gives different widgets from before.
    func contentChanged() {
        parent?.contentChanged()
    }

    func applyFrame(_ frame: FrameSpec) {
        for child in children {
            child.applyFrame(frame)
        }
    }

    final func refresh() {
        dirty = false
        update()
    }

    var depth: Int {
        var (count, node) = (0, parent)
        while let next = node {
            count += 1
            node = next.parent
        }
        return count
    }

    var windowNode: WindowNode? {
        var node: Node? = self
        while let next = node {
            if let window = next as? WindowNode {
                return window
            }
            node = next.parent
        }
        return nil
    }

    var items: [Item] {
        var out: [Item] = []
        collect(&out)
        return out
    }

    /// Keep the node in a place if the view now described there is of the
    /// kind it stands for, and otherwise put a new one in its place.
    final func reconcile(_ old: Node?, _ view: any View, _ env: EnvironmentValues) -> Node {
        if let old, type(of: old.view) == type(of: view) {
            old.view = view
            old.env = env
            old.refresh()
            return old
        }
        old?.teardown()
        let node = (view as? any _NodeView)?._makeNode(env) ?? CompositeNode(view, env)
        node.parent = self
        node.refresh()
        return node
    }

    /// Reconcile a run of children by their places.
    final func reconcileChildren(_ views: [any View], _ env: EnvironmentValues) {
        var next: [Node] = []
        for (index, view) in views.enumerated() {
            next.append(reconcile(index < children.count ? children[index] : nil, view, env))
        }
        for extra in children.dropFirst(views.count) {
            extra.teardown()
        }
        children = next
    }
}

/// A view with a body: a program's own views.
@MainActor
class CompositeNode: Node {
    var storages: [StateStorage] = []

    override func update() {
        var slot = 0
        link(view, &slot, depth: 0)
        let body = observing(self) { bodyOf(self.view) }
        reconcileChildren([body], env)
    }

    /// Find the properties press looks after, by looking through the view's
    /// stored properties, and through those of a modifier it carries.
    final func link(_ value: Any, _ slot: inout Int, depth: Int) {
        for child in Mirror(reflecting: value).children {
            if let property = child.value as? any _LinkedProperty {
                property._link(self, &slot)
            } else if depth == 0, child.value is any ViewModifier {
                link(child.value, &slot, depth: 1)
            }
        }
    }
}

@MainActor
func bodyOf<V: View>(_ view: V) -> any View {
    view.body
}

/// Several views in a row, known by their places.
@MainActor
final class ListNode: Node {
    let views: (any View) -> [any View]

    init(_ view: any View, _ env: EnvironmentValues, _ views: @escaping (any View) -> [any View]) {
        self.views = views
        super.init(view, env)
    }

    override func update() {
        reconcileChildren(views(view), env)
    }
}

@MainActor
final class ConditionalNode: Node {
    var wasFirst: Bool?

    override func update() {
        let view = self.view as! any _Conditional
        // The two sides are different places even when they hold the same
        // kind of view, so what one remembers is not handed to the other.
        if wasFirst != view._isFirst {
            for child in children {
                child.teardown()
            }
            children = []
            wasFirst = view._isFirst
        }
        reconcileChildren([view._chosen], env)
    }
}

@MainActor
protocol _Conditional {
    var _isFirst: Bool { get }
    var _chosen: any View { get }
}

extension _ConditionalContent: _Conditional {
    var _isFirst: Bool { isFirst }
    var _chosen: any View { chosen }
}

@MainActor
final class ForEachNode: Node {
    let keyed: (any View) -> [(AnyHashable, any View)]
    var keys: [AnyHashable] = []

    init(_ view: any View, _ env: EnvironmentValues, _ keyed: @escaping (any View) -> [(AnyHashable, any View)]) {
        self.keyed = keyed
        super.init(view, env)
    }

    override func update() {
        // The closure that makes each view runs here, not in a body, so
        // what it reads of observable objects is watched for here.
        let wanted = observing(self) { keyed(view) }
        var old: [AnyHashable: Node] = [:]
        for (key, node) in zip(keys, children) where old[key] == nil {
            old[key] = node
        }
        var next: [Node] = []
        var nextKeys: [AnyHashable] = []
        for (key, view) in wanted {
            // each element's id is its tag, as in a picker or a list
            let node = reconcile(old.removeValue(forKey: key), view, env)
            next.append(node)
            nextKeys.append(key)
        }
        for node in old.values {
            node.teardown()
        }
        children = next
        keys = nextKeys
    }

    override func collect(_ out: inout [Item]) {
        for (key, child) in zip(keys, children) {
            var mine: [Item] = []
            child.collect(&mine)
            for index in mine.indices where mine[index].tag == nil {
                mine[index].tag = key
            }
            out.append(contentsOf: mine)
        }
    }
}

// MARK: - Widgets

/// A view that is one widget.
@MainActor
protocol _LeafView: View, _NodeView {
    associatedtype Widget: PressCore.Widget
    /// Make the widget and connect its events. Handlers should read the
    /// node's view when they are called, since the view is replaced often.
    func _make(_ node: LeafNode<Self>) throws -> Widget
    /// Bring the widget in step with this view.
    func _update(_ widget: Widget, _ node: LeafNode<Self>) throws
    func _frame(_ widget: Widget, _ frame: FrameSpec) throws
    /// A typeface the view asks for itself, over the one around it.
    var _ownFont: Font? { get }
}

extension _LeafView {
    func _makeNode(_ env: EnvironmentValues) -> Node { LeafNode<Self>(self, env) }

    var _ownFont: Font? { nil }

    func _frame(_ widget: Widget, _ frame: FrameSpec) throws {
        try applyFrame(widget, frame)
    }
}

/// What every widget can be told of a size.
@MainActor
func applyFrame(_ widget: PressCore.Widget, _ frame: FrameSpec) throws {
    let width = frame.width ?? frame.minWidth
    let height = frame.height ?? frame.minHeight
    if width != nil || height != nil {
        try widget.setMinSize(width ?? 0, height: height ?? 0)
    }
    if let stretch = frame.stretch {
        try widget.setStretch(stretch)
    }
}

@MainActor
final class LeafNode<V: _LeafView>: Node {
    var widget: V.Widget?
    /// What the widget was last told, for views that need to know.
    var text: String?
    var number: Int?
    var labels: [String] = []
    var memory: Any?
    private var wasEnabled = true
    private var typeface = Typeface.cicero

    var typed: V { view as! V }

    override func update() {
        attempt {
            if widget == nil {
                widget = try typed._make(self)
            }
            guard let widget else { return }
            // what its bindings read of observable objects is watched, so a
            // field bound to a model hears the program change the model
            let updated: Result<Void, Error> = observing(self) { Result { try typed._update(widget, self) } }
            try updated.get()
            if env.isEnabled != wasEnabled {
                wasEnabled = env.isEnabled
                try widget.setEnabled(wasEnabled)
            }
            let face = (typed._ownFont ?? env.font ?? .body).typeface
            if face != typeface {
                typeface = face
                // only widgets with words in them have a typeface
                try? widget.setTypeface(face)
            }
        }
    }

    override func collect(_ out: inout [Item]) {
        if let widget {
            out.append(Item(widget: widget))
        }
    }

    override func applyFrame(_ frame: FrameSpec) {
        if let widget {
            attempt { try typed._frame(widget, frame) }
        }
    }

    override func teardown() {
        super.teardown()
        try? widget?.destroy()
    }

    /// Run a handler for something the user did, then draw again whatever
    /// it changed.
    func event(_ body: (V) -> Void) {
        guard alive else { return }
        Runtime.shared.event { body(typed) }
    }
}

/// A stack widget and the widgets a node has put in it.
@MainActor
final class StackHost {
    let stack: PressCore.Stack
    private var mounted: [UInt64] = []

    init(_ orientation: Orientation) throws {
        stack = try PressCore.Stack(orientation)
    }

    /// Make the stack hold these widgets in this order. Widgets no longer
    /// wanted have been destroyed by their nodes, which takes them out.
    func sync(_ items: [Item]) {
        let wanted = items.map(\.widget.handle)
        guard wanted != mounted else { return }
        var current = mounted.filter(wanted.contains)
        for (index, handle) in wanted.enumerated() {
            if index < current.count, current[index] == handle {
                continue
            }
            attempt { try stack.insert(PressCore.Widget(handle: handle), index: index) }
            current.removeAll { $0 == handle }
            current.insert(handle, at: min(index, current.count))
        }
        mounted = wanted
    }
}

// MARK: - The runtime

/// Report something press refused. A view cannot throw, so what goes wrong
/// while a window is brought up to date is said here and work goes on.
@MainActor
func attempt(_ body: () throws -> Void) {
    do {
        try body()
    } catch {
        FileHandle.standardError.write(Data("Press: \(error)\n".utf8))
    }
}

/// Run something that reads observable objects, and have the node updated
/// again when any of what was read changes.
@MainActor
func observing<T>(_ node: Node, _ body: () -> T) -> T {
    let reference = WeakNode(node)
    return withObservationTracking(body) {
        Runtime.invalidate(reference)
    }
}

@MainActor
final class Runtime {
    static let shared = Runtime()

    private var dirty: [Node] = []
    /// More than nought while a handler or an update is running; what is
    /// invalidated meanwhile is updated when it has finished.
    private var busy = 0
    private var posted = false
    var app: AppHost?
    /// What wakes the main queue when press has work.
    var loop: DispatchSourceRead?

    /// Ask for a node to be updated. This may be called from any thread.
    nonisolated static func invalidate(_ reference: WeakNode) {
        if Thread.isMainThread {
            MainActor.assumeIsolated {
                if let node = reference.node {
                    shared.invalidate(node)
                }
            }
        } else {
            try? PressCore.App.post {
                MainActor.assumeIsolated {
                    if let node = reference.node {
                        shared.invalidate(node)
                    }
                }
            }
        }
    }

    func invalidate(_ node: Node) {
        guard node.alive else { return }
        if !node.dirty {
            node.dirty = true
            dirty.append(node)
        }
        if busy == 0, !posted {
            // Not in a handler: an asynchronous task changed something.
            // Update on the next turn of the event loop.
            posted = true
            do {
                try PressCore.App.post {
                    MainActor.assumeIsolated {
                        Runtime.shared.posted = false
                        Runtime.shared.flush()
                    }
                }
            } catch {
                posted = false
            }
        }
    }

    /// Run a handler, then update whatever it invalidated.
    func event(_ body: () -> Void) {
        busy += 1
        body()
        busy -= 1
        if busy == 0 {
            flush()
        }
    }

    /// Update every node that was invalidated, those nearest the window
    /// first, since updating one updates everything inside it.
    func flush() {
        busy += 1
        defer { busy -= 1 }
        var rounds = 0
        while !dirty.isEmpty {
            rounds += 1
            if rounds > 100 {
                FileHandle.standardError.write(Data("Press: a view changes its own state every time it is updated\n".utf8))
                dirty.forEach { $0.dirty = false }
                dirty = []
                break
            }
            let batch = dirty.sorted { $0.depth < $1.depth }
            dirty = []
            for node in batch where node.dirty && node.alive {
                node.refresh()
                node.parent?.contentChanged()
            }
        }
    }
}
