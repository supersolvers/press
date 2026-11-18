// SPDX-License-Identifier: GPL-3.0-or-later
// What holds other views: stacks, scrolling, tabs, lists and tables.
import Foundation
import PressCore

// MARK: - Stacks

/// Where the views of a column sit across it.
public struct HorizontalAlignment: Hashable, Sendable {
    let align: Align

    public static let leading = HorizontalAlignment(align: .start)
    public static let center = HorizontalAlignment(align: .center)
    public static let trailing = HorizontalAlignment(align: .end)
    /// Not in SwiftUI: every view is made as wide as the column.
    public static let fill = HorizontalAlignment(align: .fill)
}

/// Where the views of a row sit across it.
public struct VerticalAlignment: Hashable, Sendable {
    let align: Align

    public static let top = VerticalAlignment(align: .start)
    public static let center = VerticalAlignment(align: .center)
    public static let bottom = VerticalAlignment(align: .end)
    /// Not in SwiftUI: every view is made as tall as the row.
    public static let fill = VerticalAlignment(align: .fill)
}

/// A view that is a stack widget holding the widgets of its content.
@MainActor
protocol _StackView: View, _NodeView {
    var _orientation: Orientation { get }
    var _align: Align { get }
    var _spacing: Int { get }
    var _padding: Int { get }
    var _content: any View { get }
}

extension _StackView {
    func _makeNode(_ env: EnvironmentValues) -> Node { StackNode(self, env) }
}

@MainActor
final class StackNode: Node {
    private var host: StackHost?
    private var told: (Align, Int, Int)?

    override func update() {
        let view = self.view as! any _StackView
        attempt {
            if host == nil {
                host = try StackHost(view._orientation)
            }
            guard let stack = host?.stack else { return }
            let wanted = (view._align, view._spacing, view._padding)
            if told.map({ $0 != wanted }) ?? true {
                told = wanted
                try stack.setAlign(wanted.0)
                try stack.setSpacing(wanted.1)
                try stack.setPadding(wanted.2)
            }
        }
        reconcileChildren([view._content], env)
        contentChanged()
    }

    override func contentChanged() {
        var inside: [Item] = []
        for child in children {
            child.collect(&inside)
        }
        host?.sync(inside)
    }

    override func collect(_ out: inout [Item]) {
        if let host {
            out.append(Item(widget: host.stack))
        }
    }

    override func applyFrame(_ frame: FrameSpec) {
        if let host {
            attempt { try Press.applyFrame(host.stack, frame) }
        }
    }

    override func teardown() {
        super.teardown()
        try? host?.stack.destroy()
    }
}

/// A column of views.
public struct VStack<Content: View>: View, _StackView {
    public typealias Body = Never
    let alignment: HorizontalAlignment
    let spacing: CGFloat?
    let content: Content

    public init(alignment: HorizontalAlignment = .center, spacing: CGFloat? = nil, @ViewBuilder content: () -> Content) {
        self.alignment = alignment
        self.spacing = spacing
        self.content = content()
    }

    var _orientation: Orientation { .vertical }
    var _align: Align { alignment.align }
    var _spacing: Int { spacing.map(pixels) ?? 8 }
    var _padding: Int { 0 }
    var _content: any View { content }
}

/// A row of views.
public struct HStack<Content: View>: View, _StackView {
    public typealias Body = Never
    let alignment: VerticalAlignment
    let spacing: CGFloat?
    let content: Content

    public init(alignment: VerticalAlignment = .center, spacing: CGFloat? = nil, @ViewBuilder content: () -> Content) {
        self.alignment = alignment
        self.spacing = spacing
        self.content = content()
    }

    var _orientation: Orientation { .horizontal }
    var _align: Align { alignment.align }
    var _spacing: Int { spacing.map(pixels) ?? 8 }
    var _padding: Int { 0 }
    var _content: any View { content }
}

/// A view with a margin round it.
public struct _PaddingView<Content: View>: View, _StackView {
    public typealias Body = Never
    let content: Content
    let padding: Int

    var _orientation: Orientation { .vertical }
    var _align: Align { .fill }
    var _spacing: Int { 8 }
    var _padding: Int { padding }
    var _content: any View { content }
}

/// The controls of a dialog, in a column from the left.
public struct Form<Content: View>: View {
    let content: Content

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 10) { content }
    }
}

/// Some views under a heading.
public struct Section<Header: View, Content: View>: View, _Describable {
    let header: Header
    let content: Content

    public init(@ViewBuilder content: () -> Content, @ViewBuilder header: () -> Header) {
        self.header = header()
        self.content = content()
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            header.font(.subheadline)
            content
        }
    }

    /// In a menu, a section is set off by dividing lines.
    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        if !out.isEmpty {
            out.append(ItemSpec(.divider, context))
        }
        describe(content, &out, context)
    }
}

extension Section where Header == Text {
    public init(_ title: some StringProtocol, @ViewBuilder content: () -> Content) {
        self.init(content: content) { Text(title) }
    }
}

extension Section where Header == EmptyView {
    public init(@ViewBuilder content: () -> Content) {
        self.init(content: content) { EmptyView() }
    }
}

/// A view with its name before it.
public struct LabeledContent<Content: View>: View {
    let title: String
    let content: Content

    public init(_ title: some StringProtocol, @ViewBuilder content: () -> Content) {
        self.title = String(title)
        self.content = content()
    }

    public var body: some View {
        HStack {
            Text(title)
            content
        }
    }
}

extension LabeledContent where Content == Text {
    public init(_ title: some StringProtocol, value: some StringProtocol) {
        self.init(title) { Text(value) }
    }
}

// MARK: - Scrolling

public enum Axis: Sendable {
    case horizontal, vertical

    public struct Set: OptionSet, Sendable {
        public let rawValue: Int

        public init(rawValue: Int) {
            self.rawValue = rawValue
        }

        public static let horizontal = Set(rawValue: 1)
        public static let vertical = Set(rawValue: 2)
    }
}

/// An opening onto views that may need more room than there is, with
/// scroll bars to move them. `.frame(width:height:)` sets its size.
public struct ScrollView<Content: View>: View, _NodeView {
    public typealias Body = Never
    let content: Content

    public init(_ axes: Axis.Set = .vertical, @ViewBuilder content: () -> Content) {
        self.content = content()
    }

    func _makeNode(_ env: EnvironmentValues) -> Node {
        ScrollNode(self, env) { ($0 as! Self).content }
    }
}

@MainActor
final class ScrollNode: Node {
    private let content: (any View) -> any View
    private var scroll: PressCore.ScrollView?
    private var host: StackHost?

    init(_ view: any View, _ env: EnvironmentValues, _ content: @escaping (any View) -> any View) {
        self.content = content
        super.init(view, env)
    }

    override func update() {
        attempt {
            if scroll == nil {
                let scroll = try PressCore.ScrollView()
                let host = try StackHost(.vertical)
                try host.stack.setAlign(.fill)
                try scroll.setContent(host.stack)
                (self.scroll, self.host) = (scroll, host)
            }
        }
        reconcileChildren([content(view)], env)
        contentChanged()
    }

    override func contentChanged() {
        var inside: [Item] = []
        for child in children {
            child.collect(&inside)
        }
        host?.sync(inside)
    }

    override func collect(_ out: inout [Item]) {
        if let scroll {
            out.append(Item(widget: scroll))
        }
    }

    override func applyFrame(_ frame: FrameSpec) {
        guard let scroll else { return }
        attempt {
            if let width = frame.width ?? frame.minWidth {
                try scroll.setWidth(width)
            }
            if let height = frame.height ?? frame.minHeight {
                try scroll.setHeight(height)
            }
            if let stretch = frame.stretch {
                try scroll.setStretch(stretch)
            }
        }
    }

    override func teardown() {
        super.teardown()
        try? scroll?.destroy()
    }
}

// MARK: - Tabs

/// Several pages in the room of one, with a row of tabs to choose between
/// them. Each view inside is a page; `.tabItem { Text("Name") }` names its
/// tab and `.tag` gives the value `selection` takes when it shows.
public struct TabView<SelectionValue: Hashable, Content: View>: View, _NodeView {
    public typealias Body = Never
    let selection: Binding<SelectionValue>?
    let content: Content

    public init(selection: Binding<SelectionValue>?, @ViewBuilder content: () -> Content) {
        self.selection = selection
        self.content = content()
    }

    func _makeNode(_ env: EnvironmentValues) -> Node {
        TabNode(
            self, env,
            content: { ($0 as! Self).content },
            selected: { ($0 as! Self).selection.flatMap { hashable($0.wrappedValue) } },
            select: { view, tag in
                if let value = tag.base as? SelectionValue {
                    (view as! Self).selection?.wrappedValue = value
                }
            })
    }
}

extension TabView where SelectionValue == Int {
    public init(@ViewBuilder content: () -> Content) {
        self.init(selection: nil, content: content)
    }
}

@MainActor
final class TabNode: Node {
    private let content: (any View) -> any View
    private let selected: (any View) -> AnyHashable?
    private let select: (any View, AnyHashable) -> Void
    private var tabs: PressCore.TabView?
    /// The pages in the order the widget has them.
    private var pages: [Item] = []
    private var showing: Int?

    init(
        _ view: any View, _ env: EnvironmentValues, content: @escaping (any View) -> any View,
        selected: @escaping (any View) -> AnyHashable?, select: @escaping (any View, AnyHashable) -> Void
    ) {
        self.content = content
        self.selected = selected
        self.select = select
        super.init(view, env)
    }

    override func update() {
        attempt {
            if tabs == nil {
                let tabs = try PressCore.TabView()
                try tabs.onChanged(
                    onMain { [weak self] (index: Int) in
                        guard let self, self.alive else { return }
                        self.showing = index
                        Runtime.shared.event {
                            if self.pages.indices.contains(index), let tag = self.pages[index].tag {
                                self.select(self.view, tag)
                            }
                        }
                    })
                self.tabs = tabs
            }
        }
        reconcileChildren([content(view)], env)
        contentChanged()
    }

    /// Pages that have gone were destroyed, which took them out; new ones
    /// go on the end, which is the only place press can put a page.
    override func contentChanged() {
        guard let tabs else { return }
        var wanted: [Item] = []
        for child in children {
            child.collect(&wanted)
        }
        let handles = Set(wanted.map(\.widget.handle))
        pages.removeAll { !handles.contains($0.widget.handle) }
        for item in wanted {
            if let index = pages.firstIndex(where: { $0.widget.handle == item.widget.handle }) {
                pages[index] = item
            } else {
                attempt { try tabs.add(item.tabTitle ?? "", content: item.widget) }
                pages.append(item)
            }
        }
        if let tag = selected(view), let index = pages.firstIndex(where: { $0.tag == tag }), showing != index {
            showing = index
            attempt { try tabs.setSelected(index) }
        }
    }

    override func collect(_ out: inout [Item]) {
        if let tabs {
            out.append(Item(widget: tabs))
        }
    }

    override func applyFrame(_ frame: FrameSpec) {
        if let tabs {
            attempt { try Press.applyFrame(tabs, frame) }
        }
    }

    override func teardown() {
        super.teardown()
        try? tabs?.destroy()
    }
}

// MARK: - Lists

/// How a list's selection is kept: not at all, as one value, or as several.
enum ListSelection<Value: Hashable> {
    case none
    case one(Binding<Value?>)
    case many(Binding<Set<Value>>)

    func tags() -> Set<AnyHashable> {
        switch self {
        case .none: return []
        case .one(let binding): return Set([binding.wrappedValue].compactMap { $0.map { AnyHashable($0) } })
        case .many(let binding): return Set(binding.wrappedValue.map { AnyHashable($0) })
        }
    }

    func set(_ tags: [AnyHashable]) {
        let values = tags.compactMap { $0.base as? Value }
        switch self {
        case .none: break
        case .one(let binding): binding.wrappedValue = values.last
        case .many(let binding): binding.wrappedValue = Set(values)
        }
    }

    var isMany: Bool {
        if case .many = self { return true }
        return false
    }
}

/// Rows of words, of which one or several may be selected. Each row is a
/// `Text` or a `Label`; a row's tag, or its id in a `ForEach`, is the value
/// the selection takes. `.onSubmit` hears a row opened with a double click
/// or Return.
public struct List<SelectionValue: Hashable, Content: View>: View, _LeafView {
    public typealias Body = Never
    let selection: ListSelection<SelectionValue>
    let content: Content

    public init(selection: Binding<SelectionValue?>, @ViewBuilder content: () -> Content) {
        self.selection = .one(selection)
        self.content = content()
    }

    public init(selection: Binding<Set<SelectionValue>>, @ViewBuilder content: () -> Content) {
        self.selection = .many(selection)
        self.content = content()
    }

    func rows(_ env: EnvironmentValues) -> [ItemSpec] {
        described(content, DescribeContext(env: env)).filter { $0.title != nil }
    }

    func _make(_ node: LeafNode<Self>) throws -> PressCore.ListBox {
        let list = try PressCore.ListBox()
        try list.setMultiple(selection.isMany)
        try list.onChanged(
            onMain { [weak node, list] (_: Int) in
                node?.event { view in
                    let rows = view.rows(node?.env ?? EnvironmentValues())
                    let chosen = rows.indices.filter { (try? list.isSelected($0)) ?? false }.compactMap { rows[$0].tag }
                    node?.memory = Set(chosen)
                    view.selection.set(chosen)
                }
            })
        try list.onActivated(onMain { [weak node] (_: Int) in node?.event { _ in node?.env.submitAction?() } })
        return list
    }

    func _update(_ widget: PressCore.ListBox, _ node: LeafNode<Self>) throws {
        let rows = observing(node) { self.rows(node.env) }
        try fill(widget, rows, node)
        let wanted = selection.tags()
        if (node.memory as? Set<AnyHashable>) != wanted {
            node.memory = wanted
            if selection.isMany {
                for (index, row) in rows.enumerated() {
                    try widget.setItemSelected(index, selected: row.tag.map(wanted.contains) ?? false)
                }
            } else {
                try widget.setSelected(rows.firstIndex { $0.tag.map(wanted.contains) ?? false } ?? -1)
            }
        }
    }

    func _frame(_ widget: PressCore.ListBox, _ frame: FrameSpec) throws {
        try frameList(widget, frame)
    }
}

extension List where SelectionValue == Never {
    public init(@ViewBuilder content: () -> Content) {
        self.selection = .none
        self.content = content()
    }
}

extension List {
    public init<Data: RandomAccessCollection, Row: View>(
        _ data: Data, selection: Binding<SelectionValue?>, @ViewBuilder rowContent: @escaping (Data.Element) -> Row
    ) where Content == ForEach<Data, Data.Element.ID, Row>, Data.Element: Identifiable {
        self.init(selection: selection) { ForEach(data, content: rowContent) }
    }

    public init<Data: RandomAccessCollection, Row: View>(
        _ data: Data, selection: Binding<Set<SelectionValue>>, @ViewBuilder rowContent: @escaping (Data.Element) -> Row
    ) where Content == ForEach<Data, Data.Element.ID, Row>, Data.Element: Identifiable {
        self.init(selection: selection) { ForEach(data, content: rowContent) }
    }

    public init<Data: RandomAccessCollection, ID: Hashable, Row: View>(
        _ data: Data, id: KeyPath<Data.Element, ID>, selection: Binding<SelectionValue?>,
        @ViewBuilder rowContent: @escaping (Data.Element) -> Row
    ) where Content == ForEach<Data, ID, Row> {
        self.init(selection: selection) { ForEach(data, id: id, content: rowContent) }
    }
}

extension List where SelectionValue == Never {
    public init<Data: RandomAccessCollection, Row: View>(
        _ data: Data, @ViewBuilder rowContent: @escaping (Data.Element) -> Row
    ) where Content == ForEach<Data, Data.Element.ID, Row>, Data.Element: Identifiable {
        self.init { ForEach(data, content: rowContent) }
    }

    public init<Data: RandomAccessCollection, ID: Hashable, Row: View>(
        _ data: Data, id: KeyPath<Data.Element, ID>, @ViewBuilder rowContent: @escaping (Data.Element) -> Row
    ) where Content == ForEach<Data, ID, Row> {
        self.init { ForEach(data, id: id, content: rowContent) }
    }
}

/// Put rows in a list box, if they are not the rows it has.
@MainActor
func fill<V: _LeafView>(_ list: PressCore.ListBox, _ rows: [ItemSpec], _ node: LeafNode<V>) throws {
    let labels = rows.map { ($0.title ?? "") + "\u{1}" + String($0.icon.rawValue) }
    guard node.labels != labels else { return }
    node.labels = labels
    node.memory = nil
    try list.clear()
    for (index, row) in rows.enumerated() {
        try list.add(row.title ?? "")
        if row.icon != .none {
            try list.setIcon(index, icon: row.icon)
        }
    }
}

@MainActor
func frameList(_ list: PressCore.ListBox, _ frame: FrameSpec) throws {
    if let width = frame.width ?? frame.minWidth {
        try list.setMinWidth(width)
    }
    if let height = frame.height ?? frame.minHeight {
        try list.setMinSize(0, height: height)
    }
    if let stretch = frame.stretch {
        try list.setStretch(stretch)
    }
}

// MARK: - Tables

/// A column of a table: its heading, and what it shows of each row.
public struct TableColumn<RowValue> {
    let title: String
    let width: Int
    let cell: (RowValue) -> String

    public init(_ title: some StringProtocol, value: KeyPath<RowValue, String>) {
        self.title = String(title)
        self.width = 120
        self.cell = { $0[keyPath: value] }
    }

    /// Only the words of the content are shown.
    @MainActor
    public init<Content: View>(_ title: some StringProtocol, @ViewBuilder content: @escaping (RowValue) -> Content) {
        self.title = String(title)
        self.width = 120
        self.cell = { row in MainActor.assumeIsolated { firstText(content(row)) ?? "" } }
    }

    init(title: String, width: Int, cell: @escaping (RowValue) -> String) {
        self.title = title
        self.width = width
        self.cell = cell
    }

    /// How wide the column starts, in pixels. The user can drag it wider.
    public func width(_ width: CGFloat) -> TableColumn<RowValue> {
        TableColumn(title: title, width: pixels(width), cell: cell)
    }
}

@resultBuilder
public enum TableColumnBuilder<RowValue> {
    /// Each column is read as one of this table's, which is how `\.name`
    /// comes to mean a property of its rows.
    public static func buildExpression(_ column: TableColumn<RowValue>) -> TableColumn<RowValue> { column }
    public static func buildBlock(_ columns: TableColumn<RowValue>...) -> [TableColumn<RowValue>] { columns }
}

/// Rows under a row of headings. The columns are those it was first given.
public struct Table<RowValue: Identifiable>: View, _LeafView {
    public typealias Body = Never
    let data: [RowValue]
    let selection: Binding<RowValue.ID?>?
    let columns: [TableColumn<RowValue>]
    var sortable = false

    public init(
        _ data: some RandomAccessCollection<RowValue>, selection: Binding<RowValue.ID?>? = nil,
        @TableColumnBuilder<RowValue> columns: () -> [TableColumn<RowValue>]
    ) {
        self.data = Array(data)
        self.selection = selection
        self.columns = columns()
    }

    /// Let the user sort the rows by clicking a heading. Not in SwiftUI,
    /// where a table is sorted through a binding to its sort order.
    public func sortable(_ sortable: Bool = true) -> Table<RowValue> {
        var copy = self
        copy.sortable = sortable
        return copy
    }

    func _make(_ node: LeafNode<Self>) throws -> PressCore.Table {
        let table = try PressCore.Table()
        for column in columns {
            try table.addColumn(column.title, width: column.width)
        }
        try table.onChanged(
            onMain { [weak node] (index: Int) in
                node?.number = index
                node?.event { view in
                    view.selection?.wrappedValue = view.data.indices.contains(index) ? view.data[index].id : nil
                }
            })
        try table.onActivated(onMain { [weak node] (_: Int) in node?.event { _ in node?.env.submitAction?() } })
        return table
    }

    func _update(_ widget: PressCore.Table, _ node: LeafNode<Self>) throws {
        let rows = data.map { row in columns.map { $0.cell(row) }.joined(separator: "\t") }
        if node.labels != rows {
            node.labels = rows
            node.number = nil
            try widget.clear()
            for row in rows {
                try widget.add(row)
            }
        }
        if node.text != String(sortable) {
            node.text = String(sortable)
            try widget.setSortable(sortable)
        }
        if let selection {
            let wanted = data.firstIndex { $0.id == selection.wrappedValue } ?? -1
            if node.number != wanted {
                node.number = wanted
                try widget.setSelected(wanted)
            }
        }
    }

    func _frame(_ widget: PressCore.Table, _ frame: FrameSpec) throws {
        try frameList(widget, frame)
    }
}
