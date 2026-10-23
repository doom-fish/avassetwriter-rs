// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "AVAssetWriterBridge",
    platforms: [
        .macOS(.v12)
    ],
    products: [
        .library(
            name: "AVAssetWriterBridge",
            type: .static,
            targets: ["AVAssetWriterBridge"])
    ],
    targets: [
        .target(
            name: "AVAssetWriterObjCBridge",
            path: "Sources/AVAssetWriterObjCBridge",
            publicHeadersPath: "include"),
        .target(
            name: "AVAssetWriterBridge",
            dependencies: ["AVAssetWriterObjCBridge"],
            path: "Sources/AVAssetWriterBridge")
    ]
)
