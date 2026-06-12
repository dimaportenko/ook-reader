# Ook Reader — Research & Plan

> Deep research on building a multi-platform (macOS / iOS / iPadOS) EPUB reader in Swift,
> using NeoVim (LazyVim) as the primary IDE instead of Xcode.
> Date: 2026-06-12. 22 sources, 23 verified claims, 2 refuted.

## TL;DR

- **NeoVim for Swift/Apple dev is viable in 2026** — ~90% of day-to-day work stays in the editor.
- Core stack: **sourcekit-lsp** + **xcode-build-server** + **xcodebuild.nvim** + **nvim-dap/codelldb**.
- **Keep Xcode installed** for signing, profiling (Instruments), asset catalogs, and some debugging.
- **Tuist** is the best project generator for a CLI-driven multi-target workflow.
- **Readium Swift Toolkit** is the EPUB library — but its Navigator is **UIKit-only** and the
  package declares **iOS only (no macOS)**. So: ship **iOS + iPadOS first**, do macOS later via **Mac Catalyst**.

---

## 1. Is NeoVim viable for Swift/Apple dev? Yes (~90% in-editor)

| Piece | Role | Install |
|---|---|---|
| **sourcekit-lsp** | LSP: completion, diagnostics, go-to-def. Apple's, bundled with Xcode. Officially documented by swift.org for NeoVim. | comes with Xcode/toolchain |
| **xcode-build-server** | Bridges sourcekit-lsp → `.xcodeproj`/`.xcworkspace` via Build Server Protocol. sourcekit-lsp natively understands only SwiftPM / `compile_commands.json`, **not** Xcode projects. | `brew install xcode-build-server` |
| **xcodebuild.nvim** | Build/run/test on simulators & devices; wraps official `xcodebuild` + `xcrun simctl`; test explorer, coverage, quickfix. | nvim plugin |
| **nvim-dap + lldb-dap** | Debugging. NOTE: research said codelldb, but xcodebuild.nvim now uses Apple's bundled **`lldb-dap`** for **Xcode 16+** (we run 26.5) — codelldb no longer needed. | nvim plugin (adapter ships with Xcode) |

Sources: [swift.org NeoVim guide](https://www.swift.org/documentation/articles/zero-to-swift-nvim.html),
[the complete guide (wojciechkulik.pl)](https://wojciechkulik.pl/ios/the-complete-guide-to-ios-macos-development-in-neovim),
[xcodebuild.nvim](https://github.com/wojciech-kulik/xcodebuild.nvim),
[xcode-build-server](https://github.com/SolaWing/xcode-build-server).

**Two corrections to common assumptions** (both adversarially verified):

- **SweetPad is NOT a NeoVim plugin** — it's a *VSCode* extension. No role in a NeoVim workflow.
- You do **not** have to abandon hand-managed `.xcodeproj` for XcodeGen/Tuist. Generators are
  *convenient* for a CLI workflow, not *required* (claim refuted 0-3).

## 2. What still forces you into Xcode

Keep Xcode installed and open it occasionally for:

- **Code signing / provisioning** (the single most-cited Xcode dependency)
- Advanced debugging (sanitizers, memory graph), view-hierarchy / UI debugging
- Performance profiling (Instruments)
- **SwiftUI previews** (xcodebuild.nvim now has partial support)
- StoreKit 2 debugging
- Editing **asset catalogs** (`.xcassets`) — just easier there

Known pain point: sourcekit-lsp **background indexing can spike CPU**
([issue #2346](https://github.com/swiftlang/sourcekit-lsp/issues/2346), still open).
Be ready to limit/disable background indexing on a multi-target project.

## 3. Project structure: Tuist is the best CLI-driven fit

For a shared macOS+iOS+iPadOS codebase, **Tuist** is the strongest match: Swift-DSL manifests
(`Project.swift` / `Workspace.swift` — Xcode autocompletion on the config itself), generates standard
Xcode projects/workspaces, auto-creates a workspace for multi-target setups, builds/tests via `xcodebuild`.
XcodeGen (YAML) is the lighter alternative. Plain SPM works but is awkward for app targets with
resources/entitlements. Source: [Tuist docs](https://docs.tuist.dev/en/guides/features/projects).

## 4. The EPUB reader: Readium — with a macOS caveat

**[Readium Swift Toolkit](https://github.com/readium/swift-toolkit)** (BSD-3-Clause, v3.9.0 May 2026,
SPM-installable) is the clear choice: EPUB reflowable + fixed-layout, PDF, audiobooks, comics.
FolioReaderKit (named in the original question) appears largely unmaintained by comparison.

**Two real constraints for the multi-platform goal:**

- The rendering Navigator (`EPUBNavigatorViewController`) is **UIKit-only** — no native SwiftUI navigator.
  Bridge it into SwiftUI with `UIViewControllerRepresentable`
  ([Readium SwiftUI guide](https://github.com/readium/swift-toolkit/blob/develop/docs/Guides/Navigator/SwiftUI.md)).
- Readium's develop-branch `Package.swift` declares **`.iOS` only — no macOS platform**. A native macOS
  target is not a drop-in. Realistic options: ship **iOS + iPadOS first** (Readium works natively), then
  use **Mac Catalyst** for desktop (runs the iPad/UIKit app on macOS, carries the UIKit Navigator along)
  rather than a separate AppKit app. A native AppKit reader would need a different rendering path
  (e.g. WKWebView-based).

---

## Step-by-step plan

### Phase 0 — Toolchain (before any app code)
1. Install Xcode + Command Line Tools (need the SDKs/simulators regardless).
2. From the LazyVim base, add: `sourcekit-lsp` via nvim-lspconfig, `brew install xcode-build-server`,
   `xcodebuild.nvim`, `nvim-dap` (debugging uses Xcode's bundled `lldb-dap` on Xcode 16+; no codelldb). Follow the
   [swift.org guide](https://www.swift.org/documentation/articles/zero-to-swift-nvim.html) for LSP and the
   [xcodebuild.nvim wiki](https://github.com/wojciech-kulik/xcodebuild.nvim/wiki/Neovim-Configuration) for the rest.
3. **Validate with a throwaway "Hello World" iOS app** — build, run on simulator, set a breakpoint, hit it.
   Don't start the real project until this round-trip works.

### Phase 1 — Learn Swift (parallel with Phase 0, ~1–2 weeks)
- Work through [The Swift Programming Language](https://docs.swift.org/swift-book/) +
  [Apple SwiftUI tutorials](https://developer.apple.com/tutorials/swiftui)
  (or [Hacking with Swift 100 Days](https://www.hackingwithswift.com/100/swiftui)).
- Keep the [API Design Guidelines](https://www.swift.org/documentation/api-design-guidelines/) open — that's
  where idiomatic naming/patterns come from.
- Default to **SwiftUI**; drop to UIKit only where Readium forces it.

### Phase 2 — Project scaffold (iOS + iPadOS first)
- `brew install tuist`; define `Project.swift` with an iOS app target (iPad comes free via device family).
- Generate the project, run `xcode-build-server config` against the generated workspace+scheme to wire up the LSP.
- Gitignore the generated `.xcodeproj`. Confirm the NeoVim build/run/debug loop on the real project.

### Phase 3 — Minimal EPUB reader
- Add Readium via SPM (`https://github.com/readium/swift-toolkit.git`).
- Build: SwiftUI library/file-picker → open `.epub` → render with `EPUBNavigatorViewController`
  wrapped in `UIViewControllerRepresentable`.
- Target: open a book, paginate, persist reading position. Ship this as the "basic reader."

### Phase 4 — Differentiator features
- Add the features missing in other readers (the point of the project), building on Readium's
  navigator events/decorations.

### Phase 5 — macOS (deferred, deliberately)
- Add a **Mac Catalyst** destination to the iOS target — lowest-effort path that keeps the Readium
  UIKit Navigator working. Treat a native AppKit reader as a separate, later investigation only if
  Catalyst's UX disappoints.

**Expectation-setters:** (1) keep Xcode for signing, Instruments, and asset catalogs — NeoVim is ~90%,
not 100%; (2) the macOS-native story is the weakest part of the stack, which is why it's last.

---

## Open questions (worth resolving before/at the relevant phase)
- Does Readium support macOS (AppKit/Catalyst) for a shared codebase, or must macOS use a different
  rendering path? (develop-branch manifest declares iOS only)
- How well does codelldb/nvim-dap debugging perform on **physical devices** (vs simulator) in 2026?
  (physical-device debugging reportedly needs `pymobiledevice3`)
- Is the sourcekit-lsp background-indexing CPU issue (#2346) reproducible on macOS/NeoVim, and what
  mitigations are recommended?
- Current real-world state of code signing/provisioning **entirely outside Xcode**
  (fastlane match, `xcodebuild -allowProvisioningUpdates`, manual profiles)?

## Key sources
- swift.org — [Configuring Neovim for Swift](https://www.swift.org/documentation/articles/zero-to-swift-nvim.html)
- Wojciech Kulik — [Complete guide to iOS/macOS dev in Neovim](https://wojciechkulik.pl/ios/the-complete-guide-to-ios-macos-development-in-neovim)
- [xcodebuild.nvim](https://github.com/wojciech-kulik/xcodebuild.nvim) · [Neovim config wiki](https://github.com/wojciech-kulik/xcodebuild.nvim/wiki/Neovim-Configuration)
- [xcode-build-server](https://github.com/SolaWing/xcode-build-server) · [sourcekit-lsp](https://github.com/swiftlang/sourcekit-lsp)
- [Tuist docs](https://docs.tuist.dev/en/guides/features/projects)
- [Readium Swift Toolkit](https://github.com/readium/swift-toolkit) · [SwiftUI guide](https://github.com/readium/swift-toolkit/blob/develop/docs/Guides/Navigator/SwiftUI.md)
- [The Swift Programming Language](https://docs.swift.org/swift-book/) · [API Design Guidelines](https://www.swift.org/documentation/api-design-guidelines/) · [Apple SwiftUI tutorials](https://developer.apple.com/tutorials/swiftui)
