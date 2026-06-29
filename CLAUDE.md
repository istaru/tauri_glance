# SwiftGlance — 技术规格（当前实现）

> 纯 Swift + AppKit 的 macOS 菜单栏系统监控。本文件描述**当前已实现的**代码行为，可作为新会话的完整上下文。

---

## 1. 应用形态

- **纯菜单栏常驻应用**：无主窗口、无 Dock 图标、不出现在 ⌘Tab。
  - 实现：`NSApp.setActivationPolicy(.accessory)`
- 每 **1 秒** 刷新一次状态栏图标。
- `NSStatusItem.autosaveName = "com.swiftglance.menubar"` —— 系统记忆图标位置，用户 ⌘ 拖拽一次后永久固定。

---

## 2. 数据采集（SystemMonitor.swift）

### 模块结构（内部 seam）

`SystemMonitor` 是对外门面，interface 仅 `sample() -> Metrics`，实现内部沿一道 seam 拆成两块：

- **`CounterSource`（端口）** + **`LiveCounters`（活体 adapter）** —— 读原始累计计数器（`RawCounters`），碰 syscall，无差分状态。
- **`RateDiffer`（纯差分）** —— 持有差分历史，把相邻两次 `RawCounters` 变成 `Metrics`。CPU 多核平均、网络回绕、首样本归零、内存百分比口径全在此处。

测试通过 `SystemMonitor(source:)` 注入脚本化 adapter，无需碰活体硬件即可覆盖差分 edge case（见 `Tests/SwiftGlanceTests/RateDifferTests.swift`）。

### Metrics 结构体

```swift
struct Metrics {
    var cpuUsage: Double      // 0–100 (%)
    var memoryUsage: Double   // 0–100 (%)
    var memoryUsed: UInt64    // bytes
    var memoryTotal: UInt64   // bytes
    var downloadBps: Double   // bytes/s（原始字节，非 KB）
    var uploadBps: Double     // bytes/s
}
```

### 性能优化

- `memoryTotal`（`hw.memsize`）和 `pageSize`（`vm_kernel_page_size`）在 `LiveCounters.init()` 里一次性读取并缓存，每秒采样不再重复 syscall。

### 3.1 CPU

`host_processor_info(PROCESSOR_CPU_LOAD_INFO)` 差分计算：
```
usage = (Δuser + Δsystem + Δnice) / (Δuser + Δsystem + Δnice + Δidle) × 100
```
跨核平均，首秒无历史返回 0。

### 3.2 内存

`host_statistics64(HOST_VM_INFO64)`：
```
used = (active_count + wire_count + compressor_page_count) × pageSize
```
口径与 Activity Monitor 一致。

### 3.3 网络

`getifaddrs()` + `AF_LINK`，只统计 `en*` 物理接口（以太网/Wi-Fi），跳过 lo0。  
返回**原始 bytes/s**（不除以 1024），由 `SpeedFormatter.format()` 按档格式化。  
首次采样无历史，速度为 0；处理计数回绕（差为负时按 0）。

---

## 3. 状态栏 UI（StatusImage.swift）

### 布局结构

图标分**两列**，每列内部分**三个子列**精确对齐：

```
[ 左列                 ]  [ 右列                 ]
  c  [数字右对齐]  [单位]    m  [数字右对齐]  [单位]
  ↓  [数字右对齐]  [单位]    ↑  [数字右对齐]  [单位]
```

实际效果示例：
```
c   8  %     m  57  %
↓  27  K     ↑  .4  M
```

### 子列宽度（均为 static let，启动时计算一次）

| 子列 | 计算方式 |
|------|----------|
| `symbolW` | `max(width("↓"), width("↑"), width("c"), width("m"))` |
| `numberW` | `max(width("99"), width(".9"))` |
| `unitW` | `max(width("B"), width("K"), width("M"), width("G"), width("%"))` |
| `mainColW` | `symbolW + numberW + unitW` |

### 图片尺寸

```
imgW = sidePad + mainColW + colGap + mainColW + sidePad
imgH = lineH * 2 + 2
```

- `sidePad = 2`：两侧等量内边距，保证视觉居中
- `colGap = 3`：两列之间间距

### 对齐方式

- **符号列、单位列**：左对齐（`lAttrs`）
- **数字列**：右对齐（`rAttrs`）—— 保证 1 位和 2 位数字右边对齐

### 绘制函数签名

```swift
static func makeStatusImage(
    downloadBps: Double,
    uploadBps: Double,
    cpuPercent: Int,
    memPercent: Int
) -> NSImage
```

`img.isTemplate = true` —— 系统自动处理深/浅色反相。

---

## 4. 网速格式化（SpeedFormatter.swift）

输入为 **bytes/s**，数字始终控制在 **1–2 位**，跨档用 `.X` 小数表示：

| 速度范围 | 格式 | 示例 |
|----------|------|------|
| 0–99 B/s | `XXB` | `7B`、`63B` |
| 100–1023 B/s | `.XK` | `100B/s→.1K`、`512B/s→.5K` |
| 1KB–99KB/s | `XXK` | `1K`、`45K`、`99K` |
| 100KB–1023KB/s | `.XM` | `100K/s→.1M`、`500K/s→.4M` |
| 1MB–99MB/s | `XXM` | `1M`、`55M`、`99M` |
| 100MB–1023MB/s | `.XG` | `500M/s→.4G` |
| ≥ 1GB/s | `XXG` | `1G`、`10G` |

小数计算：`max(1, min(9, Int(bps / nextUnit * 10)))`，保证始终显示 `.1`–`.9`，不出现 `.0`。

---

## 5. 菜单

```
开机启动  ✓          (Launch at Login)
——————————
退出  ⌘Q            (Quit)
```

- 「开机启动」：macOS 13+ 用 `SMAppService`，macOS 12 用 LaunchAgent plist（`/Library/LaunchAgents/com.swiftglance.menubar.plist`）。
- 菜单文字按系统语言自动切换中/英文（`Locale.preferredLanguages`）。

---

## 6. 性能优化清单

| 优化项 | 位置 |
|--------|------|
| `hw.memsize` 和 `vm_kernel_page_size` 只在 init 读一次 | `LiveCounters.init()` |
| `NSFont`、`attrs` 字典、图标尺寸全部 `static let` | `StatusImage` |
| 网络速度返回原始 bytes/s，避免提前精度损失 | `LiveCounters.readNetwork()` |
| `isChinese()` 每次 `buildMenu()` 只调用一次 | `AppDelegate.buildMenu()` |

---

## 7. 构建与安装

```bash
bash build_app.sh          # release 构建 + 组装 看一眼.app + ad-hoc 签名
cp -r 看一眼.app /Applications/
open /Applications/看一眼.app
```

- **应用名**：对外显示名为「看一眼」（`CFBundleName`/`CFBundleDisplayName`）；可执行名与 Bundle ID 保持 ASCII（`SwiftGlance` / `com.swiftglance.menubar`），不随显示名变动。
- **应用图标**：`Resources/AppIcon.icns`（极简仪表盘），由 `Resources/AppIcon/make_icon.swift` 渲染主图后经 `sips` + `iconutil` 生成；菜单栏内的动态文字图标不受影响。

包体积目标：< 1MB（当前约 720KB，含 .icns）。

---

## 8. 验收清单

- [x] 启动后无 Dock 图标、无窗口，仅菜单栏出现图标
- [x] 两排三列对齐：符号/数字/单位上下精确对齐，数字右对齐不抖动
- [x] 深/浅色菜单栏自动适配（isTemplate）
- [x] 数值每秒刷新，首秒网速为 0
- [x] 内存口径与活动监视器一致（active+wired+compressed）
- [x] 网速全档覆盖：B → .XK → K → .XM → M → .XG → G，数字始终 ≤ 2 位
- [x] 菜单栏图标位置持久化（autosaveName，⌘ 拖拽后永久固定）
- [x] 菜单：开机启动开关 + 退出
- [x] 打包体积 < 1MB

---

---

## 9. Tauri 跨平台版（tauri-glance/）

> Rust + Tauri v2 重写，目标支持 Win / Linux / Mac。功能与 Swift 版对齐，但实现细节不同。

### 9.1 架构

- **纯托盘应用**：`windows: []`，无 WebView 窗口；`ActivationPolicy::Accessory`（macOS）
- **全 Rust 渲染**：`font8x8` 位图字体直接写像素缓冲区，不依赖 WebView/Canvas
- **后台线程**：每秒采集 → 渲染 → `run_on_main_thread` 闭包内同步调用 `set_icon` + `set_icon_as_template`

### 9.2 深/浅色适配要点

`set_icon()` 每次重置 template 状态，必须紧随调用 `set_icon_as_template(true)`。两次调用须在同一个 `run_on_main_thread` 闭包内执行，CoreAnimation 才能将其合并为单次 display commit，避免黑白闪动。切勿将两者分开 dispatch。

图标像素颜色为**黑色**（`0,0,0,255`），透明底，由 macOS template 机制自动在深色模式下反相为白色。

### 9.3 内存口径

macOS 上通过 `macos_mem::read()`（`host_statistics64` + `vm_kernel_page_size` + `sysctlbyname("hw.memsize")`）直接采集，口径 = `active + wired + compressor`，与活动监视器一致。非 macOS 降级使用 sysinfo。

### 9.4 网络接口

与 Swift 版相同：只统计 `en*` 前缀物理接口（以太网/Wi-Fi），排除 VPN（`utun*`）、回环（`lo`）等。

### 9.5 菜单语言

`is_chinese()` 读 `defaults read -g AppleLanguages`，首选语言以 `zh` 开头则显示中文，否则英文。

### 9.6 构建

```bash
cd tauri-glance
npx tauri build
# 产物：src-tauri/target/release/bundle/macos/看一眼.app
#       src-tauri/target/release/bundle/dmg/看一眼_0.1.0_aarch64.dmg
```

### 9.7 与 Swift 版的已知差异

| 项目 | Swift 版 | Tauri 版 |
|------|----------|----------|
| 字体 | `monospacedDigitSystemFont(8, .bold)` | `font8x8` 位图字体 |
| 图标位置持久化 | `autosaveName` | 不支持（Tauri 无此 API） |
| 平台 | macOS only | macOS / Windows / Linux |

---

## Agent skills

### Issue tracker

Issue 追踪在仓库的 GitHub Issues（使用 `gh` CLI）。详见 `docs/agents/issue-tracker.md`。

### Triage 标签

使用五个标准角色的默认标签名（`needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`）。详见 `docs/agents/triage-labels.md`。

### 领域文档

单上下文布局（根目录 `CONTEXT.md` + `docs/adr/`）。详见 `docs/agents/domain.md`。
