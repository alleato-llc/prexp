// swift-tools-version: 6.2
import PackageDescription

// prexp TUI — the Swift terminal front-end, on the sibling `tint` immediate-mode
// kit. Parallel to the Rust `prexp` binary. lib/bin split: `PrexpTUI` holds the
// model + render logic (testable against a canned ProcessSource); `prexp` is the
// thin executable that wires tint's run loop.
let package = Package(
    name: "PrexpTUI",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "prexp", targets: ["prexp"]),
        .library(name: "PrexpTUI", targets: ["PrexpTUI"]),
    ],
    dependencies: [
        .package(path: "../Core"),
        .package(path: "../../../tint"),
    ],
    targets: [
        .target(name: "PrexpTUI", dependencies: [
            .product(name: "PrexpCore", package: "Core"),
            .product(name: "Tint", package: "tint"),
        ]),
        .executableTarget(name: "prexp", dependencies: [
            "PrexpTUI",
            .product(name: "PrexpCore", package: "Core"),
            .product(name: "Tint", package: "tint"),
        ]),
        .testTarget(name: "PrexpTUITests", dependencies: ["PrexpTUI",
            .product(name: "PrexpCore", package: "Core"),
            .product(name: "Tint", package: "tint"),
        ]),
    ]
)
