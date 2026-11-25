// The oldest toolchain this target builds with: Swift 6.3.3, which ships with
// Xcode 26.6 — the Xcode CI builds with. Older Swift 6.2 type checkers reject
// code 6.3 accepts (an ambiguous `CGFloat` / `Double` operator once broke the
// macOS tests on Xcode 26.3), so an older Xcode stops here with one clear
// message instead of unrelated compile errors. It follows the CI pin
// (`DEVELOPER_DIR` in `.github/workflows/macos-build.yml`): raise it with that
// pin, the other `ToolchainFloor.swift` and `docs/BUILDING.md` § 2.
#if !compiler(>=6.3.3)
    #error("TaigiKeyboard needs Xcode 26.6 (Swift 6.3.3) or newer — see docs/BUILDING.md § 2")
#endif
