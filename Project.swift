import ProjectDescription

// Ook Reader — Tuist manifest. See RESEARCH.md (Phase 2).
// One iOS app target serves both iPhone and iPad (iPadOS). macOS is deferred
// to Phase 5 via Mac Catalyst, so it is intentionally not a destination yet.
let project = Project(
    name: "OokReader",
    targets: [
        .target(
            name: "OokReader",
            destinations: [.iPhone, .iPad],
            product: .app,
            bundleId: "com.dvportenko.ookreader",
            deploymentTargets: .iOS("17.0"),
            infoPlist: .extendingDefault(with: [
                "UILaunchScreen": ["UIColorName": ""],
            ]),
            sources: ["Sources/**"],
            dependencies: []
        ),
        .target(
            name: "OokReaderTests",
            destinations: [.iPhone, .iPad],
            product: .unitTests,
            bundleId: "com.dvportenko.ookreader.tests",
            deploymentTargets: .iOS("17.0"),
            sources: ["Tests/**"],
            dependencies: [.target(name: "OokReader")]
        ),
    ]
)
