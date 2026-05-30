# SwiftGlance

A lightweight macOS menu bar app that displays real-time CPU usage, memory usage, download speed, and upload speed — all in a compact two-row icon. Built with pure Swift + AppKit, no Electron, no Flutter, no bloat.

> 中文介绍请见 [README_ZH.md](README_ZH.md)

---

## Preview

```
┌─────────────────────┐
│  ↓123K/s  ↑45K/s   │  ← row 1: network speed
│  C 78%    M 56%     │  ← row 2: CPU / memory
└─────────────────────┘
```

The icon lives in your menu bar and updates every second. No window, no Dock icon, no ⌘Tab entry.

---

## Features

- **CPU usage** — all-core average via `host_processor_info` (Activity Monitor algorithm)
- **Memory usage** — active + wired + compressed pages / total (`host_statistics64`)
- **Network speed** — download & upload via `getifaddrs`, physical `en*` interfaces only
- **Dark / Light mode** — automatic via `isTemplate = true`, no manual theming needed
- **Launch at Login** — toggle from the menu (uses `SMAppService` on macOS 13+, LaunchAgent plist on macOS 12)
- **Tiny footprint** — ~136 KB bundle, negligible CPU/RAM overhead

---

## Requirements

| Item | Minimum |
|------|---------|
| macOS | 12.0 Monterey |
| Xcode / Swift | Swift 5.9+ (`swift build`) |
| Architecture | Apple Silicon & Intel (arm64 / x86_64) |

---

## Installation

### Option A — Build from source

```bash
git clone https://github.com/istaru/swift_glance.git
cd swift_glance
bash build_app.sh
cp -r SwiftGlance.app /Applications/
open /Applications/SwiftGlance.app
```

### Option B — Manual build steps

```bash
swift build -c release
# The compiled binary is at .build/release/SwiftGlance
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
`getifaddrs()` with `AF_LINK` socket addresses, filtered to `en*` interfaces (Ethernet / Wi-Fi). Byte counters are diffed each second to produce KB/s rates. Handles counter wrap-around gracefully.

### Status Icon
Drawn with `NSAttributedString` into an `NSImage` at runtime. Column width is fixed to the widest possible string (`↓999K/s`) so digits never cause layout jitter. `isTemplate = true` lets the system apply the correct foreground color for any menu bar appearance.

---

## Performance Optimizations

- `hw.memsize` and `vm_kernel_page_size` are read once at init and cached — no repeated syscalls
- `NSFont`, `NSMutableParagraphStyle`, `attrs` dictionary, and icon dimensions are `static let` — allocated once, not every second
- No subprocess spawning (unlike shell-command approaches), no polling threads

---

## Menu

Right-click (or click) the status item to open the menu:

| Item | Action |
|------|--------|
| Launch at Login | Toggle auto-start on login |
| Quit | Terminate the app |

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
