import Foundation
import ServiceManagement

enum LaunchAtLoginManager {

    static var isEnabled: Bool {
        if #available(macOS 13.0, *) {
            return SMAppService.mainApp.status == .enabled
        } else {
            return FileManager.default.fileExists(atPath: legacyPlistPath)
        }
    }

    static func setEnabled(_ enable: Bool) {
        if #available(macOS 13.0, *) {
            do {
                if enable {
                    try SMAppService.mainApp.register()
                } else {
                    try SMAppService.mainApp.unregister()
                }
            } catch {
                NSLog("[SwiftGlance] SMAppService error: %@", error.localizedDescription)
            }
        } else {
            legacySetEnabled(enable)
        }
    }

    // MARK: - macOS 12 降级实现（LaunchAgent plist）

    private static var legacyPlistPath: String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        return "\(home)/Library/LaunchAgents/com.swiftglance.menubar.plist"
    }

    private static func legacySetEnabled(_ enable: Bool) {
        let plistURL = URL(fileURLWithPath: legacyPlistPath)
        if enable {
            guard let execURL = Bundle.main.executableURL else { return }
            // 使用 .app bundle 的 open 命令，确保以 GUI 应用方式启动。
            let appURL = execURL.deletingLastPathComponent()    // Contents/MacOS
                                .deletingLastPathComponent()    // Contents
                                .deletingLastPathComponent()    // .app
            let plist: [String: Any] = [
                "Label": "com.swiftglance.menubar",
                "ProgramArguments": ["/usr/bin/open", "-a", appURL.path],
                "RunAtLoad": true,
                "KeepAlive": false
            ]
            do {
                let agentsDir = plistURL.deletingLastPathComponent()
                try FileManager.default.createDirectory(at: agentsDir,
                    withIntermediateDirectories: true)
                let data = try PropertyListSerialization.data(
                    fromPropertyList: plist, format: .xml, options: 0)
                try data.write(to: plistURL)
                runLaunchctl(["load", plistURL.path])
            } catch {
                NSLog("[SwiftGlance] LaunchAgent write error: %@", error.localizedDescription)
            }
        } else {
            runLaunchctl(["unload", plistURL.path])
            try? FileManager.default.removeItem(at: plistURL)
        }
    }

    private static func runLaunchctl(_ args: [String]) {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/bin/launchctl")
        p.arguments = args
        try? p.run()
        p.waitUntilExit()
    }
}
