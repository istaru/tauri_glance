# 看一眼

A lightweight macOS menu bar app that displays real-time CPU usage, memory usage, download speed, and upload speed — all in a compact two-row icon. Built with Rust + Tauri v2, no Electron, no bloat.

> 中文介绍请见 [README_ZH.md](README_ZH.md)

---

## Preview

```
c  8%  m 57%
↓ 27K  ↑ .4M
```

## Features

- CPU usage (multi-core average)
- Memory usage (matches Activity Monitor: active + wired + compressed pages)
- Network speed (physical interfaces only, auto-formatted as B/K/M/G)
- Refreshes every second, adapts automatically to dark/light menu bar
- Launch at Login toggle, menu language follows system (English/Chinese)
- No Dock icon, no window

## Install

Download the latest `.zip` or `.dmg` from [Releases](../../releases).

## Build from Source

```bash
npm ci
npx tauri build
# Output: src-tauri/target/release/bundle/macos/看一眼.app
```

## Development

```bash
npm ci
npx tauri dev
```

## License

[MIT](LICENSE)
