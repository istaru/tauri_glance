# SwiftGlance 中文介绍

SwiftGlance 是一款极简 macOS 菜单栏系统监控工具，实时显示 CPU 占用、内存使用率、下载/上传网速，全部压缩进一个两行图标里。纯 Swift + AppKit 实现，无 Electron、无 Flutter、无臃肿运行时。

---

## 界面预览

```
┌─────────────────────┐
│  ↓123K/s  ↑45K/s   │  ← 第一排：网络速度
│  C 78%    M 56%     │  ← 第二排：CPU / 内存
└─────────────────────┘
```

图标常驻菜单栏，每秒刷新。没有主窗口、没有 Dock 图标、不出现在 ⌘Tab 切换列表。

---

## 功能一览

| 功能 | 说明 |
|------|------|
| CPU 使用率 | 全核平均，采用 `host_processor_info`（与活动监视器算法一致） |
| 内存使用率 | active + wired + compressed 页数之和 / 总量 |
| 网络速度 | 仅统计 `en*` 物理接口（以太网/Wi-Fi），单位 KB/s 或 MB/s |
| 深/浅色自适应 | `isTemplate = true`，系统自动处理颜色反相 |
| 开机启动 | 菜单中一键切换（macOS 13+ 用 `SMAppService`，12 用 LaunchAgent） |
| 位置记忆 | 用 `⌘ + 拖拽` 把图标移到喜欢的位置，macOS 永久记住，重启或其他 app 启动都不会再跑位 |
| 极小体积 | 安装包约 136 KB |

---

## 系统要求

- macOS 12.0 Monterey 及以上
- Swift 5.9+（使用 `swift build` 编译）
- 支持 Apple Silicon 与 Intel

---

## 安装方式

```bash
git clone https://github.com/istaru/swift_glance.git
cd swift_glance
bash build_app.sh
cp -r SwiftGlance.app /Applications/
open /Applications/SwiftGlance.app
```

---

## 技术实现简述

- **CPU**：`host_processor_info` 差分计算，首秒返回 0
- **内存**：`host_statistics64` 读页数，总量启动时一次性缓存
- **网络**：`getifaddrs` + `AF_LINK`，差分计算速率
- **图标**：`NSAttributedString` 动态绘制，列宽固定防抖动，`isTemplate` 自适应外观

---

> 英文完整文档请见 [README.md](README.md)
