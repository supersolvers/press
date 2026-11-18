// SPDX-License-Identifier: GPL-3.0-or-later
// The same dialog as examples/c/hello.c, written with Press: what the
// window holds is described, and press is kept in step with it.
//
//     swift run --package-path examples/swift HelloUI
import Press

@main
struct HelloUI: App {
    @State private var loud = false
    @State private var greeting = 0

    var body: some Scene {
        WindowGroup("Hello from Press") {
            Dialog(loud: $loud, greeting: greeting)
        }
        .commands {
            CommandMenu("Options") {
                Toggle("Say It Loudly", isOn: $loud)
                    .keyboardShortcut("l")
                Picker("Greeting", selection: $greeting) {
                    ForEach(greetings.indices, id: \.self) { index in
                        Text(greetings[index])
                    }
                }
            }
        }
    }
}

let greetings = ["Hello", "Good day"]

struct Dialog: View {
    @Binding var loud: Bool
    let greeting: Int
    @State private var name = ""
    @State private var status = "Press Return to say hello."
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("What is your name?")
            TextField("Type a name", text: $name)
                .frame(minWidth: 268)
            Toggle("Say it loudly", isOn: $loud)
            Text(status)
            HStack(spacing: 12) {
                Button("Cancel", role: .cancel) { dismiss() }
                Button("OK") {
                    status = "\(greetings[greeting]), \(name)\(loud ? "!!!" : ".")"
                    print("ok:", name)
                }
                .keyboardShortcut(.defaultAction)
                .disabled(name.isEmpty)
            }
        }
        .padding(16)
        .onChange(of: loud) { print("loud:", loud ? "on" : "off") }
    }
}
