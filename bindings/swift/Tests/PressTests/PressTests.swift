// SPDX-License-Identifier: GPL-3.0-or-later
// Press, exercised with no display: views are put in a window, the
// window is typed into and clicked through press's test hooks, and what the
// views then say is checked against what the widgets show.
import Foundation
import PressCore
import Observation
import XCTest

@testable import Press

@Observable final class Model {
    var name = "one"
    var unread = 0
}

@Observable final class EditedDoc {
    var text = "hello world"
    var selection: TextSelection?
}

@Observable final class Settings {
    var name = "env"
}

@MainActor
final class PressTests: XCTestCase {
    override func setUp() async throws {
        setenv("PRESS_BACKEND", "headless", 1)
        try PressCore.App.initialize()
    }

    override func tearDown() async throws {
        Runtime.shared.reset()
        try PressCore.App.shutdown()
    }

    /// Put a view in a window and hand back the window.
    private func mount(_ view: some View) throws -> PressCore.Window {
        let host = AppHost("Test", view)
        Runtime.shared.launch(host)
        return try XCTUnwrap(host.windows.first).window
    }

    /// The widget with the keyboard, as a text field.
    private func field(_ window: PressCore.Window) throws -> PressCore.TextField {
        PressCore.TextField(handle: try XCTUnwrap(try window.focused()).handle)
    }

    private func picture(_ window: PressCore.Window) throws -> Data {
        let path = NSTemporaryDirectory() + "pressui-\(getpid())-\(UUID().uuidString).png"
        defer { try? FileManager.default.removeItem(atPath: path) }
        try window.snapshot(path, scale: 1)
        return try XCTUnwrap(FileManager.default.contents(atPath: path))
    }

    // MARK: A canvas that grows

    func testACanvasThatGrowsIsToldItsRoom() throws {
        final class Seen { var sizes: [CGSize] = []; var drawn: [CGSize] = [] }
        let seen = Seen()
        let window = try mount(
            VStack {
                Text("A caption wider than the canvas asks to be, so that it has room to grow")
                Canvas { _, size in seen.drawn.append(size) }
                    .onResize { seen.sizes.append($0) }
                    .frame(minWidth: 40, maxWidth: .infinity, minHeight: 30, maxHeight: .infinity)
            })
        try PressCore.App.start()
        defer { try? PressCore.App.stop() }
        for _ in 0..<20 where seen.sizes.isEmpty {
            _ = try PressCore.App.dispatch()
        }
        let room = try XCTUnwrap(seen.sizes.last, "told its room when first laid out")
        XCTAssertGreaterThan(room.width, 40, "it grew across to the caption's width")
        XCTAssertEqual(room.height, 30)
        XCTAssertEqual(seen.drawn.last, room, "and drawn at that size")
        XCTAssertFalse(try picture(window).isEmpty)
    }

    // MARK: Full screen

    func testABindingPutsTheWindowInFullScreen() throws {
        let model = FullScreenModel()
        let window = try mount(FullScreenShown(model: model))
        XCTAssertFalse(try window.fullScreen)
        Runtime.shared.event { model.full = true }
        try PressCore.App.start()
        defer { try? PressCore.App.stop() }
        _ = try PressCore.App.dispatch()
        XCTAssertTrue(try window.fullScreen, "the window follows the binding")
        Runtime.shared.event { model.full = false }
        _ = try PressCore.App.dispatch()
        XCTAssertFalse(try window.fullScreen)
    }

    // MARK: State

    func testStateDrivesTheWindow() throws {
        struct Counter: View {
            @State private var count = 0
            var body: some View {
                VStack {
                    Text("Count: \(count)")
                    Button("More") { count += 1 }.keyboardShortcut(.defaultAction)
                }
            }
        }
        let window = try mount(Counter())
        let before = try picture(window)
        try window.pressKey(.return, modifiers: 0)
        let after = try picture(window)
        XCTAssertNotEqual(before, after, "the label was drawn again with the new count")

        // the same window as one made by hand showing the count it reached
        struct Fixed: View {
            var body: some View {
                VStack {
                    Text("Count: 1")
                    Button("More") {}.keyboardShortcut(.defaultAction)
                }
            }
        }
        Runtime.shared.reset()
        XCTAssertEqual(try picture(try mount(Fixed())), after)
    }

    func testTextFieldWritesToItsBinding() throws {
        final class Seen { var values: [String] = [] }
        let seen = Seen()
        struct Form: View {
            let seen: Seen
            @State private var name = ""
            var body: some View {
                VStack(alignment: .leading) {
                    TextField("Name", text: $name)
                    Text(name.isEmpty ? "Nobody" : "Hello, \(name)")
                    Button("Clear") { name = "" }.disabled(name.isEmpty)
                }
                .onChange(of: name) { seen.values.append(name) }
            }
        }
        let window = try mount(Form(seen: seen))
        try window.typeText("Zoë")
        XCTAssertEqual(seen.values, ["Zoë"])
        XCTAssertEqual(try field(window).text, "Zoë")
        // Tab to the button, now enabled, and press it: the field is emptied
        try window.pressKey(.tab, modifiers: 0)
        try window.typeText(" ")
        XCTAssertEqual(seen.values.last, "")
        try window.pressKey(.tab, modifiers: Modifier.shift.rawValue)
        XCTAssertEqual(try field(window).text, "")
    }

    func testFocusSurvivesAnUpdate() throws {
        struct Two: View {
            @State private var first = ""
            @State private var second = ""
            var body: some View {
                VStack {
                    TextField("First", text: $first)
                    TextField("Second", text: $second)
                    Text(first + second)
                }
            }
        }
        let window = try mount(Two())
        try window.typeText("a")
        try window.pressKey(.tab, modifiers: 0)
        // each letter updates the views; the caret must stay where it is
        try window.typeText("xyz")
        XCTAssertEqual(try field(window).text, "xyz")
    }

    // MARK: Structure

    func testConditionalsAddAndRemoveWidgets() throws {
        struct Details: View {
            @State private var shown = false
            @State private var note = ""
            var body: some View {
                VStack(alignment: .leading) {
                    Toggle("Show details", isOn: $shown)
                    if shown {
                        Text("Details")
                        TextField("Note", text: $note)
                    } else {
                        Text("Nothing to see")
                    }
                    Text("End")
                }
            }
        }
        struct Open: View {
            var body: some View {
                VStack(alignment: .leading) {
                    Toggle("Show details", isOn: .constant(true))
                    Text("Details")
                    TextField("Note", text: .constant(""))
                    Text("End")
                }
            }
        }
        // Every window has a menu bar, 20 pixels deep, above its content.
        let window = try mount(Details())
        XCTAssertNil(try window.focused(), "no field yet")
        try window.pressKey(.tab, modifiers: 0)  // to the checkbox
        try window.typeText(" ")
        try window.pressKey(.tab, modifiers: 0)  // to the field that came in
        try window.typeText("hi")
        XCTAssertEqual(try field(window).text, "hi")
        try window.pressKey(.tab, modifiers: Modifier.shift.rawValue)
        try window.typeText(" ")  // hidden
        try window.typeText(" ")  // and shown again
        try window.pressKey(.tab, modifiers: 0)
        XCTAssertEqual(try field(window).text, "hi", "a new field, showing the state its view kept")
    }

    func testWidgetsComeInAtTheirPlace() throws {
        struct Details: View {
            @State private var shown = false
            var body: some View {
                VStack(alignment: .leading) {
                    Toggle("Show details", isOn: $shown)
                    if shown {
                        Text("Details")
                        Text("More details")
                    }
                    Text("End")
                }
            }
        }
        struct Open: View {
            var body: some View {
                VStack(alignment: .leading) {
                    Toggle("Show details", isOn: .constant(true))
                    Text("Details")
                    Text("More details")
                    Text("End")
                }
            }
        }
        // Every window has a menu bar, 20 pixels deep, above its content.
        let window = try mount(Details())
        try window.clickAt(8, y: 28)  // the checkbox
        let shown = try picture(window)
        Runtime.shared.reset()
        // the window made closed is the size of less content, so compare
        // with one of that size
        let fixed = try mount(Open())
        XCTAssertNotEqual(try picture(fixed), shown)
        Runtime.shared.reset()
        let host = AppHost("Test", Details())
        Runtime.shared.launch(host)
        let again = try XCTUnwrap(host.windows.first).window
        try again.clickAt(8, y: 28)
        XCTAssertEqual(try picture(again), shown)
        try again.clickAt(8, y: 28)
        try again.clickAt(8, y: 28)
        XCTAssertEqual(try picture(again), shown, "taken out and put back, they are where they were")
    }

    func testForEachKeepsStateWithItsElement() throws {
        struct Row: View {
            let name: String
            @State private var note = ""
            var body: some View {
                HStack {
                    Text(name)
                    TextField("note", text: $note)
                }
            }
        }
        struct Rows: View {
            @State private var names = ["a", "b", "c"]
            var body: some View {
                VStack {
                    ForEach(names, id: \.self) { Row(name: $0) }
                    Button("Turn round") { names.reverse() }.keyboardShortcut(.defaultAction)
                    Button("Drop b") { names.removeAll { $0 == "b" } }.keyboardShortcut(.cancelAction)
                }
            }
        }
        let window = try mount(Rows())
        try window.typeText("first")  // into row a's field
        let a = try field(window)
        try window.pressKey(.return, modifiers: 0)  // c, b, a
        XCTAssertEqual(try field(window).handle, a.handle, "the same widget, moved")
        XCTAssertEqual(try a.text, "first")
        try window.pressKey(.escape, modifiers: 0)  // c, a
        XCTAssertEqual(try a.text, "first")
        try window.pressKey(.tab, modifiers: 0)
        try window.pressKey(.tab, modifiers: 0)
        XCTAssertNotEqual(try window.focused()?.handle, a.handle)
    }

    // MARK: Observation

    func testObservableObjectsAreWatched() throws {
        struct Shows: View {
            let model: Model
            var body: some View { TextField("", text: .constant(model.name)) }
        }
        struct Edits: View {
            @Bindable var model: Model
            var body: some View { TextField("", text: $model.name) }
        }
        let model = Model()
        let window = try mount(Shows(model: model))
        XCTAssertEqual(try field(window).text, "one")
        Runtime.shared.event { model.name = "two" }
        XCTAssertEqual(try field(window).text, "two")
        // changed outside any handler: drawn again on the next turn of the loop
        try PressCore.App.start()
        defer { try? PressCore.App.stop() }
        model.name = "three"
        XCTAssertEqual(try field(window).text, "two")
        _ = try PressCore.App.dispatch()
        XCTAssertEqual(try field(window).text, "three")

        Runtime.shared.reset()
        let editing = try mount(Edits(model: model))
        try editing.pressKey(.end, modifiers: 0)
        try editing.typeText("!")
        XCTAssertEqual(model.name, "three!")
    }

    func testEnvironmentIsHandedDown() throws {
        struct Leaf: View {
            @Environment(Settings.self) private var settings
            @Environment(\.isEnabled) private var isEnabled
            var body: some View { TextField("", text: .constant("\(settings.name) \(isEnabled)")) }
        }
        let window = try mount(VStack { Leaf() }.disabled(true).environment(Settings()))
        // a disabled field does not take the keyboard, so find it by looking
        XCTAssertNil(try window.focused())
        Runtime.shared.reset()
        let enabled = try mount(VStack { Leaf() }.environment(Settings()))
        XCTAssertEqual(try field(enabled).text, "env true")
    }

    // MARK: Controls

    func testPickerAndRadioGroup() throws {
        final class Seen { var value = "" }
        let seen = Seen()
        struct Choice: View {
            let seen: Seen
            let style: PickerStyle
            @State private var flavour = "b"
            var body: some View {
                VStack(alignment: .leading) {
                    Picker("Flavour", selection: $flavour) {
                        Text("Apple").tag("a")
                        Text("Banana").tag("b")
                        Text("Cherry").tag("c")
                    }
                    .pickerStyle(style)
                    TextField("", text: .constant(flavour))
                }
                .onChange(of: flavour, initial: true) { seen.value = flavour }
            }
        }
        let window = try mount(Choice(seen: seen, style: .radioGroup))
        XCTAssertEqual(seen.value, "b")
        // radio buttons: Tab from the field goes to the chosen one, Down moves on
        try window.pressKey(.tab, modifiers: 0)
        try window.pressKey(.down, modifiers: 0)
        XCTAssertEqual(seen.value, "c")
        try window.pressKey(.up, modifiers: 0)
        try window.pressKey(.up, modifiers: 0)
        XCTAssertEqual(seen.value, "a")

        Runtime.shared.reset()
        let popup = try mount(Choice(seen: seen, style: .menu))
        XCTAssertEqual(seen.value, "b")
        XCTAssertEqual(try field(popup).text, "b")
    }

    func testListSelection() throws {
        struct Fruit: Identifiable { let id: Int; let name: String }
        final class Seen { var opened = 0 }
        let seen = Seen()
        struct Fruits: View {
            let seen: Seen
            @State private var fruit = [Fruit(id: 10, name: "Apple"), Fruit(id: 20, name: "Banana"), Fruit(id: 30, name: "Cherry")]
            @State private var chosen: Int? = 20
            var body: some View {
                VStack(alignment: .leading) {
                    List(fruit, selection: $chosen) { Text($0.name) }
                        .onSubmit { seen.opened += 1 }
                    TextField("", text: .constant(chosen.map(String.init) ?? "none"))
                    Button("Remove") {
                        fruit.removeAll { $0.id == chosen }
                        chosen = nil
                    }
                    .keyboardShortcut(.defaultAction)
                }
            }
        }
        let window = try mount(Fruits(seen: seen))
        XCTAssertEqual(try field(window).text, "20")
        let text = try field(window)
        try window.clickAt(20, y: 28)  // the first row, under the menu bar
        XCTAssertEqual(try text.text, "10")
        try window.pressKey(.down, modifiers: 0)
        try window.pressKey(.down, modifiers: 0)
        XCTAssertEqual(try text.text, "30")
        try window.pressKey(.return, modifiers: 0)  // opens the row, and presses the default button
        XCTAssertEqual(seen.opened, 1)
        XCTAssertEqual(try text.text, "none", "Cherry was removed, and nothing is selected")
    }

    func testTableAndTextEditor() throws {
        struct File: Identifiable { let id: String; let size: Int }
        struct Files: View {
            @State private var files = [File(id: "notes.txt", size: 12), File(id: "todo.txt", size: 3)]
            @State private var chosen: String?
            @State private var notes = "first line"
            var body: some View {
                VStack(alignment: .leading) {
                    Table(files, selection: $chosen) {
                        TableColumn("Name", value: \.id)
                        TableColumn("Size") { Text("\($0.size) K") }.width(60)
                    }
                    TextEditor(text: $notes)
                    TextField("", text: .constant("\(chosen ?? "none") / \(notes)"))
                }
            }
        }
        let window = try mount(Files())
        // the editor is the first field, so it has the keyboard
        try window.pressKey(.end, modifiers: Modifier.control.rawValue)
        try window.pressKey(.return, modifiers: 0)  // a new line, not the default button
        try window.typeText("second")
        try window.pressKey(.tab, modifiers: 0)
        // a field of one line shows the line break as a space
        XCTAssertEqual(try field(window).text, "none / first line second")
        let summary = try field(window)
        // the second row of the table: under the menu bar, the frame and the headings
        try window.clickAt(20, y: 20 + 2 + 18 + 16 + 8)
        XCTAssertEqual(try summary.text, "todo.txt / first line second")
    }

    func testTextEditorSelection() throws {
        let doc = EditedDoc()
        struct Editor: View {
            @Bindable var doc: EditedDoc
            var body: some View {
                TextEditor(text: $doc.text, selection: $doc.selection)
            }
        }
        let window = try mount(Editor(doc: doc))
        // the user moves the caret: the binding follows
        try window.pressKey(.end, modifiers: 0)
        let atEnd = try XCTUnwrap(doc.selection)
        XCTAssertTrue(atEnd.isInsertion)
        XCTAssertEqual(atEnd.offsets(in: doc.text).0, 11)
        try window.pressKey(.left, modifiers: Modifier.shift.rawValue | Modifier.control.rawValue)
        guard case .selection(let word) = try XCTUnwrap(doc.selection).indices else { return XCTFail() }
        XCTAssertEqual(String(doc.text[word]), "world")
        // typing over it changes the text, then the selection
        try window.typeText("there")
        XCTAssertEqual(doc.text, "hello there")
        XCTAssertEqual(doc.selection?.offsets(in: doc.text).0, 11)
        // the program puts the caret somewhere: the editor puts it there
        let text = doc.text
        doc.selection = TextSelection(insertionPoint: text.index(text.startIndex, offsetBy: 5))
        Runtime.shared.flush()
        try window.typeText(",")
        XCTAssertEqual(doc.text, "hello, there")
        // and replacing the text keeps the caret where the program says
        doc.text = "# " + doc.text
        doc.selection = TextSelection(insertionPoint: doc.text.endIndex)
        Runtime.shared.flush()
        try window.typeText("!")
        XCTAssertEqual(doc.text, "# hello, there!")
    }

    func testCanvasTextInItsOwnFont() throws {
        struct Words: View {
            let font: Font?
            var body: some View {
                Canvas { context, _ in
                    context.draw(Text("Heading").font(font), at: CGPoint(x: 4, y: 4), anchor: .topLeading)
                }
                .frame(width: 160, height: 40)
            }
        }
        let plain = try picture(try mount(Words(font: nil)))
        Runtime.shared.reset()
        let large = try picture(try mount(Words(font: .title)))
        XCTAssertNotEqual(plain, large, "the heading is drawn in its own typeface")
        XCTAssertEqual(Text("x").bold().font, Font.body.bold())
        XCTAssertEqual(Text("x").monospaced().font, .press(.pica))
    }

    func testTabsAndScrolling() throws {
        struct Pages: View {
            @State private var page = "two"
            var body: some View {
                VStack {
                    TabView(selection: $page) {
                        Text("First page").tabItem { Text("One") }.tag("one")
                        ScrollView {
                            ForEach(0..<20) { Text("Line \($0)") }
                        }
                        .frame(width: 160, height: 80)
                        .tabItem { Text("Two") }.tag("two")
                    }
                    TextField("", text: .constant(page))
                }
            }
        }
        let window = try mount(Pages())
        XCTAssertEqual(try field(window).text, "two")
        let text = try field(window)
        try window.clickAt(20, y: 28)  // the first tab
        XCTAssertEqual(try text.text, "one")
    }

    func testSliderAndProgress() throws {
        final class Seen { var value = 0.0 }
        let seen = Seen()
        struct Volume: View {
            let seen: Seen
            @State private var level = 0.5
            var body: some View {
                VStack {
                    Slider(value: $level, in: 0...10, step: 1)
                    ProgressView(value: level, total: 10)
                    ProgressView()
                }
                .onChange(of: level, initial: true) { seen.value = level }
            }
        }
        let window = try mount(Volume(seen: seen))
        XCTAssertEqual(seen.value, 0.5)
        try window.pressKey(.tab, modifiers: 0)
        try window.pressKey(.right, modifiers: 0)
        XCTAssertEqual(seen.value, 2, "half rounds to one step, and Right moves one more")
    }

    // MARK: Menus

    func testCommandsAndTheStandardMenus() throws {
        struct Program: Press.App {
            @State private var loud = false
            @State private var size = 2
            var body: some Scene {
                WindowGroup("Program") {
                    VStack {
                        TextField("", text: .constant("loud \(loud) size \(size)"))
                    }
                }
                .commands {
                    CommandMenu("Options") {
                        Toggle("Loud", isOn: $loud).keyboardShortcut("l")
                        Divider()
                        Picker("Size", selection: $size) {
                            ForEach(1..<4) { Text("Size \($0)") }
                        }
                        if loud {
                            Button("Quieter") { loud = false }
                        }
                    }
                    CommandGroup(before: .appTermination) {
                        Button("Close") {}.keyboardShortcut("w")
                    }
                }
            }
        }
        let host = AppHost(Program())
        Runtime.shared.launch(host)
        let window = try XCTUnwrap(host.windows.first).window
        let text = try field(window)
        XCTAssertEqual(try text.text, "loud false size 2")

        // File, Edit, Options: pull down File and go left, round to Options
        try window.pressKey(.menu, modifiers: 0)
        try window.pressKey(.left, modifiers: 0)
        try window.pressKey(.return, modifiers: 0)  // Loud, the first item
        XCTAssertEqual(try text.text, "loud true size 2")

        // the menu has another item now, so it was made afresh; choose it
        try window.pressKey(.menu, modifiers: 0)
        try window.pressKey(.left, modifiers: 0)
        try window.pressKey(.up, modifiers: 0)  // from the top round to the last: Quieter
        try window.pressKey(.return, modifiers: 0)
        XCTAssertEqual(try text.text, "loud false size 2")

        // the Edit menu works on the field with the keyboard
        try window.pressKey(.menu, modifiers: 0)
        try window.pressKey(.right, modifiers: 0)
        for _ in 0..<3 {
            try window.pressKey(.down, modifiers: 0)
        }
        try window.pressKey(.return, modifiers: 0)  // Select All
        try window.typeText("x")
        XCTAssertEqual(try text.text, "x", "everything was selected, so typing took its place")
    }

    func testDismissClosesTheWindow() throws {
        struct Closer: View {
            @Environment(\.dismiss) private var dismiss
            var body: some View { Button("Close") { dismiss() }.keyboardShortcut(.defaultAction) }
        }
        let host = AppHost("Test", Closer())
        Runtime.shared.launch(host)
        let window = try XCTUnwrap(host.windows.first).window
        try window.pressKey(.return, modifiers: 0)
        XCTAssertTrue(host.windows.isEmpty)
        XCTAssertThrowsError(try window.setTitle("gone"))
    }

    func testTasksRunOnTheMainActor() async throws {
        struct Loader: View {
            @State private var text = "loading"
            var body: some View {
                TextField("", text: .constant(text))
                    .task {
                        try? await Task.sleep(for: .milliseconds(5))
                        text = "loaded"
                    }
            }
        }
        let window = try mount(Loader())
        let text = try field(window)
        XCTAssertEqual(try text.text, "loading")
        try PressCore.App.start()
        try await Task.sleep(for: .milliseconds(100))
        // the task changed the state outside any handler, which asks press
        // for a turn of its loop; a program's own loop takes it at once
        _ = try PressCore.App.dispatch()
        XCTAssertEqual(try text.text, "loaded")
        try PressCore.App.stop()
    }

    func testCanvasDrawsAndHearsClicks() throws {
        final class Seen { var points: [CGPoint] = [] }
        let seen = Seen()
        struct Drawing: View {
            let seen: Seen
            @State private var marks: [CGPoint] = []
            var body: some View {
                Canvas { context, size in
                    context.stroke(Path(CGRect(origin: .zero, size: size)))
                    context.fill(Path(CGRect(x: 10, y: 10, width: 30, height: 20)), with: .color(.gray))
                    context.draw(Text("\(marks.count) marks"), at: CGPoint(x: size.width / 2, y: 50))
                    for mark in marks {
                        context.fill(Path(CGRect(x: mark.x - 2, y: mark.y - 2, width: 5, height: 5)))
                    }
                }
                .onTapGesture { point in
                    marks.append(point)
                    seen.points.append(point)
                }
                .frame(width: 160, height: 90)
            }
        }
        let window = try mount(Drawing(seen: seen))
        let blank = try picture(window)
        try window.clickAt(100, y: 20 + 70)
        XCTAssertEqual(seen.points, [CGPoint(x: 100, y: 70)])
        XCTAssertNotEqual(try picture(window), blank, "drawn again with the mark and the new count")
    }

    func testCanvasDragsDoubleClicksAndBitmaps() throws {
        final class Heard {
            var changes: [CGPoint] = []
            var ended: DragGesture.Value?
            var taps: [String] = []
        }
        let heard = Heard()
        struct Board: View {
            let heard: Heard
            var body: some View {
                Canvas { context, _ in
                    // a black bar eight pixels long, a byte to each row
                    context.draw(Image(bitmap: [255, 255], width: 8, height: 2), at: CGPoint(x: 4, y: 4))
                }
                .gesture(
                    DragGesture(minimumDistance: 4)
                        .onChanged { heard.changes.append($0.location) }
                        .onEnded { heard.ended = $0 }
                )
                .onTapGesture { _ in heard.taps.append("one") }
                .onTapGesture(count: 2) { _ in heard.taps.append("two") }
                .frame(width: 120, height: 60)
            }
        }
        let window = try mount(Board(heard: heard))
        try window.dragAt(10, y0: 20 + 10, x1: 50, y1: 20 + 30)
        XCTAssertEqual(heard.changes.last, CGPoint(x: 50, y: 30))
        XCTAssertGreaterThanOrEqual(heard.changes.count, 3)
        XCTAssertEqual(heard.ended?.startLocation, CGPoint(x: 10, y: 10))
        XCTAssertEqual(heard.ended?.translation, CGSize(width: 40, height: 20))
        XCTAssertTrue(heard.taps.isEmpty, "a drag is not a tap")
        // a click is a tap, and a second straight after it a double one
        try window.clickAt(30, y: 20 + 30)
        try window.clickAt(30, y: 20 + 30)
        XCTAssertEqual(heard.taps, ["one", "two"])
        // the bitmap is drawn: a blank canvas looks different
        struct Blank: View {
            var body: some View {
                Canvas { _, _ in }.frame(width: 120, height: 60)
            }
        }
        let drawn = try picture(window)
        Runtime.shared.reset()
        XCTAssertNotEqual(try picture(try mount(Blank())), drawn)
    }

    func testLifecycle() throws {
        final class Seen { var log: [String] = [] }
        let seen = Seen()
        struct Inner: View {
            let seen: Seen
            var body: some View {
                Text("inner").onAppear { seen.log.append("appear") }.onDisappear { seen.log.append("disappear") }
            }
        }
        struct Outer: View {
            let seen: Seen
            @State private var shown = true
            var body: some View {
                VStack {
                    if shown { Inner(seen: seen) }
                    Button("Toggle") { shown.toggle() }.keyboardShortcut(.defaultAction)
                }
            }
        }
        let window = try mount(Outer(seen: seen))
        XCTAssertEqual(seen.log, ["appear"])
        try window.pressKey(.return, modifiers: 0)
        XCTAssertEqual(seen.log, ["appear", "disappear"])
        try window.pressKey(.return, modifiers: 0)
        XCTAssertEqual(seen.log, ["appear", "disappear", "appear"])
    }
}

@Observable final class FullScreenModel {
    var full = false
}

/// A view whose window fills the screen as its model says.
struct FullScreenShown: View {
    @Bindable var model: FullScreenModel
    var body: some View {
        Text("A film").fullScreen(isOn: $model.full)
    }
}
