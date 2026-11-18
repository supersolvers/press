// SPDX-License-Identifier: GPL-3.0-or-later
// The binding, exercised with no display: PRESS_BACKEND=headless is set
// before the library starts, and input comes from the test hooks.
import Foundation
import XCTest

@testable import PressCore

final class PressCoreTests: XCTestCase {
    override func setUpWithError() throws {
        setenv("PRESS_BACKEND", "headless", 1)
        try App.initialize()
    }

    override func tearDownWithError() throws {
        try App.shutdown()
    }

    func testVersionAgrees() throws {
        XCTAssertEqual(try App.abiVersion(), pressABIVersion)
    }

    func testStringsGoBothWays() throws {
        let field = try TextField("Zoë")
        XCTAssertEqual(try field.text, "Zoë")
        try field.setText("")
        XCTAssertEqual(try field.text, "")
        let long = String(repeating: "Ångström 한국어 ", count: 200)
        try field.setText(long)
        XCTAssertEqual(try field.text, long)
    }

    func testErrorsAreThrown() throws {
        let button = try Button("OK")
        // a Button is not a Label
        XCTAssertThrowsError(try Label(handle: button.handle).setText("x")) { error in
            XCTAssertEqual(error as? PressError, PressError(.wrongType))
        }
        try button.destroy()
        XCTAssertThrowsError(try button.setLabel("x")) { error in
            XCTAssertEqual((error as? PressError)?.status, .invalidHandle)
        }
        XCTAssertEqual(PressError(.wrongType).description, "wrongType: the handle refers to an object of another class")
        XCTAssertNil(PressError(code: 999).status)
    }

    func testHandlersAreCalledAndLetGo() throws {
        final class Witness {}
        var witness: Witness? = Witness()
        weak var watched = witness

        let window = try Window(title: "Test", width: 200, height: 100)
        let field = try TextField("")
        try window.setContent(field)
        var seen: [String] = []
        try field.onChanged { [witness] text in
            _ = witness
            seen.append(text)
        }
        witness = nil
        try window.typeText("hé")
        XCTAssertEqual(seen.last, "hé")
        XCTAssertNotNil(watched, "the library keeps the handler alive")

        // a new handler takes the place of the old, which is let go
        var count = 0
        try field.onChanged { _ in count += 1 }
        XCTAssertNil(watched)
        try window.typeText("!")
        XCTAssertEqual(count, 1)

        try field.onChanged(nil)
        try window.typeText("?")
        XCTAssertEqual(count, 1)
        XCTAssertEqual(try field.text, "hé!?")
    }

    func testHandlerMayReplaceItself() throws {
        let window = try Window(title: "Test", width: 200, height: 100)
        let field = try TextField("")
        try window.setContent(field)
        var calls = 0
        try field.onChanged { _ in
            calls += 1
            try? field.onChanged(nil)
            calls += 1  // still running, though disconnected
        }
        try window.typeText("ab")
        XCTAssertEqual(calls, 2)
    }

    func testDestroyingAnObjectLetsItsHandlerGo() throws {
        final class Witness {}
        var witness: Witness? = Witness()
        weak var watched = witness
        let button = try Button("OK")
        try button.onClicked { [witness] in _ = witness }
        witness = nil
        XCTAssertNotNil(watched)
        try button.destroy()
        XCTAssertNil(watched)
    }

    func testKeysAndFlags() throws {
        let window = try Window(title: "Test", width: 300, height: 120)
        let column = try Stack(.vertical)
        let field = try TextField("")
        let check = try Checkbox("Loud")
        try column.add(field)
        try column.add(check)
        try window.setContent(column)
        var toggled: [Bool] = []
        try check.onToggled { toggled.append($0) }
        try window.pressKey(.tab, modifiers: 0)
        try window.typeText(" ")  // Space presses the focused checkbox
        XCTAssertEqual(toggled, [true])
        XCTAssertTrue(try check.checked)
        let both: Modifier = [.shift, .control]
        XCTAssertEqual(both.rawValue, 3)
        try window.pressKey(.tab, modifiers: Modifier.shift.rawValue)
        try window.typeText("x")
        XCTAssertEqual(try field.text, "x")
    }

    func testPostedWorkRunsOnTheUIThread() throws {
        try App.start()
        defer { try? App.stop() }
        let main = pthread_self()
        let ran = expectation(description: "posted work ran")
        let worker = Thread {
            try? App.post {
                XCTAssertEqual(pthread_self(), main)
                ran.fulfill()
            }
        }
        worker.start()
        let deadline = Date().addingTimeInterval(5)
        while !worker.isFinished && Date() < deadline {
            usleep(1000)
        }
        _ = try App.dispatch()
        wait(for: [ran], timeout: 0)
    }

    func testSnapshot() throws {
        let window = try Window(title: "Test", width: 120, height: 60)
        try window.setContent(try Label("Hello"))
        let path = NSTemporaryDirectory() + "press-swift-\(getpid()).png"
        defer { try? FileManager.default.removeItem(atPath: path) }
        try window.snapshot(path, scale: 1)
        let data = try XCTUnwrap(FileManager.default.contents(atPath: path))
        XCTAssertEqual(Array(data.prefix(4)), [0x89, 0x50, 0x4E, 0x47])
    }
}
