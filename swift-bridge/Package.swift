// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "AVAssetWriterBridge",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "AVAssetWriterBridge",
            type: .static,
            targets: ["AVAssetWriterBridge"])
    ],
    targets: [
        .target(
            name: "AVAssetWriterBridge",
            path: "Sources/AVAssetWriterBridge",
            publicHeadersPath: "include")
    ]
)
