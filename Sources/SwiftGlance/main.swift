import AppKit

// SwiftPM 可执行入口：手动搭建 NSApplication + AppDelegate。
let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
