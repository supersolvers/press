// SPDX-License-Identifier: GPL-3.0-or-later
// The controls: words, buttons, boxes to tick, fields to type in, and the
// rest of what a window is made of.
import Foundation
import PressCore

/// Make a handler press can call out of one that touches views. press calls
/// its handlers on the thread the program runs its windows on, which is the
/// main one.
func onMain(_ body: @escaping @MainActor () -> Void) -> () -> Void {
    { MainActor.assumeIsolated(body) }
}

func onMain<A>(_ body: @escaping @MainActor (A) -> Void) -> (A) -> Void {
    { value in MainActor.assumeIsolated { body(value) } }
}

func onMain<A, B>(_ body: @escaping @MainActor (A, B) -> Void) -> (A, B) -> Void {
    { a, b in MainActor.assumeIsolated { body(a, b) } }
}

func onMain<A, B, C>(_ body: @escaping @MainActor (A, B, C) -> Void) -> (A, B, C) -> Void {
    { a, b, c in MainActor.assumeIsolated { body(a, b, c) } }
}

// MARK: - Words

/// A line of words.
public struct Text: View, _LeafView, _Describable {
    public typealias Body = Never
    let string: String
    var font: Font?

    var _ownFont: Font? { font }

    /// These words in a typeface, whatever the views around them use. On a
    /// canvas, the words are drawn in it.
    public func font(_ font: Font?) -> Text {
        var copy = self
        copy.font = font
        return copy
    }

    /// These words in the heavy cut of their typeface, where it has one.
    public func bold() -> Text {
        font((font ?? .body).bold())
    }

    /// These words in the typeface whose characters are all one width.
    public func monospaced() -> Text {
        font((font ?? .body).monospaced())
    }

    public init(_ content: some StringProtocol) {
        string = String(content)
    }

    public init(verbatim content: String) {
        string = content
    }

    func _make(_ node: LeafNode<Text>) throws -> PressCore.Label {
        node.text = string
        return try PressCore.Label(string)
    }

    func _update(_ widget: PressCore.Label, _ node: LeafNode<Text>) throws {
        if node.text != string {
            node.text = string
            try widget.setText(string)
        }
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        out.append(ItemSpec(.text(string), context))
    }
}

/// Words with a small picture before them. The picture shows in a list,
/// where `systemImage` may be "folder", "doc" or "app".
public struct Label: View, _Describable {
    let title: String
    let icon: Icon
    var png: [UInt8]?

    public init(_ title: some StringProtocol, systemImage: String) {
        self.title = String(title)
        switch systemImage {
        case let name where name.hasPrefix("folder"): icon = .folder
        case let name where name.hasPrefix("doc"): icon = .document
        case let name where name.hasPrefix("app"): icon = .application
        default: icon = .none
        }
    }

    public var body: some View {
        Text(title)
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        var spec = ItemSpec(.text(title), context)
        spec.icon = icon
        spec.png = png
        out.append(spec)
    }
}

/// Room that takes up whatever is spare in a stack, pushing its neighbours
/// apart.
public struct Spacer: View, _LeafView {
    public typealias Body = Never
    let minLength: Int

    public init(minLength: CGFloat? = nil) {
        self.minLength = minLength.map(pixels) ?? 0
    }

    func _make(_ node: LeafNode<Spacer>) throws -> PressCore.Label {
        let spring = try PressCore.Label("")
        try spring.setStretch(true)
        try spring.setMinSize(minLength, height: minLength)
        return spring
    }

    func _update(_ widget: PressCore.Label, _ node: LeafNode<Spacer>) throws {}
}

/// A dividing line between the items of a menu. In a window it is nothing.
public struct Divider: View, _NodeView, _Describable {
    public typealias Body = Never

    public init() {}

    func _makeNode(_ env: EnvironmentValues) -> Node { Node(self, env) }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        out.append(ItemSpec(.divider, context))
    }
}

// MARK: - Buttons

/// What a button is for, where that changes how it behaves.
public struct ButtonRole: Hashable, Sendable {
    let name: String

    /// The button that answers Escape.
    public static let cancel = ButtonRole(name: "cancel")
    public static let destructive = ButtonRole(name: "destructive")
}

/// A push button, or in a menu a command.
public struct Button<Label: View>: View, _LeafView, _Describable {
    public typealias Body = Never
    let role: ButtonRole?
    let action: () -> Void
    let label: Label
    /// A picture in colour before the label: the bytes of a PNG file.
    var png: [UInt8]?

    public init(action: @escaping () -> Void, @ViewBuilder label: () -> Label) {
        self.role = nil
        self.action = action
        self.label = label()
    }

    public init(role: ButtonRole?, action: @escaping () -> Void, @ViewBuilder label: () -> Label) {
        self.role = role
        self.action = action
        self.label = label()
    }

    var title: String { firstText(label) ?? "" }

    func _make(_ node: LeafNode<Self>) throws -> PressCore.Button {
        let button = try PressCore.Button(title)
        node.text = title
        if let png {
            try button.setImage(png)
        }
        try button.onClicked(onMain { [weak node] in node?.event { $0.action() } })
        return button
    }

    func _update(_ widget: PressCore.Button, _ node: LeafNode<Self>) throws {
        let title = self.title
        if node.text != title {
            node.text = title
            try widget.setLabel(title)
        }
        // 1 for the default button, 2 for the one that cancels; and ten more for one that lies flat
        let shortcut = node.env.keyboardShortcut
        let flat = node.env.buttonStyle == .borderless
        let kind = (shortcut == .defaultAction ? 1 : (shortcut == .cancelAction || role == .cancel) ? 2 : 0) + (flat ? 10 : 0)
        if node.number != kind {
            node.number = kind
            try widget.setIsDefault(kind % 10 == 1)
            try widget.setIsCancel(kind % 10 == 2)
            try widget.setFlat(flat)
        }
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        out.append(ItemSpec(.button(title, action), context))
    }
}

extension Button where Label == Text {
    public init(_ title: some StringProtocol, action: @escaping () -> Void) {
        self.init(action: action) { Text(title) }
    }

    public init(_ title: some StringProtocol, role: ButtonRole?, action: @escaping () -> Void) {
        self.init(role: role, action: action) { Text(title) }
    }
}

/// A box that is ticked or not, or in a menu an item with a tick.
public struct Toggle<Label: View>: View, _LeafView, _Describable {
    public typealias Body = Never
    let isOn: Binding<Bool>
    let label: Label

    public init(isOn: Binding<Bool>, @ViewBuilder label: () -> Label) {
        self.isOn = isOn
        self.label = label()
    }

    var title: String { firstText(label) ?? "" }

    func _make(_ node: LeafNode<Self>) throws -> PressCore.Checkbox {
        let box = try PressCore.Checkbox(title)
        node.text = title
        node.number = 0
        try box.onToggled(
            onMain { [weak node] checked in
                node?.number = checked ? 1 : 0
                node?.event { $0.isOn.wrappedValue = checked }
            })
        return box
    }

    func _update(_ widget: PressCore.Checkbox, _ node: LeafNode<Self>) throws {
        let title = self.title
        if node.text != title {
            node.text = title
            // a checkbox's caption is fixed when it is made
            try? PressCore.Button(handle: widget.handle).setLabel(title)
        }
        let wanted = isOn.wrappedValue ? 1 : 0
        if node.number != wanted {
            node.number = wanted
            try widget.setChecked(wanted == 1)
        }
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        var spec = ItemSpec(.toggle(title, isOn), context)
        spec.checked = isOn.wrappedValue
        out.append(spec)
    }
}

extension Toggle where Label == Text {
    public init(_ title: some StringProtocol, isOn: Binding<Bool>) {
        self.init(isOn: isOn) { Text(title) }
    }
}

// MARK: - Typing

/// What the three kinds of field share: they show a binding's text, and
/// write to it what the user types.
@MainActor
protocol _FieldView: _LeafView where Widget: PressCore.TextField {
    var text: Binding<String> { get }
    var placeholder: String { get }
}

extension _FieldView {
    func _update(_ widget: Widget, _ node: LeafNode<Self>) throws {
        try updateField(widget, node)
    }

    func connect(_ field: Widget, _ node: LeafNode<Self>) throws {
        node.text = text.wrappedValue
        try field.onChanged(
            onMain { [weak node] (typed: String) in
                node?.text = typed
                node?.event { $0.text.wrappedValue = typed }
            })
    }

    /// Bring the text and placeholder in step. Says whether the text was
    /// replaced, which puts the caret back at the start.
    @discardableResult
    func updateField(_ widget: Widget, _ node: LeafNode<Self>) throws -> Bool {
        var replaced = false
        let wanted = text.wrappedValue
        if node.text != wanted {
            node.text = wanted
            try widget.setText(wanted)
            replaced = true
        }
        if node.labels.first != placeholder {
            node.labels = [placeholder]
            try widget.setPlaceholder(placeholder)
        }
        return replaced
    }

    func _frame(_ widget: Widget, _ frame: FrameSpec) throws {
        if let width = frame.width ?? frame.minWidth {
            try widget.setMinWidth(width)
        }
        try applyFrame(widget, FrameSpec(height: frame.height, minHeight: frame.minHeight, stretch: frame.stretch))
    }
}

/// One line to type in.
public struct TextField: View, _FieldView {
    public typealias Body = Never
    let text: Binding<String>
    let placeholder: String

    /// The title is shown greyed while the field is empty.
    public init(_ title: some StringProtocol, text: Binding<String>, prompt: Text? = nil) {
        self.text = text
        self.placeholder = prompt?.string ?? String(title)
    }

    func _make(_ node: LeafNode<TextField>) throws -> PressCore.TextField {
        let field = try PressCore.TextField(text.wrappedValue)
        try connect(field, node)
        try field.onSubmitted(onMain { [weak node] (_: String) in node?.event { _ in node?.env.submitAction?() } })
        return field
    }
}

/// One line to type a password in, shown as bullets.
public struct SecureField: View, _FieldView {
    public typealias Body = Never
    let text: Binding<String>
    let placeholder: String

    public init(_ title: some StringProtocol, text: Binding<String>, prompt: Text? = nil) {
        self.text = text
        self.placeholder = prompt?.string ?? String(title)
    }

    func _make(_ node: LeafNode<SecureField>) throws -> PressCore.TextField {
        let field = try PressCore.TextField(text.wrappedValue)
        try field.setPassword(true)
        try connect(field, node)
        try field.onSubmitted(onMain { [weak node] (_: String) in node?.event { _ in node?.env.submitAction?() } })
        return field
    }
}

/// Where the caret is in a text editor, or what is selected.
public struct TextSelection: Hashable, Sendable {
    public enum Indices: Hashable, Sendable {
        case selection(Range<String.Index>)
    }

    public var indices: Indices

    /// The words from one place to another.
    public init(range: Range<String.Index>) {
        indices = .selection(range)
    }

    /// The caret alone, at a place.
    public init(insertionPoint: String.Index) {
        indices = .selection(insertionPoint..<insertionPoint)
    }

    /// Whether this is the caret alone, with nothing selected.
    public var isInsertion: Bool {
        if case .selection(let range) = indices { return range.isEmpty }
        return false
    }

    /// The selection's ends as byte offsets into `text`'s UTF-8.
    func offsets(in text: String) -> (Int, Int) {
        guard case .selection(let range) = indices else { return (0, 0) }
        let utf8 = text.utf8
        let clamp = { (index: String.Index) in min(max(index, utf8.startIndex), utf8.endIndex) }
        return (utf8.distance(from: utf8.startIndex, to: clamp(range.lowerBound)), utf8.distance(from: utf8.startIndex, to: clamp(range.upperBound)))
    }

    /// A selection from byte offsets into `text`'s UTF-8.
    init(offsets: (Int, Int), in text: String) {
        let utf8 = text.utf8
        let place = { (offset: Int) in utf8.index(utf8.startIndex, offsetBy: min(max(offset, 0), utf8.count)) }
        indices = .selection(place(min(offsets.0, offsets.1))..<place(max(offsets.0, offsets.1)))
    }
}

/// Several lines to type in. With a selection binding, it says where the
/// caret is as the user moves it, and puts the caret where the program sets
/// it.
public struct TextEditor: View, _FieldView {
    public typealias Body = Never
    let text: Binding<String>
    let selection: Binding<TextSelection?>?
    var placeholder: String { "" }

    public init(text: Binding<String>) {
        self.text = text
        self.selection = nil
    }

    public init(text: Binding<String>, selection: Binding<TextSelection?>) {
        self.text = text
        self.selection = selection
    }

    func _make(_ node: LeafNode<TextEditor>) throws -> PressCore.TextArea {
        let area = try PressCore.TextArea(text.wrappedValue)
        try connect(area, node)
        try area.onSelected(
            onMain { [weak node] (start: Int, end: Int) in
                guard let node else { return }
                // told of a move the program has since overtaken, by changing
                // the text or the selection in answer to the same keystroke
                if let area = node.widget, let now = try? (area.selectionStart(), area.selectionEnd()), now != (start, end) {
                    return
                }
                node.memory = (start, end)
                node.event { view in
                    view.selection?.wrappedValue = TextSelection(offsets: (start, end), in: node.text ?? view.text.wrappedValue)
                }
            })
        return area
    }

    func _update(_ widget: PressCore.TextArea, _ node: LeafNode<TextEditor>) throws {
        let replaced = try updateField(widget, node)
        guard let wanted = selection?.wrappedValue else { return }
        let offsets = wanted.offsets(in: text.wrappedValue)
        // put the caret where the program says, unless it is there already
        let known = node.memory as? (Int, Int)
        if replaced || known.map({ $0 != offsets }) ?? true {
            node.memory = offsets
            try widget.select(offsets.0, to: offsets.1)
        }
    }
}

// MARK: - Numbers

/// A number picked by sliding a knob along a groove.
public struct Slider: View, _LeafView {
    public typealias Body = Never
    let get: () -> Double
    let set: (Double) -> Void
    let low: Double
    let high: Double
    /// How many places the knob can rest between the ends.
    let steps: Int

    public init<V: BinaryFloatingPoint>(value: Binding<V>, in bounds: ClosedRange<V> = 0...1, step: V? = nil) {
        get = { Double(value.wrappedValue) }
        set = { value.wrappedValue = V($0) }
        low = Double(bounds.lowerBound)
        high = Double(bounds.upperBound)
        let span = high - low
        steps = step.map { max(1, Int((span / Double($0)).rounded())) } ?? 100
    }

    func position(_ value: Double) -> Int {
        high > low ? Int(((value - low) / (high - low) * Double(steps)).rounded()) : 0
    }

    func _make(_ node: LeafNode<Slider>) throws -> PressCore.Slider {
        let slider = try PressCore.Slider(.horizontal)
        try slider.onChanged(
            onMain { [weak node] (position: Int) in
                node?.number = position
                node?.event { $0.set($0.low + ($0.high - $0.low) * Double(position) / Double($0.steps)) }
            })
        return slider
    }

    func _update(_ widget: PressCore.Slider, _ node: LeafNode<Slider>) throws {
        if node.labels.first != String(steps) {
            node.labels = [String(steps)]
            try widget.setMax(steps)
        }
        let wanted = min(max(position(get()), 0), steps)
        if node.number != wanted {
            node.number = wanted
            try widget.setValue(wanted)
        }
    }

    func _frame(_ widget: PressCore.Slider, _ frame: FrameSpec) throws {
        if let width = frame.width ?? frame.minWidth {
            try widget.setLength(width)
        }
        if let stretch = frame.stretch {
            try widget.setStretch(stretch)
        }
    }
}

/// A bar that shows how far along some work is, or with no value, stripes
/// that move while work of no known length goes on.
public struct ProgressView: View, _LeafView {
    public typealias Body = Never
    /// Thousandths done, or nil when there is no telling.
    let amount: Int?

    public init() {
        amount = nil
    }

    public init<V: BinaryFloatingPoint>(value: V?, total: V = 1.0) {
        amount = value.map { total > 0 ? Int((Double($0) / Double(total) * 1000).rounded()) : 0 }
    }

    func _make(_ node: LeafNode<ProgressView>) throws -> PressCore.ProgressBar {
        let bar = try PressCore.ProgressBar()
        try bar.setMax(1000)
        node.number = 0
        return bar
    }

    func _update(_ widget: PressCore.ProgressBar, _ node: LeafNode<ProgressView>) throws {
        let wanted = amount ?? -1
        if node.number != wanted {
            node.number = wanted
            try widget.setBusy(amount == nil)
            try widget.setValue(min(max(amount ?? 0, 0), 1000))
        }
    }

    func _frame(_ widget: PressCore.ProgressBar, _ frame: FrameSpec) throws {
        if let width = frame.width ?? frame.minWidth {
            try widget.setLength(width)
        }
        if let stretch = frame.stretch {
            try widget.setStretch(stretch)
        }
    }
}

// MARK: - Choosing one of several

/// How a picker is shown.
public struct PickerStyle: Hashable, Sendable {
    let name: String

    /// A box showing the choice, which pops the others up when pressed.
    public static let menu = PickerStyle(name: "menu")
    public static let automatic = PickerStyle(name: "menu")
    /// A column of radio buttons.
    public static let radioGroup = PickerStyle(name: "radio")
}

/// A choice of one among several. Each choice is a `Text` with a `tag`, or
/// comes from a `ForEach`, whose ids are the tags. In a menu it is an item
/// that opens the choices beside it, the one chosen ticked.
public struct Picker<SelectionValue: Hashable, Content: View>: View, _Describable {
    let title: String
    let selection: Binding<SelectionValue>
    let content: Content
    @Environment(\.pickerStyle) private var style

    public init(_ title: some StringProtocol, selection: Binding<SelectionValue>, @ViewBuilder content: () -> Content) {
        self.title = String(title)
        self.selection = selection
        self.content = content()
    }

    /// The choices: what each is called, and the value it stands for.
    func choices(_ env: EnvironmentValues) -> [ItemSpec] {
        described(content, DescribeContext(env: env)).filter { $0.title != nil }
    }

    func choose(_ spec: ItemSpec) {
        if let value = spec.tag?.base as? SelectionValue {
            selection.wrappedValue = value
        }
    }

    func isChosen(_ spec: ItemSpec) -> Bool {
        spec.tag != nil && spec.tag == hashable(selection.wrappedValue)
    }

    public var body: some View {
        if style == .radioGroup {
            _RadioPicker(picker: self)
        } else {
            _PopupPicker(picker: self)
        }
    }

    func _describe(_ out: inout [ItemSpec], _ context: DescribeContext) {
        let items = choices(context.env).map { choice -> ItemSpec in
            var item = ItemSpec(.button(choice.title ?? "", { self.choose(choice) }), context)
            item.checked = isChosen(choice)
            item.enabled = choice.enabled
            item.shortcut = nil
            return item
        }
        out.append(ItemSpec(.menu(title, items), context))
    }
}

/// A picker as a box that pops its choices up.
struct _PopupPicker<SelectionValue: Hashable, Content: View>: View, _LeafView {
    typealias Body = Never
    let picker: Picker<SelectionValue, Content>

    func _make(_ node: LeafNode<Self>) throws -> PressCore.PopupButton {
        let popup = try PressCore.PopupButton()
        try popup.onChanged(
            onMain { [weak node] (index: Int) in
                node?.number = index
                node?.event { view in
                    let choices = view.picker.choices(node?.env ?? EnvironmentValues())
                    if choices.indices.contains(index) {
                        view.picker.choose(choices[index])
                    }
                }
            })
        return popup
    }

    func _update(_ widget: PressCore.PopupButton, _ node: LeafNode<Self>) throws {
        let choices = observing(node) { picker.choices(node.env) }
        let labels = choices.map { $0.title ?? "" }
        if node.labels != labels {
            node.labels = labels
            node.number = nil
            try widget.clear()
            for label in labels {
                try widget.add(label)
            }
        }
        if let wanted = choices.firstIndex(where: picker.isChosen), node.number != wanted {
            node.number = wanted
            try widget.setSelected(wanted)
        }
    }
}

/// A picker as a column of radio buttons.
struct _RadioPicker<SelectionValue: Hashable, Content: View>: View, _LeafView {
    typealias Body = Never
    let picker: Picker<SelectionValue, Content>

    func _make(_ node: LeafNode<Self>) throws -> PressCore.Stack {
        let column = try PressCore.Stack(.vertical)
        try column.setSpacing(4)
        return column
    }

    func _update(_ widget: PressCore.Stack, _ node: LeafNode<Self>) throws {
        let choices = observing(node) { picker.choices(node.env) }
        let labels = choices.map { $0.title ?? "" }
        var radios = node.memory as? [PressCore.RadioButton] ?? []
        if node.labels != labels {
            node.labels = labels
            node.number = nil
            for radio in radios {
                try? radio.destroy()
            }
            radios = []
            for (index, label) in labels.enumerated() {
                let radio = try PressCore.RadioButton(label)
                try radio.onChosen(
                    onMain { [weak node] in
                        node?.number = index
                        node?.event { view in
                            let choices = view.picker.choices(node?.env ?? EnvironmentValues())
                            if choices.indices.contains(index) {
                                view.picker.choose(choices[index])
                            }
                        }
                    })
                try widget.add(radio)
                radios.append(radio)
            }
            node.memory = radios
        }
        if let wanted = choices.firstIndex(where: picker.isChosen), node.number != wanted, radios.indices.contains(wanted) {
            node.number = wanted
            try radios[wanted].setChecked(true)
        }
    }
}
