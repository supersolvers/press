// swift-tools-version:5.9
// SPDX-License-Identifier: GPL-3.0-or-later
// PRESS Renders Every Style in Swift.
//
// Two libraries. Press is the framework: a program says what its windows
// hold, as values, in the manner of SwiftUI, and Press keeps the widgets in
// step with them. PressCore is the engine's interface as it is, one call for
// one call, generated from idl/press.toml over the C interface in
// include/press.h; Press is written on top of it.
import Foundation
import PackageDescription

// Where libpress.so is. PRESS_LIB_DIR says so outright; otherwise, of the
// builds in this checkout, the one built last is the one meant. With neither,
// the linker looks where it always does, which finds an installed library.
let checkout = URL(fileURLWithPath: #filePath).deletingLastPathComponent().standardizedFileURL
let built = ["release", "debug"]
    .map { checkout.appendingPathComponent("target/\($0)").path }
    .compactMap { dir -> (String, Date)? in
        let attributes = try? FileManager.default.attributesOfItem(atPath: dir + "/libpress.so")
        return (attributes?[.modificationDate] as? Date).map { (dir, $0) }
    }
    .sorted { $0.1 > $1.1 }
    .map { $0.0 }
let libraryDir = ProcessInfo.processInfo.environment["PRESS_LIB_DIR"] ?? built.first
let linking: [LinkerSetting] = libraryDir.map { [.unsafeFlags(["-L", $0, "-Xlinker", "-rpath", "-Xlinker", $0])] } ?? []

let package = Package(
    name: "Press",
    products: [
        .library(name: "Press", targets: ["Press"]),
        .library(name: "PressCore", targets: ["PressCore"]),
    ],
    targets: [
        .systemLibrary(name: "CPress", path: "bindings/swift/Sources/CPress"),
        .target(name: "PressCore", dependencies: ["CPress"], path: "bindings/swift/Sources/PressCore", linkerSettings: linking),
        .target(name: "Press", dependencies: ["PressCore"], path: "bindings/swift/Sources/Press"),
        .testTarget(name: "PressCoreTests", dependencies: ["PressCore"], path: "bindings/swift/Tests/PressCoreTests"),
        .testTarget(name: "PressTests", dependencies: ["Press"], path: "bindings/swift/Tests/PressTests"),
    ]
)
