// swift-tools-version: 6.2
import PackageDescription

// PrexpCore — the Swift half of fdtop's `core` layer. A native reimplementation
// of the process/file-descriptor data layer (models + `ProcessSource` + formatters),
// parallel to the Rust `prexp-core` + `prexp-ffi` crates. No FFI to Rust: the native
// source talks straight to macOS libproc/Mach via the `Darwin` module (no C shim
// needed — every API `prexp-ffi` wraps is reachable directly from Swift).
let package = Package(
    name: "PrexpCore",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "PrexpCore", targets: ["PrexpCore"]),
        // Smoke tool: dumps `snapshot_all` as JSON/TSV, to eyeball parity against
        // `cargo run -p prexp -- --output json`.
        .executable(name: "prexp-smoke", targets: ["prexp-smoke"]),
    ],
    targets: [
        .target(name: "PrexpCore"),
        .executableTarget(name: "prexp-smoke", dependencies: ["PrexpCore"]),
        .testTarget(name: "PrexpCoreTests", dependencies: ["PrexpCore"]),
    ]
)
