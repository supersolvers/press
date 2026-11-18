// swift-tools-version:5.9
// SPDX-License-Identifier: GPL-3.0-or-later
// The examples in Swift, built against the package in this checkout.
import PackageDescription

let package = Package(
    name: "PressExamples",
    dependencies: [.package(name: "Press", path: "../..")],
    targets: [
        .executableTarget(name: "Hello", dependencies: [.product(name: "PressCore", package: "Press")]),
        .executableTarget(name: "HelloUI", dependencies: [.product(name: "Press", package: "Press")]),
        .executableTarget(name: "GalleryUI", dependencies: [.product(name: "Press", package: "Press")]),
        .executableTarget(name: "ToolsUI", dependencies: [.product(name: "Press", package: "Press")]),
    ]
)
