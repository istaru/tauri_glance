# 纯 Swift 菜单栏系统监控 —— 交接规格

> 本文件是自包含的需求 + UI + 算法规格。把它交给一个新的 Swift 项目（及新的 AI 会话），无需任何 Flutter 源码即可 1:1 复现原应用。原应用是 Flutter 实现的 macOS 菜单栏监控，现要用**纯 Swift / AppKit** 重写以大幅缩小体积（Flutter 包 18M → 纯 Swift 预计 <1M）。

---

## 1. 应用形态（最重要）

- **纯菜单栏（menu bar）常驻应用**：没有主窗口、没有 Dock 图标、不出现在 ⌘Tab。
  - 实现：`Info.plist` 设 `LSUIElement = true`（或 `NSApp.setActivationPolicy(.accessory)`）。
- 启动后只在系统菜单栏（屏幕右上角）显示一个 `NSStatusItem`，实时刷新。
- 点击状态栏图标弹出菜单，菜单**只有一项「退出」**（`NSApplication.terminate`）。
- 每 **1 秒**刷新一次全部数据。

---

## 2. 监控的 4 个指标

| 指标 | 含义 | 单位/范围 |
|------|------|-----------|
| CPU 使用率 | 全核平均占用 | 0–100 (%) |
| 内存使用率 | 已用/总量百分比 | 0–100 (%) |
| 下载速度 | 网络接收速率 | KB/s 或 MB/s |
| 上传速度 | 网络发送速率 | KB/s 或 MB/s |
| (附带) 内存已用 / 总量 | 字节数，用于算百分比 | bytes |

---

## 3. 数据采集算法（macOS）

> 原 Flutter 版本是用 `Process` 跑系统命令实现的。下面**每项都给两种实现**：
> **(A) 命令法**——直接照搬，保证数字和原版完全一致，但需关闭沙盒（`com.apple.security.app-sandbox = false`），且每秒起子进程较重。
> **(B) 原生 API 法**——推荐，无需沙盒例外、更省资源、更优雅。新项目优先选 B，行为对齐 A。

### 3.1 CPU 使用率

**(A) 命令法**：`ps -A -o %cpu`
- 对输出（跳过表头第一行）每行的 `%cpu` 求和，得到 `total`。
- `cpuUsage = clamp(total / 核心数, 0, 100)`，核心数取 `ProcessInfo.processInfo.activeProcessorCount`。

**(B) 原生法**：`host_processor_info(... PROCESSOR_CPU_LOAD_INFO ...)`
- 读取每核的 user/system/nice/idle ticks，与上一秒差分：
  `usage = (Δuser + Δsystem + Δnice) / (Δuser + Δsystem + Δnice + Δidle) * 100`，跨核平均。
- 这是 Activity Monitor 式算法，比 ps 更准。首秒无历史，返回 0。

### 3.2 内存

**(A) 命令法**：
- 总量：`sysctl -n hw.memsize` → 字节数（**唯一可靠来源**，别用 hw.physmem）。
- 已用：`vm_stat`，解析三项页数后乘 page size：
  - `page size of N bytes`（正则取 N，缺省 4096）
  - `Pages active`、`Pages wired down`、`Pages occupied by compressor`（每项正则 `KEY:\s+(\d+)\.`）
  - `used = (active + wired + compressed) * pageSize`（与活动监视器口径一致）
- `memoryUsage = clamp(used / total * 100, 0, 100)`

**(B) 原生法**：
- 总量：`sysctlbyname("hw.memsize", ...)`。
- 已用：`host_statistics64(... HOST_VM_INFO64 ...)` 得 `vm_statistics64_data_t`，
  `used = (active_count + wire_count + compressor_page_count) * vm_page_size`。
- 口径与命令法完全相同，推荐用 B。

### 3.3 网络速度

**(A) 命令法**：`netstat -ibn`
- 遍历每行，**只处理接口名以 `en` 开头的行**（en0/en1…，即以太网/Wi-Fi）。
- 按空白切分，`parts[6]` = 接收字节累计(Ibytes)，`parts[9]` = 发送字节累计(Obytes)，累加得 `rx`、`tx`。
  - ⚠️ 原版会把同一接口的多行重复累加（netstat 每个地址族一行），数字偏大。B 法更准。

**(B) 原生法**：`getifaddrs()` + `AF_LINK`
- 对每个 `AF_LINK` 接口，取 `if_data->ifi_ibytes` / `ifi_obytes`。
- 建议只统计活动物理接口（名字前缀 en），跳过 lo0。

**速率计算（两法通用）**：
- 保存上一秒的 `rx`/`tx`（`previousRxBytes` / `previousTxBytes`）。
- `downloadSpeed(KB/s) = max(0, (rx - prevRx) / 1024)`；上传同理用 tx。
- **首次采样无历史，速度显示 0**（prev 初始为 0 时跳过差分）。

---

## 4. 状态栏 UI 布局（像素级规格）

状态栏图标是**自绘的两排两列位图**（不是文字 title）。原 Swift 实现的核心参数如下，照此复现：

```
第一排：  ↓123K/s   │   ↑45K/s
第二排：  C 78%      │   M 56%
```

- `NSStatusItem`：`NSStatusBar.system.statusItem(withLength: .variableLength)`，`button.imagePosition = .imageOnly`。
- 字体：`NSFont.monospacedDigitSystemFont(ofSize: 8, weight: .bold)`。
- 段落：`NSMutableParagraphStyle`，`alignment = .center`。
- **列宽统一**：以最宽内容 `"↓999K/s"` 测量宽度作为单列宽 `colW`（保证两排上下对齐、数字跳动时不抖动）。
  - `lineH = 测得高度`，`colGap = 1`，`imgW = colW*2 + colGap`，`imgH = lineH*2 + 2`。
  - 第一排 y = `lineH + 2`，第二排 y = `0`；左列 x=0，右列 x=`colW + colGap`。
- **模板图标**：`image.isTemplate = true` —— 系统自动按菜单栏深/浅色反相，无需自己判断主题。
- CPU/内存始终用单字母缩写：左 `C {cpu}%`，右 `M {mem}%`（cpu/mem 为整数）。

### 可直接复用的绘制函数（来自原项目，已验证）

```swift
static func makeStatusImage(
  down: String, up: String,
  cpuVal: String, memVal: String,
  cpuShort: String = "C", memShort: String = "M"
) -> NSImage {
  let font = NSFont.monospacedDigitSystemFont(ofSize: 8, weight: .bold)
  let para = NSMutableParagraphStyle(); para.alignment = .center
  let attrs: [NSAttributedString.Key: Any] = [
    .font: font, .foregroundColor: NSColor.black, .paragraphStyle: para
  ]
  let maxNet = NSAttributedString(string: "↓999K/s", attributes: attrs)
  let lineH = maxNet.size().height
  let colW  = maxNet.size().width
  let colGap: CGFloat = 1
  let imgW = colW * 2 + colGap
  let imgH = lineH * 2 + 2
  let cpuLabel = "\(cpuShort) \(cpuVal)%"
  let memLabel = "\(memShort) \(memVal)%"
  let img = NSImage(size: NSSize(width: imgW, height: imgH), flipped: false) { _ in
    let row1Y = lineH + 2
    NSAttributedString(string: down, attributes: attrs)
      .draw(in: CGRect(x: 0, y: row1Y, width: colW, height: lineH))
    NSAttributedString(string: up, attributes: attrs)
      .draw(in: CGRect(x: colW + colGap, y: row1Y, width: colW, height: lineH))
    NSAttributedString(string: cpuLabel, attributes: attrs)
      .draw(in: CGRect(x: 0, y: 0, width: colW, height: lineH))
    NSAttributedString(string: memLabel, attributes: attrs)
      .draw(in: CGRect(x: colW + colGap, y: 0, width: colW, height: lineH))
    return true
  }
  img.isTemplate = true
  return img
}
```

---

## 5. 数值格式化

**速度**（输入为 KB/s 的 double）：
```swift
func speedLabel(_ kbps: Double) -> String {
  if kbps < 1024 { return "\(min(Int(kbps), 999))K/s" }       // 整数，封顶 999
  else           { return "\(min(Int(kbps/1024), 999))M/s" }  // 整数，封顶 999
}
```
- 状态栏第一排显示 `"↓" + speedLabel(down)` 和 `"↑" + speedLabel(up)`。
- CPU / 内存在状态栏取整显示（`Int(value)`）。

> 注：原浮窗里还有更精细的格式（如 `12.34 MB/s`、CPU 一位小数），但新版**没有浮窗**，状态栏只需上面的紧凑整数格式。

---

## 6. 菜单与退出

- 点击 `NSStatusItem` 弹出 `NSMenu`，**仅一项**：「退出 / Quit」，action = `#selector(NSApplication.terminate(_:))`，keyEquivalent `"q"`。
- 无「显示主窗口」之类项（新版没有任何窗口）。

---

## 7. 国际化（可选，低优先级）

- 原版只把两个缩写做了 i18n：CPU→`C`、Memory→`M`（默认值就是 C/M）。
- 纯 Swift 版可直接硬编码 `C`/`M`，或用 `Localizable.strings`。菜单「退出」可按 `Locale` 显示「退出」/「Quit」。

---

## 8. 沙盒 / entitlements

- 若用**命令法(A)**：必须 `com.apple.security.app-sandbox = false`（沙盒禁止 fork 子进程跑 ps/netstat）。
- 若用**原生 API 法(B)**：无需任何特殊 entitlement，可保持沙盒开启或干脆不加 entitlements。**这也是推荐 B 的另一个理由。**

---

## 9. 推荐技术栈与项目结构

- **Swift + AppKit**（不用 SwiftUI 也行；菜单栏图标自绘用 AppKit 最直接）。
- 最小可以是一个 SwiftPM 可执行包，或一个 Xcode macOS App（App target，删掉默认窗口）。
- 关键骨架：
  - `AppDelegate`：`applicationDidFinishLaunching` 里建 `NSStatusItem` + `NSMenu`，启动 1 秒 `Timer`。
  - `SystemMonitor`：封装 §3 的采集（建议用 B 法），输出 cpu/mem/down/up。
  - 每秒：采集 → `makeStatusImage(...)` → `statusItem.button?.image = img`。
- 体积目标：< 1MB（链接系统框架，不捆绑运行时）。

---

## 10. 验收清单

- [ ] 启动后无 Dock 图标、无窗口，仅菜单栏出现两排两列图标。
- [ ] 第一排 ↓下载/↑上传，第二排 C cpu%/M mem%，上下两排左右对齐、数字跳动不抖动。
- [ ] 深色/浅色菜单栏下文字颜色自动正确（template image）。
- [ ] 数值每秒刷新；首秒网速为 0。
- [ ] 内存口径与「活动监视器」接近（active+wired+compressed）。
- [ ] 菜单只有「退出」，点击能退出。
- [ ] 打包体积 < 1MB。
```
