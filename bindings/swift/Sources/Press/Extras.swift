// SPDX-License-Identifier: GPL-3.0-or-later
// What a whole program has round its dialogs: a box with a title, a bar of
// tools and a bar of news, buttons that lie flat, a word of help, a number
// with arrows to step it, and a text field with a list of things it might
// say. Where SwiftUI has the thing, it is under SwiftUI's name.
import Foundation
import PressCore

// MARK: - Boxes and bars

/// A box with a title, round things that belong together.
public struct GroupBox<Content: View>: View, _StackView {
    public typealias Body = Never
    let title: String
    let content: Content

    public init(_ title: some StringProtocol = "", @ViewBuilder content: () -> Content) {
        self.title = String(title)
        self.content = content()
    }

    var _orientation: Orientation { .vertical }
    var _align: Align { .start }
    var _spacing: Int { 8 }
    var _padding: Int { 0 }
    var _content: any View { content }
    var _dress: StackDress { .group(title) }
}

/// A row on a bar the style draws.
struct _Bar<Content: View>: View, _StackView {
    typealias Body = Never
    let dress: StackDress
    let content: Content

    var _orientation: Orientation { .horizontal }
    var _align: Align { .center }
    var _spacing: Int { dress == .toolbar ? 2 : (dress == .buttons ? 10 : 12) }
    var _padding: Int { 0 }
    var _content: any View { content }
    var _dress: StackDress { dress }
}

/// A bar of tools, for the top of a window. The buttons in it lie flat:
/// they have no edge until the pointer is over them. Not in SwiftUI, which
/// hangs its toolbars on the window; put this at the top of a column.
public struct Toolbar<Content: View>: View {
    let content: Content

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    public var body: some View {
        _Bar(dress: .toolbar, content: content).buttonStyle(.borderless)
    }
}

/// A bar of news, for the foot of a window. Not in SwiftUI; put this at
/// the bottom of a column. With panels, each thing in it is shown in a
/// cell of its own, the last reaching the end of the bar.
public struct StatusBar<Content: View>: View {
    let panels: Bool
    let content: Content

    public init(panels: Bool = false, @ViewBuilder content: () -> Content) {
        self.panels = panels
        self.content = content()
    }

    public var body: some View {
        _Bar(dress: .status(panels), content: content)
    }
}

/// The row of buttons at the foot of a dialog. Put the button that agrees
/// first, then the one that calls it off, then any others: they are shown
/// in the order the style of the program has them (OK then Cancel in the
/// manner of Windows, Cancel then OK in that of the Macintosh), at the far
/// end of the row. Not in SwiftUI, which orders an alert's buttons itself.
public struct ButtonRow<Content: View>: View {
    let content: Content

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    public var body: some View {
        _Bar(dress: .buttons, content: content)
    }
}

extension Label {
    /// A row of a list with a picture in colour before its title: the
    /// bytes of a PNG file, up to 16 pixels each way.
    public init(_ title: some StringProtocol, png: [UInt8]) {
        self.init(title, systemImage: "")
        self.png = png
    }
}

// MARK: - Buttons

/// How the buttons inside are drawn.
public struct ButtonStyle: Hashable, Sendable {
    let name: String

    /// As the style of the program draws a button.
    public static let automatic = ButtonStyle(name: "automatic")
    public static let bordered = ButtonStyle(name: "automatic")
    /// Flat, with no edge until the pointer is over it or down on it: a tool in a bar.
    public static let borderless = ButtonStyle(name: "borderless")
}

private enum ButtonStyleKey: EnvironmentKey { static let defaultValue = ButtonStyle.automatic }
private enum HelpKey: EnvironmentKey { static let defaultValue: String? = nil }

extension EnvironmentValues {
    var buttonStyle: ButtonStyle {
        get { self[ButtonStyleKey.self] }
        set { self[ButtonStyleKey.self] = newValue }
    }

    var help: String? {
        get { self[HelpKey.self] }
        set { self[HelpKey.self] = newValue }
    }
}

extension View {
    /// How the buttons inside are drawn.
    public func buttonStyle(_ style: ButtonStyle) -> some View {
        transformingEnvironment { $0.buttonStyle = style }
    }

    /// A word of help, shown beside the pointer when it rests on the view.
    public func help(_ text: some StringProtocol) -> some View {
        transformingEnvironment { $0.help = String(text) }
    }
}

extension Button where Label == Text {
    /// A button with a picture in colour before its title: the bytes of a
    /// PNG file. With an empty title it is the picture alone.
    public init(_ title: some StringProtocol, png: [UInt8], action: @escaping () -> Void) {
        self.init(action: action) { Text(title) }
        self.png = png
    }
}

// MARK: - Steppers

/// A whole number with arrows beside it, to step it down and up.
public struct Stepper<Label: View>: View {
    let label: Label
    let value: Binding<Int>
    let bounds: ClosedRange<Int>
    let step: Int

    public init(value: Binding<Int>, in bounds: ClosedRange<Int> = 0...100, step: Int = 1, @ViewBuilder label: () -> Label) {
        self.label = label()
        self.value = value
        self.bounds = bounds
        self.step = step
    }

    public var body: some View {
        HStack(spacing: 8) {
            label
            _StepperField(value: value, bounds: bounds, step: step)
        }
    }
}

extension Stepper where Label == Text {
    public init(_ title: some StringProtocol, value: Binding<Int>, in bounds: ClosedRange<Int> = 0...100, step: Int = 1) {
        self.init(value: value, in: bounds, step: step) { Text(title) }
    }
}

/// The number and its arrows.
struct _StepperField: View, _LeafView {
    typealias Body = Never
    let value: Binding<Int>
    let bounds: ClosedRange<Int>
    let step: Int

    func _make(_ node: LeafNode<_StepperField>) throws -> PressCore.Stepper {
        let stepper = try PressCore.Stepper()
        try stepper.onChanged(
            onMain { [weak node] (to: Int) in
                node?.number = to
                node?.event { $0.value.wrappedValue = to }
            })
        return stepper
    }

    func _update(_ widget: PressCore.Stepper, _ node: LeafNode<_StepperField>) throws {
        let limits = "\(bounds.lowerBound) \(bounds.upperBound) \(step)"
        if node.labels.first != limits {
            node.labels = [limits]
            // the most first, so that the least is never above it on the way
            try widget.setMax(max(bounds.upperBound, bounds.lowerBound))
            try widget.setMin(bounds.lowerBound)
            try widget.setMax(bounds.upperBound)
            try widget.setStep(max(1, step))
            node.number = Int.min
        }
        let wanted = min(max(value.wrappedValue, bounds.lowerBound), bounds.upperBound)
        if node.number != wanted {
            node.number = wanted
            try widget.setValue(wanted)
        }
    }
}

// MARK: - Combo boxes

/// One line of text to type, with a list of things it might say dropped
/// from a button at its end. Not in SwiftUI. The choices may change
/// from one drawing of the view to the next.
public struct ComboBox: View, _FieldView {
    public typealias Body = Never
    let text: Binding<String>
    let placeholder: String
    let choices: [String]

    public init(_ title: some StringProtocol, text: Binding<String>, choices: [String]) {
        self.text = text
        self.placeholder = String(title)
        self.choices = choices
    }

    func _make(_ node: LeafNode<ComboBox>) throws -> PressCore.ComboBox {
        let box = try PressCore.ComboBox(text.wrappedValue)
        try connect(box, node)
        try box.onSubmitted(onMain { [weak node] (_: String) in node?.event { _ in node?.env.submitAction?() } })
        return box
    }

    func _update(_ widget: PressCore.ComboBox, _ node: LeafNode<ComboBox>) throws {
        try updateField(widget, node)
        if (node.memory as? [String]) != choices {
            node.memory = choices
            try widget.clear()
            for choice in choices {
                try widget.add(choice)
            }
        }
    }
}
