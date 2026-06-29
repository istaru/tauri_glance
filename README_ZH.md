# 看一眼

极简 macOS 菜单栏系统监控工具，实时显示 CPU 占用、内存使用率、下载/上传网速，全部压缩进一个两行图标里。基于 Rust + Tauri v2，无 Electron、无臃肿运行时。

---

## 效果预览

```
c  8%  m 57%
↓ 27K  ↑ .4M
```

## 功能

- CPU 使用率（多核平均）
- 内存占用（口径与活动监视器一致：active + wired + compressed）
- 网络上下行速度（仅统计物理网卡，自动格式化为 B/K/M/G）
- 每秒刷新，深/浅色菜单栏自动适配
- 开机启动开关，菜单语言跟随系统（中/英）
- 无 Dock 图标，无窗口

## 安装

从 [Releases](../../releases) 下载最新 `.zip` 或 `.dmg`，解压后把 `看一眼.app` 拖入应用程序文件夹即可。

## 从源码构建

```bash
npm ci
npx tauri build
# 产物在 src-tauri/target/release/bundle/macos/
```

## 开发调试

```bash
npm ci
npx tauri dev
```

## 协议

[MIT](LICENSE)
