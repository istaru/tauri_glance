use std::{sync::Mutex, time::Duration};
#[cfg(target_os = "macos")]
use font8x8::UnicodeFonts;

use sysinfo::{Networks, System};
use tauri::{
    menu::{CheckMenuItem, Menu, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
// 仅 macOS 在托盘菜单栏里画像素文字；Windows/Linux 走悬浮窗，不用托盘图像
#[cfg(target_os = "macos")]
use tauri::image::Image;

// ── 图标尺寸（仅 macOS 托盘菜单栏用）──────────────────────────────────────────
// macOS：宽条形，适配菜单栏（SCALE=2 对应 Retina）。
// Windows/Linux 不在托盘画文字，改用悬浮 WebView 小窗，故无需这些常量。
#[cfg(target_os = "macos")]
const SCALE: u32 = 2;
// 两行各 9 列（含组间空格）× 8px + 左右内边距，留到 78 以免末字符被裁
#[cfg(target_os = "macos")]
const ICON_W: u32 = 78;
#[cfg(target_os = "macos")]
const ICON_H: u32 = 20;
#[cfg(target_os = "macos")]
const PW: u32 = ICON_W * SCALE;
#[cfg(target_os = "macos")]
const PH: u32 = ICON_H * SCALE;

// ── 自定义箭头 8×8 位图（仅 macOS 托盘像素渲染用）─────────────────────────────
#[cfg(target_os = "macos")]
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
#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn get_glyph(c: char) -> [u8; 8] {
    match c {
        '↓' => GLYPH_DOWN,
        '↑' => GLYPH_UP,
        _ => font8x8::BASIC_FONTS.get(c).unwrap_or([0u8; 8]),
    }
}

#[cfg(target_os = "macos")]
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

// macOS 菜单栏图标 与 Windows/Linux 悬浮窗 共用的两行版式（单一来源，逐字符一致）：
//   row1 = "c97% m77%"   （CPU 组与内存组之间留一个空格）
//   row2 = "↓66B ↑ 0B"   （下载组与上传组之间留一个空格；数字右对齐宽度 2）
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn format_rows(cpu: i32, mem: i32, down_bps: f64, up_bps: f64) -> (String, String) {
    let (d_num, d_unit) = format_speed(down_bps);
    let (u_num, u_unit) = format_speed(up_bps);
    let row1 = format!("c{:>2}% m{:>2}%", cpu.min(99), mem.min(99));
    let row2 = format!("↓{:>2}{} ↑{:>2}{}", d_num, d_unit, u_num, u_unit);
    (row1, row2)
}

// 仅 macOS：把两行版式画进菜单栏宽条图标（黑字 + template 自动深/浅色反相）
#[cfg(target_os = "macos")]
fn render_icon(cpu: i32, mem: i32, down_bps: f64, up_bps: f64) -> Vec<u8> {
    let mut buf = vec![0u8; (PW * PH * 4) as usize];

    let (row1, row2) = format_rows(cpu, mem, down_bps, up_bps);
    let left = SCALE * 2;
    for (i, c) in row1.chars().enumerate() {
        draw_char(&mut buf, left + i as u32 * 8 * SCALE, SCALE * 2, &get_glyph(c), 0, 0, 0);
    }
    for (i, c) in row2.chars().enumerate() {
        draw_char(&mut buf, left + i as u32 * 8 * SCALE, SCALE * 12, &get_glyph(c), 0, 0, 0);
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

// Windows/Linux 悬浮窗：每秒 emit("metrics") 推两行文本，前端按 macOS 同款版式渲染
#[cfg(any(target_os = "windows", target_os = "linux"))]
#[derive(Clone, serde::Serialize)]
struct MetricsPayload {
    row1: String, // 例 "c97%m77%"
    row2: String, // 例 "↓66B↑ 0B"
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

            // Windows/Linux：把指标推给悬浮 WebView 小窗（托盘图标保持静态，仅承载菜单）
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            {
                use tauri::Emitter;
                let (row1, row2) = format_rows(
                    m.cpu_percent,
                    m.mem_percent,
                    m.download_bps,
                    m.upload_bps,
                );
                let _ = app.emit("metrics", MetricsPayload { row1, row2 });
            }

            // macOS：把指标渲染进托盘菜单栏像素图标
            #[cfg(target_os = "macos")]
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
        }
    });
}

// ── 语言检测 ───────────────────────────────────────────────────────────────────

fn is_chinese() -> bool {
    // macOS：读系统首选语言（与 Swift 版 Locale.preferredLanguages 等价）
    #[cfg(target_os = "macos")]
    {
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
        false
    }

    // Windows：取用户界面语言 LANGID，主语言号 == LANG_CHINESE(0x04) 即中文
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
        let langid = unsafe { GetUserDefaultUILanguage() };
        (langid & 0x3ff) == 0x04
    }

    // Linux：读 locale 环境变量（zh_CN.UTF-8 等）
    #[cfg(target_os = "linux")]
    {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(key) {
                if !v.is_empty() {
                    return v.to_lowercase().starts_with("zh");
                }
            }
        }
        false
    }
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

// ── Windows：把悬浮窗 SetParent 进任务栏，作为 Shell_TrayWnd 的子窗口 ──────────
// 注意：这是非官方手法，跨 Windows 版本/更新可能失效（Win11 无受支持的任务栏嵌入 API）。
#[cfg(target_os = "windows")]
fn embed_into_taskbar(win: &tauri::WebviewWindow) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, FindWindowW, GetWindowLongPtrW, GetWindowRect, SetParent,
        SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_NOZORDER, SWP_SHOWWINDOW, WS_CHILD,
        WS_POPUP, WS_VISIBLE,
    };

    // 生成 null 结尾的宽字符串
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // 取自身 HWND（tauri 的 windows::HWND，其 .0 是 *mut c_void，可直接当 windows-sys HWND）
    let hwnd = match win.hwnd() {
        Ok(h) => h.0 as _,
        Err(_) => return,
    };

    unsafe {
        let cls_taskbar = wide("Shell_TrayWnd");
        let taskbar = FindWindowW(cls_taskbar.as_ptr(), std::ptr::null());
        if taskbar.is_null() {
            return;
        }

        // 1) 去掉 WS_POPUP、加 WS_CHILD，使其成为可被 SetParent 收纳的子窗口
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style =
            (style & !(WS_POPUP as isize)) | (WS_CHILD as isize) | (WS_VISIBLE as isize);
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

        // 2) 认任务栏为父窗口
        SetParent(hwnd, taskbar);

        // 3) 量任务栏尺寸 + 自身物理尺寸
        let mut tb = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(taskbar, &mut tb);
        let tb_h = tb.bottom - tb.top;
        let (w, h) = match win.outer_size() {
            Ok(s) => (s.width as i32, s.height as i32),
            Err(_) => (120, 48),
        };

        // 4) 定位到时钟区域(TrayNotifyWnd)左侧、垂直居中于任务栏（坐标相对任务栏客户区）
        let cls_tray = wide("TrayNotifyWnd");
        let tray = FindWindowExW(
            taskbar,
            std::ptr::null_mut(),
            cls_tray.as_ptr(),
            std::ptr::null(),
        );
        let x = if !tray.is_null() {
            let mut tr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(tray, &mut tr);
            (tr.left - tb.left) - w - 8
        } else {
            (tb.right - tb.left) - w - 200
        };
        let y = (tb_h - h) / 2;

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            x.max(0),
            y.max(0),
            w,
            h,
            SWP_NOZORDER | SWP_SHOWWINDOW,
        );
    }
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

            // Windows/Linux：创建指标小窗（无边框 / 透明 / 跳过任务栏 / 点击穿透）。
            // 尺寸刚好容纳两行各 9 个等宽字符 —— 内容定宽，故固定尺寸即"自适应"。
            // macOS 不创建窗口，继续用菜单栏托盘图标。
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            {
                use tauri::{WebviewUrl, WebviewWindowBuilder};
                let win = WebviewWindowBuilder::new(
                    app,
                    "widget",
                    WebviewUrl::App("index.html".into()),
                )
                .title("看一眼")
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .focused(false)
                .inner_size(82.0, 36.0)
                .build()?;

                // 鼠标点击穿透，不挡到下方内容
                let _ = win.set_ignore_cursor_events(true);

                // Windows：SetParent 进 Shell_TrayWnd，真正嵌入任务栏（时钟左侧）
                #[cfg(target_os = "windows")]
                embed_into_taskbar(&win);

                // Linux：SetParent 是 Win32 专属，任务栏嵌入需按桌面环境另案处理。
                // 暂用悬浮窗贴屏幕右下角（面板位置因 DE 而异，留待真机微调）。
                #[cfg(target_os = "linux")]
                {
                    use tauri::PhysicalPosition;
                    if let Ok(Some(mon)) = win.primary_monitor() {
                        let scr = mon.size();
                        let sf = mon.scale_factor();
                        let wsz = win
                            .outer_size()
                            .unwrap_or(tauri::PhysicalSize { width: 82, height: 36 });
                        let margin = (8.0 * sf) as i32;
                        let panel = (40.0 * sf) as i32;
                        let x = scr.width as i32 - wsz.width as i32 - margin;
                        let y = scr.height as i32 - wsz.height as i32 - panel;
                        let _ = win.set_position(PhysicalPosition::new(x.max(0), y.max(0)));
                    }
                }
            }

            start_monitor(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
