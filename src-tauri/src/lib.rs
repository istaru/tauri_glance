use std::{sync::Mutex, time::Duration};

use sysinfo::{Networks, System};
use tauri::{
    menu::{CheckMenuItem, Menu, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
// 仅 macOS 在托盘菜单栏里画文字图标（Core Text 系统字体）；Windows/Linux 走悬浮窗
#[cfg(target_os = "macos")]
use tauri::image::Image;

// ── 图标尺寸（仅 macOS 托盘菜单栏用）──────────────────────────────────────────
// macOS：宽条形，适配菜单栏（SCALE=2 对应 Retina）。
// Windows/Linux 不在托盘画文字，改用悬浮 WebView 小窗，故无需这些常量。
#[cfg(target_os = "macos")]
const SCALE: u32 = 2;
// 9 列 × 14px(cell) + 左右内边距 ≈ 132px(@2x) → 66 逻辑像素，刚好包住文字
#[cfg(target_os = "macos")]
const ICON_W: u32 = 66;
#[cfg(target_os = "macos")]
const ICON_H: u32 = 20;
#[cfg(target_os = "macos")]
const PW: u32 = ICON_W * SCALE;
#[cfg(target_os = "macos")]
const PH: u32 = ICON_H * SCALE;

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

// 仅 macOS：用 Core Text 以系统字体把两行版式光栅化进菜单栏宽条图标。
// 逐字符在等宽格子里居中绘制 —— 既用系统字体（抗锯齿），又保证数字每秒变化不抖动、
// 与 Windows/Linux 的等宽两行版式一致。黑字 + template 由系统自动深/浅色反相。
#[cfg(target_os = "macos")]
fn render_icon(cpu: i32, mem: i32, down_bps: f64, up_bps: f64) -> Vec<u8> {
    use core_foundation::attributed_string::CFMutableAttributedString;
    use core_foundation::base::{TCFType, CFRange};
    use core_foundation::string::CFString;
    use core_graphics::base::kCGImageAlphaPremultipliedLast;
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_text::font as ct_font;
    use core_text::line::CTLine;
    use core_text::string_attributes::kCTFontAttributeName;

    let w = PW as usize;
    let h = PH as usize;
    let cell = 14.0; // 每个字符的等宽格子（物理像素）；加粗后字形更宽，格子相应加大
    let left = 3.0;
    let font_size = 18.0; // 物理像素；@2x 图标在菜单栏显示为 ~9pt 两行（更大更醒目）
    let (row1, row2) = format_rows(cpu, mem, down_bps, up_bps);

    // CGBitmapContext 原点在左下角，故"视觉上排"用较大的 y、"下排"用较小的 y。
    // 内存布局是顶行在前，绘制-底 映射到 内存-底，输出即为上而下，正对 tauri Image。
    let cs = CGColorSpace::create_device_rgb();
    let mut ctx = CGContext::create_bitmap_context(None, w, h, 8, w * 4, &cs, kCGImageAlphaPremultipliedLast);
    ctx.set_should_antialias(true);
    ctx.set_should_smooth_fonts(true);
    ctx.set_rgb_fill_color(0.0, 0.0, 0.0, 1.0); // 黑字（template 反相）

    // 加粗系统字体（emphasized system font = 菜单栏系统字体的 bold 变体）
    let font = ct_font::new_ui_font_for_language(
        ct_font::kCTFontEmphasizedSystemFontType,
        font_size,
        None,
    );

    let draw_glyph = |c: char, cell_x: f64, baseline: f64| {
        if c == ' ' {
            return;
        }
        let mut astr = CFMutableAttributedString::new();
        let s = CFString::new(&c.to_string());
        astr.replace_str(&s, CFRange::init(0, 0));
        let len = astr.char_len();
        astr.set_attribute(
            CFRange::init(0, len),
            unsafe { kCTFontAttributeName },
            &font,
        );
        let line = CTLine::new_with_attributed_string(astr.as_concrete_TypeRef());
        let bounds = line.get_typographic_bounds();
        let x = cell_x + (cell - bounds.width) / 2.0; // 在格子里水平居中
        ctx.set_text_position(x, baseline);
        line.draw(&ctx);
    };

    let top_baseline = 22.0; // 上排基线（原点在左下角，故上排用较大 y）
    let bot_baseline = 3.0; // 下排基线（贴近底部）
    for (i, c) in row1.chars().enumerate() {
        draw_glyph(c, left + i as f64 * cell, top_baseline);
    }
    for (i, c) in row2.chars().enumerate() {
        draw_glyph(c, left + i as f64 * cell, bot_baseline);
    }

    ctx.data().to_vec()
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

// ── Windows：把顶层透明窗口叠加在任务栏上（不 SetParent）──────────────────────
// 之前用 SetParent 做成任务栏子窗口虽然"真嵌入"，但子窗口收不到鼠标事件 → 无法拖动。
// 改为顶层置顶窗口叠在任务栏那一条上：视觉上仍像嵌在任务栏里，但能正常收鼠标、可拖动。
// 用屏幕坐标定位（与 Linux 统一），默认放在时钟区(TrayNotifyWnd)左侧、垂直居中于任务栏。
#[cfg(target_os = "windows")]
fn place_on_taskbar(win: &tauri::WebviewWindow) {
    use tauri::PhysicalPosition;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let (w, h) = match win.outer_size() {
        Ok(s) => (s.width as i32, s.height as i32),
        Err(_) => (132, 44),
    };

    unsafe {
        let cls_taskbar = wide("Shell_TrayWnd");
        let taskbar = FindWindowW(cls_taskbar.as_ptr(), std::ptr::null());
        if taskbar.is_null() {
            return;
        }
        // 任务栏屏幕矩形
        let mut tb = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(taskbar, &mut tb);

        // 默认 x：通知区(TrayNotifyWnd)左边缘 = 「显示隐藏图标」的 ^ 箭头左边缘，
        // 让小窗右边缘紧挨该箭头左侧（GAP=2，视觉上贴住又不压到箭头的 hover 区）。
        const GAP: i32 = 2;
        let cls_tray = wide("TrayNotifyWnd");
        let tray = FindWindowExW(taskbar, std::ptr::null_mut(), cls_tray.as_ptr(), std::ptr::null());
        let default_x = if !tray.is_null() {
            let mut tr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(tray, &mut tr);
            tr.left - w - GAP
        } else {
            tb.right - w - 200
        };
        // 拖动存档（屏幕 x）优先，限制在任务栏 x 范围内
        let min_x = tb.left;
        let max_x = (tb.right - w).max(tb.left);
        let x = load_widget_x(win.app_handle())
            .map(|sx| sx.clamp(min_x, max_x))
            .unwrap_or(default_x)
            .clamp(min_x, max_x);
        let y = tb.top + (tb.bottom - tb.top - h) / 2; // 垂直居中于任务栏

        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
}

// ── 悬浮窗位置：持久化 + 拖动 ────────────────────────────────────────────────────

// 位置存档文件（应用配置目录下的 widget_x）
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn widget_pos_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("widget_x"))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn load_widget_x(app: &AppHandle) -> Option<i32> {
    std::fs::read_to_string(widget_pos_file(app)?)
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn save_widget_x(app: &AppHandle, x: i32) {
    if let Some(f) = widget_pos_file(app) {
        let _ = std::fs::write(f, x.to_string());
    }
}

// 拖动中：把 widget 水平移动 dx 物理像素（前端 pointermove 实时调用）。
// Windows/Linux 都是顶层窗口，统一用屏幕坐标的 set_position 平移（只动 x，保持在任务栏那一行）。
#[tauri::command]
fn move_widget(app: AppHandle, dx: i32) {
    let _ = (&app, dx);
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    if let Some(win) = app.get_webview_window("widget") {
        if let Ok(pos) = win.outer_position() {
            let _ = win.set_position(tauri::PhysicalPosition::new(pos.x + dx, pos.y));
        }
    }
}

// 拖动结束：把当前屏幕 x 存盘
#[tauri::command]
fn save_widget_pos(app: AppHandle) {
    let _ = &app;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    if let Some(win) = app.get_webview_window("widget") {
        if let Ok(pos) = win.outer_position() {
            save_widget_x(&app, pos.x);
        }
    }
}

// ── 入口 ────────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例守卫必须最先注册：第二次启动会触发此回调（在已运行的实例里）后自行退出。
        // 本应用常驻托盘/任务栏，无主窗口可聚焦，回调留空即可——关键是第二个进程不再起来。
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {}))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .invoke_handler(tauri::generate_handler![move_widget, save_widget_pos])
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

            // Windows/Linux：创建指标小窗（无边框 / 透明 / 跳过任务栏）。
            // 尺寸刚好容纳两行各 9 个等宽字符 —— 内容定宽，故固定尺寸即"自适应"。
            // 不开点击穿透：widget 需接收鼠标以支持拖动定位。
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

                // Windows：把顶层窗口叠加定位到任务栏上（默认时钟左侧，有存档则用存档 x）
                #[cfg(target_os = "windows")]
                place_on_taskbar(&win);

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
                        let max_x = (scr.width as i32 - wsz.width as i32).max(0);
                        // 有拖动存档则用存档 x，否则默认贴右侧
                        let x = load_widget_x(win.app_handle())
                            .map(|sx| sx.clamp(0, max_x))
                            .unwrap_or(max_x - margin);
                        let y = scr.height as i32 - wsz.height as i32 - panel;
                        let _ = win.set_position(PhysicalPosition::new(x, y.max(0)));
                    }
                }
            }

            start_monitor(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 把 macOS 图标渲染存成 PNG（黑字合成到白底以便肉眼查看），用于本机视觉验证。
// 运行：cargo test --lib preview_icon -- --nocapture
#[cfg(all(test, target_os = "macos"))]
mod preview {
    #[test]
    fn preview_icon() {
        let px = super::render_icon(8, 57, 27_000.0, 400_000.0); // c 8% m57% ↓27K ↑.4M
        let (w, h) = (super::PW, super::PH);
        let mut img = image::RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let a = px[i + 3] as f32 / 255.0;
                let v = (255.0 * (1.0 - a)) as u8; // 黑字合成到白底
                img.put_pixel(x, y, image::Rgba([v, v, v, 255]));
            }
        }
        let out = "/private/tmp/claude-501/-Users-julian-AIProjects-tauri-glance/07326ce3-6888-44e7-9b66-246e947b71ec/scratchpad/icon_preview.png";
        img.save(out).unwrap();
        println!("wrote {out}");
    }
}
