import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {

    private var statusItem: NSStatusItem!
    private let pipeline = StatusPipeline(sample: SystemMonitor().sample)
    private var timer: Timer?
    private var launchAtLoginItem: NSMenuItem!

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.autosaveName = "com.swiftglance.menubar"
        statusItem.button?.imagePosition = .imageOnly

        buildMenu()
        refresh()

        let t = Timer(timeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.refresh()
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    func applicationWillTerminate(_ notification: Notification) {
        timer?.invalidate()
    }

    // MARK: - 菜单

    private func buildMenu() {
        let menu = NSMenu()

        let isCN = isChinese()

        // 「开机启动」开关（带对勾状态）。
        launchAtLoginItem = NSMenuItem(
            title: isCN ? "开机启动" : "Launch at Login",
            action: #selector(toggleLaunchAtLogin),
            keyEquivalent: ""
        )
        launchAtLoginItem.target = self
        launchAtLoginItem.state = LaunchAtLoginManager.isEnabled ? .on : .off
        menu.addItem(launchAtLoginItem)

        menu.addItem(.separator())

        let quitTitle = isCN ? "退出" : "Quit"
        let quitItem = NSMenuItem(
            title: quitTitle,
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q"
        )
        quitItem.target = NSApp
        menu.addItem(quitItem)

        statusItem.menu = menu
    }

    @objc private func toggleLaunchAtLogin() {
        let newValue = !LaunchAtLoginManager.isEnabled
        LaunchAtLoginManager.setEnabled(newValue)
        launchAtLoginItem.state = newValue ? .on : .off
    }

    // MARK: - 每秒刷新

    private func refresh() {
        statusItem.button?.image = pipeline.tick()
    }

    private func isChinese() -> Bool {
        (Locale.preferredLanguages.first ?? "").hasPrefix("zh")
    }
}
