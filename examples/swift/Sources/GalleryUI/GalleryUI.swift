// SPDX-License-Identifier: GPL-3.0-or-later
// A tour of Press: controls, a list kept in an observable object, a
// drawing, and a clock that ticks in an asynchronous task.
//
//     swift run --package-path examples/swift GalleryUI
import Foundation
import Press
import Observation

@main
struct GalleryUI: App {
    @State private var library = Library()

    var body: some Scene {
        WindowGroup("Gallery") {
            TabView {
                ControlsPage().tabItem { Text("Controls") }
                LibraryPage().tabItem { Text("List") }
                DrawingPage().tabItem { Text("Drawing") }
            }
            .padding(12)
            .environment(library)
        }
        .commands {
            CommandMenu("Library") {
                Button("Add a Book") { library.add() }
                    .keyboardShortcut("n")
                Button("Remove the Last") { library.books.removeLast() }
                    .disabled(library.books.isEmpty)
            }
        }
        Window("About", id: "about") {
            Text("Press: press in the manner of SwiftUI.").padding()
        }
    }
}

// MARK: - Controls

struct ControlsPage: View {
    @State private var volume = 4.0
    @State private var paper = "A4"
    @State private var ink = "Black"
    @State private var password = ""
    @State private var seconds = 0
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Form {
            LabeledContent("Volume") {
                Slider(value: $volume, in: 0...10, step: 1)
                Text("\(Int(volume))")
            }
            ProgressView(value: volume, total: 10)
            LabeledContent("Paper") {
                Picker("Paper", selection: $paper) {
                    ForEach(["A4", "A5", "Letter"], id: \.self) { Text($0) }
                }
            }
            Picker("Ink", selection: $ink) {
                Text("Black").tag("Black")
                Text("Sepia").tag("Sepia")
            }
            .pickerStyle(.radioGroup)
            SecureField("Password", text: $password)
                .frame(width: 180)
            Text("\(paper), \(ink.lowercased()) ink, a password of \(password.count)")
                .font(.caption)
            HStack {
                Text("Open for \(seconds) s").monospaced()
                Spacer()
                Button("About") { openWindow(id: "about") }
            }
        }
        .padding(8)
        // The clock: a task that sleeps and counts for as long as the page lives.
        .task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(1))
                seconds += 1
            }
        }
    }
}

// MARK: - A list

struct Book: Identifiable {
    let id: Int
    var title: String
}

@Observable
final class Library {
    var books = [Book(id: 1, title: "Kidnapped"), Book(id: 2, title: "Middlemarch"), Book(id: 3, title: "Persuasion")]
    private var next = 4

    func add() {
        books.append(Book(id: next, title: "Untitled \(next)"))
        next += 1
    }
}

struct LibraryPage: View {
    @Environment(Library.self) private var library
    @State private var chosen: Int?

    var body: some View {
        @Bindable var library = library
        VStack(alignment: .leading) {
            List(library.books, selection: $chosen) { book in
                Label(book.title, systemImage: "doc")
            }
            .frame(width: 240)
            .contextMenu {
                Button("Add a Book") { library.add() }
            }
            if let index = library.books.firstIndex(where: { $0.id == chosen }) {
                TextField("Title", text: $library.books[index].title)
            } else {
                Text("Choose a book to rename it.").font(.callout)
            }
        }
        .padding(8)
    }
}

// MARK: - Drawing

struct DrawingPage: View {
    @State private var marks: [CGPoint] = []

    var body: some View {
        VStack(alignment: .leading) {
            Canvas { context, size in
                context.stroke(Path(CGRect(origin: .zero, size: size)))
                var wave = Path()
                wave.addLines(
                    stride(from: 0, to: size.width, by: 4).map { x in
                        CGPoint(x: x, y: size.height / 2 - sin(x / 18) * size.height / 3)
                    })
                context.stroke(wave)
                context.stroke(
                    Path { $0.addLines([CGPoint(x: 0, y: size.height / 2), CGPoint(x: size.width, y: size.height / 2)]) },
                    style: StrokeStyle(dash: [1]))
                for mark in marks {
                    context.fill(Path(CGRect(x: mark.x - 3, y: mark.y - 3, width: 7, height: 7)))
                }
                context.draw(Text("Click to leave a mark"), at: CGPoint(x: size.width / 2, y: 4), anchor: .top)
            }
            .onTapGesture { marks.append($0) }
            .accessibilityLabel("A sine wave, and the marks left by clicking")
            .frame(width: 280, height: 140)
            HStack {
                Text("\(marks.count) marks")
                Spacer()
                Button("Clear") { marks = [] }.disabled(marks.isEmpty)
            }
        }
        .padding(8)
    }
}
