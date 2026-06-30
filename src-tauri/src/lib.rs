use std::{sync::Mutex, time::Duration};
use font8x8::UnicodeFonts;

use sysinfo::{Networks, System};
use tauri::{
    menu::{CheckMenuItem, Menu, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
#[cfg(not(target_os = "windows"))]
use tauri::image::Image;

// ── 图标尺寸（平台相关）────────────────────────────────────────────────────────
// macOS：宽条形，适配菜单栏（SCALE=2 对应 Retina）
#[cfg(target_os = "macos")]
const SCALE: u32 = 2;
#[cfg(target_os = "macos")]
const ICON_W: u32 = 72;
#[cfg(target_os = "macos")]
const ICON_H: u32 = 20;

// Windows/Linux：32×32 正方形，适配系统托盘图标槽
#[cfg(not(target_os = "macos"))]
const SCALE: u32 = 1;
#[cfg(not(target_os = "macos"))]
const ICON_W: u32 = 32;
#[cfg(not(target_os = "macos"))]
const ICON_H: u32 = 32;

const PW: u32 = ICON_W * SCALE;
const PH: u32 = ICON_H * SCALE;

// ── 自定义箭头 8×8 位图 ────────────────────────────────────────────────────────
#[rustfmt::skip]
const GLYPH_DOWN: [u8; 8] = [
    0b00011000,
    0b00011000,
    0b00011000,
    0b11111110,
    0b01111100,
    0b00111000,
    0b00010000,
    0b00000000,
];
#[rustfmt::skip]
const GLYPH_UP: [u8; 8] = [
    0b00010000,
    0b00111000,
    0b01111100,
    0b11111110,
    0b00011000,
    0b00011000,
    0b00011000,
    0b00000000,
];

fn get_glyph(c: char) -> [u8; 8] {
    match c {
        '↓' => GLYPH_DOWN,
        '↑' => GLYPH_UP,
        _ => font8x8::BASIC_FONTS.get(c).unwrap_or([0u8; 8]),
    }
}

fn draw_char(buf: &mut [u8], x: u32, y: u32, glyph: &[u8; 8], r: u8, g: u8, b: u8) {
    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..8u32 {
            if bits & (1 << col) != 0 {
                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        let px = x + col * SCALE + sx;
                        let py = y + row as u32 * SCALE + sy;
                        if px < PW && py < PH {
                            let i = ((py * PW + px) * 4) as usize;
                            buf[i]     = r;
                            buf[i + 1] = g;
                            buf[i + 2] = b;
                            buf[i + 3] = 255;
                        }
                    }
                }
            }
        }
    }
}

// 与 SpeedFormatter.swift 一致
fn format_speed(bps: f64) -> (String, &'static str) {
    let kb = 1024.0f64;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let frac = |b: f64, u: f64| -> String {
        format!(".{}", (b / u * 10.0).floor().max(1.0).min(9.0) as u32)
    };
    if bps < 100.0         { (format!("{}", bps as u32),                    "B") }
    else if bps < kb       { (frac(bps, kb),                                "K") }
    else if bps < 100.0*kb { (format!("{}", (bps/kb)  as u32),             "K") }
    else if bps < mb       { (frac(bps, mb),                                "M") }
    else if bps < 100.0*mb { (format!("{}", (bps/mb)  as u32),             "M") }
    else if bps < gb       { (frac(bps, gb),                                "G") }
    else                   { (format!("{}", ((bps/gb) as u32).min(99)),     "G") }
}

fn render_icon(cpu: i32, mem: i32, down_bps: f64, up_bps: f64) -> Vec<u8> {
    let (d_num, d_unit) = format_speed(down_bps);
    let (u_num, u_unit) = format_speed(up_bps);
    let mut buf = vec![0u8; (PW * PH * 4) as usize];

    // macOS：宽条形两行布局，黑色字体（配合 template 模式自动深/浅色适配）
    #[cfg(target_os = "macos")]
    {
        let row1 = format!("c{:>2}%m{:>2}%", cpu.min(99), mem.min(99));
        let row2 = format!("↓{:>2}{}↑{:>2}{}", d_num, d_unit, u_num, u_unit);
        let left = SCALE * 2;
        for (i, c) in row1.chars().enumerate() {
            draw_char(&mut buf, left + i as u32 * 8 * SCALE, SCALE * 2, &get_glyph(c), 0, 0, 0);
        }
        for (i, c) in row2.chars().enumerate() {
            draw_char(&mut buf, left + i as u32 * 8 * SCALE, SCALE * 12, &get_glyph(c), 0, 0, 0);
        }
    }

    // Windows/Linux：32×32 正方形四行布局，白色字体（适配深色系统托盘背景）
    // 每行 4 字符 × 8px = 32px，共 4 行 × 8px = 32px
    #[cfg(not(target_os = "macos"))]
    {
        let lines: [String; 4] = [
            format!("c{:>2}%", cpu.min(99)),
            format!("m{:>2}%", mem.min(99)),
            format!("↓{:>2}{}", d_num, d_unit),
            format!("↑{:>2}{}", u_num, u_unit),
        ];
        for (row_i, line) in lines.iter().enumerate() {
            let y = row_i as u32 * 8 * SCALE;
            for (col_i, c) in line.chars().enumerate() {
                draw_char(&mut buf, col_i as u32 * 8 * SCALE, y, &get_glyph(c), 255, 255, 255);
            }
        }
    }

    buf
}

// ── macOS 内存：直接调 host_statistics64，口径与活动监视器一致 ─────────────────
#[cfg(target_os = "macos")]
mod macos_mem {
    use std::{ffi::c_void, mem};

    const HOST_VM_INFO64: i32 = 4;
    const KERN_SUCCESS: i32 = 0;

    // 对应 vm_statistics64_data_t（macOS <mach/vm_statistics.h>）
    #[repr(C)]
    struct VmStatistics64 {
        free_count:              u32,
        active_count:            u32,
        inactive_count:          u32,
        wire_count:              u32,
        zero_fill_count:         u64,
        reactivations:           u64,
        pageins:                 u64,
        pageouts:                u64,
        faults:                  u64,
        cow_faults:              u64,
        lookups:                 u64,
        hits:                    u64,
        purges:                  u64,
        purgeable_count:         u32,
        speculative_count:       u32,
        decompressions:          u64,
        compressions:            u64,
        swapins:                 u64,
        swapouts:                u64,
        compressor_page_count:   u32,
        throttled_count:         u32,
        external_page_count:     u32,
        internal_page_count:     u32,
        total_uncompressed_pages_in_compressor: u64,
    }

    extern "C" {
        fn mach_host_self() -> u32;
        fn host_statistics64(host: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
        static vm_kernel_page_size: u32; // 与 Swift 版 vm_kernel_page_size 相同
        fn sysctlbyname(
            name: *const u8,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> i32;
    }

    /// 返回 (已用内存 bytes, 总内存 bytes)，口径 = active + wired + compressor
    pub fn read() -> Option<(u64, u64)> {
        let page_size = unsafe { vm_kernel_page_size } as u64;

        let mut stats: VmStatistics64 = unsafe { mem::zeroed() };
        let mut count = (mem::size_of::<VmStatistics64>() / mem::size_of::<i32>()) as u32;
        let kr = unsafe {
            host_statistics64(
                mach_host_self(),
                HOST_VM_INFO64,
                &mut stats as *mut _ as *mut i32,
                &mut count,
            )
        };
        if kr != KERN_SUCCESS {
            return None;
        }

        let used = (stats.active_count as u64
            + stats.wire_count as u64
            + stats.compressor_page_count as u64)
            * page_size;

        let mut total: u64 = 0;
        let mut sz = mem::size_of::<u64>();
        unsafe {
            sysctlbyname(
                b"hw.memsize\0".as_ptr(),
                &mut total as *mut u64 as *mut c_void,
                &mut sz,
                std::ptr::null_mut(),
                0,
            );
        }

        Some((used, total))
    }
}

// ── 系统状态 ───────────────────────────────────────────────────────────────────

struct SysState {
    sys: Mutex<System>,
    networks: Mutex<Networks>,
    has_net_history: Mutex<bool>,
}

impl SysState {
    fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        Self {
            sys: Mutex::new(sys),
            networks: Mutex::new(Networks::new_with_refreshed_list()),
            has_net_history: Mutex::new(false),
        }
    }
}

struct Metrics {
    cpu_percent: i32,
    mem_percent: i32,
    download_bps: f64,
    upload_bps: f64,
}

fn collect_metrics(state: &SysState) -> Metrics {
    // CPU（sysinfo，跨平台）
    let cpu = {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.global_cpu_usage().round() as i32
    };

    // 内存：macOS 用直接 syscall 保证口径；其他平台用 sysinfo
    #[cfg(target_os = "macos")]
    let (mem_used, mem_total) = macos_mem::read().unwrap_or((0, 1));

    #[cfg(not(target_os = "macos"))]
    let (mem_used, mem_total) = {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_memory();
        (sys.used_memory(), sys.total_memory())
    };

    let mem_percent = if mem_total > 0 { (mem_used * 100 / mem_total) as i32 } else { 0 };

    // 网络：按平台过滤物理接口
    // macOS：en*（以太网/Wi-Fi）
    // Linux：en*/eth*（以太网）+ wl*（Wi-Fi）
    // Windows：排除回环，统计所有真实接口
    let mut networks = state.networks.lock().unwrap();
    networks.refresh(false);
    let (rx, tx) = networks
        .iter()
        .filter(|(name, _)| {
            #[cfg(target_os = "macos")]
            { name.starts_with("en") }
            #[cfg(target_os = "linux")]
            { name.starts_with("en") || name.starts_with("eth") || name.starts_with("wl") }
            #[cfg(target_os = "windows")]
            { !name.to_lowercase().contains("loopback") && !name.is_empty() }
        })
        .fold((0u64, 0u64), |(a, b), (_, d)| (a + d.received(), b + d.transmitted()));

    let mut has_history = state.has_net_history.lock().unwrap();
    let (download_bps, upload_bps) = if !*has_history {
        *has_history = true;
        (0.0, 0.0)
    } else {
        (rx as f64, tx as f64)
    };

    Metrics { cpu_percent: cpu, mem_percent, download_bps, upload_bps }
}

fn start_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let state = app.state::<SysState>();
            let m = collect_metrics(&state);

            // macOS / Linux：更新托盘图标（文字像素渲染）
            // Windows 不走这条路：信息由 overlay 悬浮窗显示，托盘保持默认图标
            #[cfg(not(target_os = "windows"))]
            {
                let pixels = render_icon(m.cpu_percent, m.mem_percent, m.download_bps, m.upload_bps);
                let img = Image::new_owned(pixels, PW, PH);
                // 在同一个 run loop iteration 内完成 set_icon + set_icon_as_template，
                // CoreAnimation 合并为一次 display commit，避免两帧之间的闪动。
                let app2 = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(tray) = app2.tray_by_id("main") {
                        let _ = tray.set_icon(Some(img));
                        #[cfg(target_os = "macos")]
                        let _ = tray.set_icon_as_template(true);
                    }
                });
            }

            // Windows：把指标推送到任务栏悬浮窗（overlay window）
            #[cfg(target_os = "windows")]
            {
                let (d_num, d_unit) = format_speed(m.download_bps);
                let (u_num, u_unit) = format_speed(m.upload_bps);
                let line1 = format!("CPU:{:>3}%  ↑:{}{}", m.cpu_percent.min(99), u_num, u_unit);
                let line2 = format!("内存:{:>2}%  ↓:{}{}", m.mem_percent.min(99), d_num, d_unit);
                let _ = app.emit("metrics-update", format!("{}\n{}", line1, line2));
            }
        }
    });
}

// ── 语言检测 ───────────────────────────────────────────────────────────────────

fn is_chinese() -> bool {
    // 读 macOS 系统首选语言（与 Swift 版 Locale.preferredLanguages 等价）
    if let Ok(out) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
    {
        if let Ok(text) = String::from_utf8(out.stdout) {
            // 输出示例：(\n    "zh-Hans-CN",\n    en,\n)\n
            for line in text.lines() {
                let t = line.trim().trim_matches(|c: char| c == '"' || c == ',');
                if t.is_empty() || t == "(" || t == ")" {
                    continue;
                }
                return t.starts_with("zh");
            }
        }
    }
    // 降级：读 LANG 环境变量
    std::env::var("LANG").map(|l| l.starts_with("zh")).unwrap_or(false)
}

// ── 菜单 ────────────────────────────────────────────────────────────────────────

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri_plugin_autostart::ManagerExt;
    let is_on = app.autolaunch().is_enabled().unwrap_or(false);
    let cn = is_chinese();
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        if cn { "开机启动" } else { "Launch at Login" },
        true,
        is_on,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = PredefinedMenuItem::quit(app, Some(if cn { "退出" } else { "Quit" }))?;
    Menu::with_items(app, &[&autostart, &sep, &quit])
}

// ── 入口 ────────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(SysState::new())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let menu = build_menu(app.handle())?;

            let builder = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "autostart" {
                        use tauri_plugin_autostart::ManagerExt;
                        let al = app.autolaunch();
                        if al.is_enabled().unwrap_or(false) {
                            let _ = al.disable();
                        } else {
                            let _ = al.enable();
                        }
                    }
                });

            // 标记初始图标为 template，深/浅色由系统自动处理
            #[cfg(target_os = "macos")]
            let builder = builder.icon_as_template(true);

            builder.build(app)?;

            // Windows：在任务栏左侧创建透明悬浮窗
            // 窗口位于屏幕底部（与任务栏等高），always_on_top 保证显示在任务栏上方
            #[cfg(target_os = "windows")]
            {
                let overlay = tauri::WebviewWindowBuilder::new(
                    app,
                    "overlay",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("看一眼")
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .inner_size(175.0, 40.0)
                .resizable(false)
                .visible(false)  // 先隐藏，定位完成后再显示，避免位置跳变
                .build()?;

                // 获取主显示器逻辑分辨率，计算任务栏位置
                let screen_h = overlay
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map(|m| m.size().height as f64 / m.scale_factor())
                    .unwrap_or(1080.0); // 无法获取时退回 1080p
                let _ = overlay.set_position(tauri::LogicalPosition::new(0.0_f64, screen_h - 40.0));
                // build() 之后再次设置 always_on_top，确保 z-order 在任务栏之上
                let _ = overlay.set_always_on_top(true);
                let _ = overlay.show();
            }

            start_monitor(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
