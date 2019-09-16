// swift-tools-version:5.1

import PackageDescription

let package = Package(
    name: "Ockam",
    platforms: [
        .macOS(.v10_15),
    ],
    products: [
        .library(
            name: "Ockam",
            targets: ["Ockam"]),
    ],
    dependencies: [
    ],
    targets: [
        .target(
            name: "Ockam",
            dependencies: []),
        .testTarget(
            name: "OckamTests",
            dependencies: ["Ockam"]),
    ]
)
