// SPDX-License-Identifier: GPL-3.0-or-later
// The same dialog as examples/c/hello.c, through the generated Swift binding.
//
//     swift run --package-path examples/swift Hello [--snapshot FILE.png]
import PressCore

try App.initialize()
let version = try App.abiVersion()
precondition(version == pressABIVersion)

let window = try Window(title: "Hello from Swift", width: 300, height: 198)
let column = try Stack(.vertical)
try column.setPadding(16)
try column.setSpacing(12)

let name = try TextField("")
try name.setPlaceholder("Type a name")
try name.setMinWidth(268)
let check = try Checkbox("Say it loudly")
let status = try Label("Press Return to say hello.")
let ok = try Button("OK")
try ok.setIsDefault(true)
try ok.setEnabled(false)
let cancel = try Button("Cancel")
try cancel.setIsCancel(true)

let buttons = try Stack(.horizontal)
try buttons.setSpacing(12)
try buttons.setAlign(.center)
try buttons.add(cancel)
try buttons.add(ok)

for widget in [try Label("What is your name?"), name, check, status, buttons] {
    try column.add(widget)
}
try window.setContent(column)

let greetings = ["Hello", "Good day"]
var greeting = 0
var shout = false

/// A handler cannot throw, so what goes wrong inside one is reported here.
func attempt(_ body: () throws -> Void) {
    do {
        try body()
    } catch {
        print("error:", error)
    }
}

/// Add an item to a menu, with its shortcut and what it does.
@discardableResult
func item(_ menu: Menu, _ label: String, _ key: String, _ action: @escaping () throws -> Void) throws -> MenuItem {
    let entry = try MenuItem(label)
    try entry.setShortcut(key)
    try entry.onActivated { attempt(action) }
    try menu.add(entry)
    return entry
}

let bar = try MenuBar()
let file = try Menu("File")
let edit = try Menu("Edit")
let options = try Menu("Options")
try item(file, "Quit", "Q") { try App.quit() }
try item(edit, "Cut", "X") { try name.cut() }
try item(edit, "Copy", "C") { try name.copy() }
try item(edit, "Paste", "V") { try name.paste() }
try edit.addSeparator()
try item(edit, "Select All", "A") { try name.selectAll() }
let loud = try item(options, "Say It Loudly", "L") { try setShout(!shout) }

/// The checkbox and the Options menu item show the same setting.
func setShout(_ value: Bool) throws {
    shout = value
    try check.setChecked(value)
    try loud.setChecked(value)
    print("loud:", value ? "on" : "off")
}

// a submenu: an item that opens another menu beside it
let greetingMenu = try Menu("")
var greetingItems: [MenuItem] = []
for (index, word) in greetings.enumerated() {
    greetingItems.append(
        try item(greetingMenu, word, "") {
            greeting = index
            for (i, entry) in greetingItems.enumerated() {
                try entry.setChecked(i == index)
            }
        })
}
try greetingItems[0].setChecked(true)
let greetingItem = try MenuItem("Greeting")
try greetingItem.setSubmenu(greetingMenu)
try options.add(greetingItem)
for menu in [file, edit, options] {
    try bar.add(menu)
}
try window.setMenuBar(bar)

try ok.onClicked {
    attempt {
        let who = try name.text
        try status.setText("\(greetings[greeting]), \(who)\(shout ? "!!!" : ".")")
        print("ok:", who)
    }
}
try cancel.onClicked { attempt { try App.quit() } }
try name.onChanged { text in attempt { try ok.setEnabled(!text.isEmpty) } }
try check.onToggled { value in attempt { try setShout(value) } }

if CommandLine.arguments.count == 3, CommandLine.arguments[1] == "--snapshot" {
    try window.typeText("Zoë")  // the first text field has the keyboard
    try window.pressKey(.tab, modifiers: 0)
    try window.typeText(" ")  // Space presses the focused checkbox
    try window.pressKey(.tab, modifiers: Modifier.shift.rawValue)  // back to the field, text selected
    try window.pressKey(.end, modifiers: 0)
    try window.typeText(" Ångström")
    try window.pressKey(.return, modifiers: 0)  // the default button
    try window.pressKey(.menu, modifiers: 0)  // pull down File...
    try window.pressKey(.left, modifiers: 0)  // ...and wrap round to Options
    try window.snapshot(CommandLine.arguments[2], scale: 2)
    do {
        try Label(handle: ok.handle).setText("x")  // a Button is not a Label
    } catch {
        print("wrong type:", error)
    }
} else {
    try window.show()
    try App.run()
}
try App.shutdown()
