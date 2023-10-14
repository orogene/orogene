// swift-tools-version: 5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "alabaster-swift",
    platforms: [
       .macOS(.v11)
    ],
    products: [
        .library(name: "alabaster-swift", type: .static, targets: ["alabaster-swift"]),
    ],
    dependencies: [
    ],
    targets: [
        .target(
            name: "alabaster-swift",
            dependencies: []
        )
    ]
)
