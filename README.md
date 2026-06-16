# SwiftGlance

A lightweight macOS menu bar app that displays real-time CPU usage, memory usage, download speed, and upload speed — all in a compact two-row icon. Built with pure Swift + AppKit, no Electron, no Flutter, no bloat.

> 中文介绍请见 [README_ZH.md](README_ZH.md)

---

## Preview

```
┌──────────────────────┐
│  ↓  27  K   ↑  .4  M │  ← row 1: network speed
│  c   8  %   m  57  % │  ← row 2: CPU / memory
└──────────────────────┘
```

The icon lives in your menu bar and updates every second. No window, no Dock icon, no ⌘Tab entry.

---

## Features

- **CPU usage** — all-core average via `host_processor_info` (Activity Monitor algorithm)
- **Memory usage** — active + wired + compressed pages / total (`host_statistics64`)
- **Network speed** — download & upload via `getifaddrs`, physical `en*` interfaces only; full range from B/s to GB/s
- **Smart speed format** — numbers always 1–2 digits; switches to `.X` fractional notation when crossing unit boundaries (e.g. `↓.4M` = 400 KB/s)
- **Dark / Light mode** — automatic via `isTemplate = true`, no manual theming needed
- **Launch at Login** — toggle from the menu (uses `SMAppService` on macOS 13+, LaunchAgent plist on macOS 12)
- **Pinned position** — ⌘-drag the icon to your preferred spot once; macOS remembers it permanently across restarts and when other apps add their own status items
- **Tiny footprint** — ~152 KB bundle, negligible CPU/RAM overhead

---

## Requirements

| Item | Minimum |
|------|---------|
| macOS | 12.0 Monterey |
| Xcode / Swift | Swift 5.9+ (`swift build`) |
| Architecture | Apple Silicon & Intel (arm64 / x86_64) |

---

## Installation

### Option A — Download pre-built binary (easiest)

1. Go to the [Releases page](https://github.com/istaru/swift_glance/releases/latest)
2. Download `SwiftGlance.zip` and unzip it
3. Move `SwiftGlance.app` to `/Applications/`
4. Open it — macOS may ask you to confirm the first launch in System Settings → Privacy & Security

The release binary is a **Universal Binary** (Apple Silicon + Intel).

### Option B — Build from source

```bash
git clone https://github.com/istaru/swift_glance.git
cd swift_glance
bash build_app.sh
cp -r SwiftGlance.app /Applications/
open /Applications/SwiftGlance.app
```

---

## Project Structure

```
swift_glance/
├── Sources/SwiftGlance/
│   ├── main.swift               # Entry point — sets up NSApplication
│   ├── AppDelegate.swift        # Status item, menu, 1-second timer
│   ├── SystemMonitor.swift      # CPU / memory / network sampling
│   ├── StatusImage.swift        # Draws the two-row bitmap icon
│   └── LaunchAtLoginManager.swift  # SMAppService + legacy LaunchAgent
├── Package.swift
├── SwiftGlance.entitlements
├── build_app.sh                 # Release build + .app assembly script
└── CLAUDE.md                    # Full technical spec (AI-readable)
```

---

## How It Works

### CPU
Uses `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` to read per-core tick counts each second, then computes the delta:

```
usage = (Δuser + Δsystem + Δnice) / (Δuser + Δsystem + Δnice + Δidle) × 100
```

Averaged across all cores. First sample returns 0 (no history yet).

### Memory
`host_statistics64(HOST_VM_INFO64)` gives page counts. Memory total is read once at startup via `sysctlbyname("hw.memsize")` and cached:

```
used = (active_count + wire_count + compressor_page_count) × page_size
```

This matches Activity Monitor's "used memory" definition.

### Network
`getifaddrs()` with `AF_LINK` socket addresses, filtered to `en*` interfaces (Ethernet / Wi-Fi). Raw byte counters are diffed each second to produce bytes/s. Handles counter wrap-around gracefully.

### Speed Formatting
Numbers are always kept to 1–2 digits. When a value exceeds 2 digits in the current unit, it switches to a `.X` fractional representation of the next unit:

| Range | Display | Example |
|-------|---------|---------|
| 0–99 B/s | `XXB` | `63B` |
| 100–1023 B/s | `.XK` | `.5K` |
| 1–99 KB/s | `XXK` | `45K` |
| 100KB–1023KB/s | `.XM` | `.4M` |
| 1–99 MB/s | `XXM` | `12M` |
| 100MB–1023MB/s | `.XG` | `.3G` |
| ≥ 1 GB/s | `XXG` | `2G` |

### Status Icon
Drawn with `NSAttributedString` into an `NSImage` at runtime. Each column is split into three sub-columns — **symbol** (left-aligned), **number** (right-aligned), **unit** (left-aligned) — so all rows align precisely regardless of value width. `isTemplate = true` lets the system apply the correct foreground color for any menu bar appearance.

---

## Performance Optimizations

- `hw.memsize` and `vm_kernel_page_size` are read once at init and cached — no repeated syscalls
- `NSFont`, attributes dictionaries, and all icon dimensions are `static let` — allocated once, not every second
- Network sampling returns raw bytes/s, avoiding premature precision loss from early division
- No subprocess spawning (unlike shell-command approaches), no polling threads

---

## Menu

Right-click (or click) the status item to open the menu:

| Item | Action |
|------|--------|
| Launch at Login | Toggle auto-start on login |
| Quit | Terminate the app |

**Pinning the icon position**: ⌘-drag the SwiftGlance icon to your preferred position in the menu bar. macOS persists this via `autosaveName` and restores it on every launch — even after other apps add or remove their own status items.

The menu title respects the system locale — Chinese or English.

---

## Building & Signing

`build_app.sh` performs an ad-hoc code signature (`codesign --sign -`) sufficient for local use. For distribution, replace with a Developer ID signature:

```bash
codesign --force --deep --sign "Developer ID Application: Your Name (TEAMID)" SwiftGlance.app
```

---

## License

MIT — do whatever you want, no warranty.
