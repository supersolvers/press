// swift-tools-version:5.9
// SPDX-License-Identifier: GPL-3.0-or-later
// PRESS Renders Every Style in Swift.
//
// Two libraries. Press is the framework: a program says what its windows
// hold, as values, in the manner of SwiftUI, and Press keeps the widgets in
// step with them. PressCore is the engine's interface as it is, one call for
// one call, generated from idl/press.toml over the C interface in
// include/press.h; Press is written on top of it.
import PackageDescription

let package = Package(
    name: "Press",
    products: [
        .library(name: "Press", targets: ["Press"]),
        .library(name: "PressCore", targets: ["PressCore"]),
    ],
    targets: [
        .systemLibrary(
            name: "CPress",
            path: "bindings/swift/Sources/CPress",
            pkgConfig: "press",
            providers: [.apt(["libpress-dev"])]
        ),
        .target(name: "PressCore", dependencies: ["CPress"], path: "bindings/swift/Sources/PressCore"),
        .target(name: "Press", dependencies: ["PressCore"], path: "bindings/swift/Sources/Press"),
        .testTarget(name: "PressCoreTests", dependencies: ["PressCore"], path: "bindings/swift/Tests/PressCoreTests"),
        .testTarget(name: "PressTests", dependencies: ["Press"], path: "bindings/swift/Tests/PressTests"),
    ]
)
