// SPDX-License-Identifier: GPL-3.0-or-later
// A colour picture on a canvas: drawn, and drawn as its colours.

import Foundation
import PressCore
import XCTest

@testable import Press

@MainActor
final class PhotoTests: XCTestCase {
    override func setUp() async throws {
        setenv("PRESS_BACKEND", "headless", 1)
        setenv("PRESS_QUIET", "1", 1)
        try PressCore.App.initialize()
    }

    override func tearDown() async throws {
        Runtime.shared.reset()
        try PressCore.App.shutdown()
    }

    /// The window's picture with a canvas showing `photo`, if any.
    private func shown(_ photo: Photo?) throws -> Data {
        struct Frame: View {
            let photo: Photo?
            var body: some View {
                Canvas { context, _ in
                    if let photo { context.draw(photo, at: CGPoint(x: 10, y: 10)) }
                }
                .frame(width: 60, height: 40)
            }
        }
        let host = AppHost("Test", Frame(photo: photo))
        Runtime.shared.launch(host)
        let window = try XCTUnwrap(host.windows.first).window
        let path = NSTemporaryDirectory() + "pressui-photo-\(getpid())-\(UUID().uuidString).png"
        defer { try? FileManager.default.removeItem(atPath: path) }
        try window.snapshot(path, scale: 1)
        Runtime.shared.reset()
        return try XCTUnwrap(FileManager.default.contents(atPath: path))
    }

    func testAPhotoIsDrawnInItsColours() throws {
        func solid(_ r: UInt8, _ g: UInt8, _ b: UInt8) -> Photo {
            Photo(rgb: [UInt8](repeating: 0, count: 0) + (0..<(20 * 10)).flatMap { _ in [r, g, b] }, width: 20, height: 10)
        }
        let blank = try shown(nil)
        let red = try shown(solid(255, 0, 0))
        let blue = try shown(solid(0, 0, 255))
        XCTAssertNotEqual(red, blank)
        XCTAssertNotEqual(red, blue, "two colours are two pictures")
        // the same again is the same picture
        XCTAssertEqual(try shown(solid(255, 0, 0)), red)
    }
}
