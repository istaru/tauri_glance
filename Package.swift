// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SwiftGlance",
    platforms: [
        .macOS(.v12)
    ],
    targets: [
        .executableTarget(
            name: "SwiftGlance",
            path: "Sources/SwiftGlance",
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("ServiceManagement")
            ]
        )
    ]
)
