use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow, Window, window::Color};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItem};
use tauri_plugin_opener::OpenerExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use winapi::um::winuser::{GetLastInputInfo, LASTINPUTINFO};
use winapi::um::sysinfoapi::GetTickCount;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;

#[derive(Debug, Serialize, Deserialize)]
struct SavedWindowState {
    x: i32,
    y: i32,
    width: Option<u32>,
    height: Option<u32>,
}

const DEFAULT_WIDGET_WIDTH: f64 = 360.0;
const DEFAULT_WIDGET_HEIGHT: f64 = 382.0;
const MIN_WIDGET_WIDTH: f64 = 340.0;
const MIN_WIDGET_HEIGHT: f64 = 382.0;

fn window_state_file(app: &tauri::AppHandle) -> Option<PathBuf> {
    let mut dir = app.path().app_data_dir().ok()?;
    dir.push("window-state.json");
    Some(dir)
}

fn save_window_state(window: &Window) {
    let Some(path) = window_state_file(&window.app_handle()) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(position) = window.outer_position() {
        let size = window.outer_size().ok();
        let data = SavedWindowState {
            x: position.x,
            y: position.y,
            width: size.map(|s| s.width),
            height: size.map(|s| s.height),
        };

        if let Ok(json) = serde_json::to_string(&data) {
            let _ = fs::write(path, json);
        }
    }
}

fn save_webview_window_state(window: &WebviewWindow) {
    let Some(path) = window_state_file(&window.app_handle()) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(position) = window.outer_position() {
        let size = window.outer_size().ok();
        let data = SavedWindowState {
            x: position.x,
            y: position.y,
            width: size.map(|s| s.width),
            height: size.map(|s| s.height),
        };

        if let Ok(json) = serde_json::to_string(&data) {
            let _ = fs::write(path, json);
        }
    }
}

fn restore_window_state(window: &WebviewWindow) {
    let Some(path) = window_state_file(&window.app_handle()) else {
        return;
    };

    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };

    let Ok(saved) = serde_json::from_str::<SavedWindowState>(&raw) else {
        return;
    };

    if let (Some(width), Some(height)) = (saved.width, saved.height) {
        if width > 0 && height > 0 {
            let _ = window.set_size(Size::Physical(PhysicalSize::new(width, height)));
        }
    }
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(saved.x, saved.y)));
}

fn reset_window_to_default(window: &WebviewWindow) {
    let _ = window.set_size(Size::Logical(LogicalSize::new(DEFAULT_WIDGET_WIDTH, DEFAULT_WIDGET_HEIGHT)));
    let _ = window.center();
}

fn clear_saved_window_state(app: &tauri::AppHandle) {
    if let Some(path) = window_state_file(app) {
        let _ = fs::remove_file(path);
    }
}


#[tauri::command]
fn get_idle_seconds() -> f64 {
    unsafe {
        let mut info: LASTINPUTINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<LASTINPUTINFO>() as u32;
        if GetLastInputInfo(&mut info) != 0 {
            let ticks = GetTickCount();
            let idle_ticks = ticks - info.dwTime;
            return (idle_ticks as f64) / 1000.0;
        }
    }
    0.0
}

#[tauri::command]
fn send_pavlok_alert(token: String, stimulus_type: String) -> String {
    let client = reqwest::blocking::Client::new();
    let url = "https://api.pavlok.com/api/v5/stimulus/send";
    let token_trimmed = token.trim();
    let token_value = {
        let mut parts = token_trimmed.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some(scheme), Some(value)) if scheme.eq_ignore_ascii_case("bearer") => value,
            _ => token_trimmed,
        }
    };
    
    let actual_type = match stimulus_type.as_str() {
        "vibro" => "vibe",
        "vibration" => "vibe",
        "vibe" => "vibe",
        "zap" => "zap",
        _ => "beep",
    };

    let payload = serde_json::json!({
        "stimulus": {
            "stimulusType": actual_type,
            "stimulusValue": 100
        },
        "reason": "Fatigue limit"
    });

    match client.post(url)
        .header("Authorization", format!("Bearer {}", token_value))
        .json(&payload)
        .send() {
            Ok(res) => {
                if res.status().is_success() {
                    return "Sent".to_string();
                } else {
                    return format!("Error: {}", res.status());
                }
            },
            Err(e) => return format!("Failed: {}", e),
        }
}

#[tauri::command]
fn start_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = HWND(win32_handle.hwnd.get() as _);
                unsafe {
                    let _ = ReleaseCapture();
                    let _ = SendMessageW(hwnd, 0x0112, WPARAM(0xF012), LPARAM(0));
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn start_resize_drag(window: Window, direction: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = win32_handle.hwnd.get() as isize;

                // SC_SIZE direction codes:
                // w=1, e=2, n=3, nw=4, ne=5, s=6, sw=7, se=8
                let command = match direction.as_str() {
                    "w" => 0xF001,
                    "e" => 0xF002,
                    "n" => 0xF003,
                    "nw" => 0xF004,
                    "ne" => 0xF005,
                    "s" => 0xF006,
                    "sw" => 0xF007,
                    "se" => 0xF008,
                    _ => return Ok(()),
                };

                unsafe {
                    let _ = ReleaseCapture();
                    let _ = SendMessageW(HWND(hwnd as _), 0x0112, WPARAM(command), LPARAM(0));
                }
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_webview_borderless(window: &WebviewWindow) -> Result<(), String> {
    let _ = window.set_shadow(false);
    let _ = window.set_resizable(false);
    let _ = window.set_min_size(Some(Size::Logical(LogicalSize::new(MIN_WIDGET_WIDTH, MIN_WIDGET_HEIGHT))));
    let _ = window.set_title("");
    let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            let hwnd = win32_handle.hwnd.get() as isize;

            #[link(name = "user32")]
            extern "system" {
                fn GetWindowLongPtrW(hwnd: isize, n_index: i32) -> isize;
                fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
                fn SetWindowPos(hwnd: isize, hwnd_insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, u_flags: u32) -> i32;
                fn SetClassLongPtrW(hWnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
                fn SetWindowTextW(hWnd: isize, lpString: *const u16) -> i32;
                fn GetWindowRect(hwnd: isize, lpRect: *mut RECT) -> i32;
                fn SetWindowRgn(hwnd: isize, hRgn: isize, bRedraw: i32) -> i32;
                fn EnumChildWindows(hWndParent: isize, lpEnumFunc: unsafe extern "system" fn(hwnd: isize, _lparam: isize) -> i32, l_param: isize) -> i32;
            }
            #[link(name = "gdi32")]
            extern "system" {
                fn GetStockObject(i: i32) -> isize;
                fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, width: i32, height: i32) -> isize;
            }
            #[link(name = "dwmapi")]
            extern "system" {
                fn DwmExtendFrameIntoClientArea(hwnd: isize, pMarInset: *const MARGINS) -> i32;
                fn DwmSetWindowAttribute(hwnd: isize, dwAttribute: u32, pvAttribute: *const std::ffi::c_void, cbAttribute: u32) -> i32;
            }

            const GWL_STYLE: i32 = -16;
            const GWL_EXSTYLE: i32 = -20;
            const WS_POPUP: isize = 0x80000000u32 as isize;
            const WS_VISIBLE: isize = 0x10000000;
            const WS_EX_LAYERED: isize = 0x00080000;
            const WS_EX_TOOLWINDOW: isize = 0x00000080;
            const SWP_FRAMECHANGED: u32 = 0x0020;
            const SWP_NOMOVE: u32 = 0x0002;
            const SWP_NOSIZE: u32 = 0x0001;
            const SWP_NOZORDER: u32 = 0x0004;
            const GCL_HBRBACKGROUND: i32 = -10;
            const NULL_BRUSH: i32 = 5;

            const WS_CAPTION: isize = 0x00C00000;
            const WS_THICKFRAME: isize = 0x00040000;
            const WS_SYSMENU: isize = 0x00080000;
            const WS_MAXIMIZEBOX: isize = 0x00010000;
            const WS_MINIMIZEBOX: isize = 0x00020000;

            #[repr(C)]
            #[allow(non_snake_case)]
            struct MARGINS {
                cxLeftWidth: i32,
                cxRightWidth: i32,
                cyTopHeight: i32,
                cyBottomHeight: i32,
            }

            #[repr(C)]
            struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

            unsafe {
                let current_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let new_style = (current_style & !WS_CAPTION & !WS_THICKFRAME & !WS_SYSMENU & !WS_MAXIMIZEBOX & !WS_MINIMIZEBOX) | WS_POPUP | WS_VISIBLE;
                SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

                let current_exstyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let new_exstyle = current_exstyle | WS_EX_LAYERED | WS_EX_TOOLWINDOW;
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_exstyle);

                SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);

                let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 0, cyBottomHeight: 0 };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                let ncrendering_policy: u32 = 1;
                let dwmwa_ncrendering_policy = 2;
                let corner_preference: u32 = 1;
                let dwmwa_window_corner_preference = 33;
                let border_color: u32 = 0xFFFFFFFE;
                let dwmwa_border_color = 34;

                let _ = DwmSetWindowAttribute(hwnd, dwmwa_ncrendering_policy, &ncrendering_policy as *const u32 as *const std::ffi::c_void, 4);
                let _ = DwmSetWindowAttribute(hwnd, dwmwa_window_corner_preference, &corner_preference as *const u32 as *const std::ffi::c_void, 4);
                let _ = DwmSetWindowAttribute(hwnd, dwmwa_border_color, &border_color as *const u32 as *const std::ffi::c_void, 4);

                let null_brush = GetStockObject(NULL_BRUSH);
                SetClassLongPtrW(hwnd, GCL_HBRBACKGROUND, null_brush);

                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                let empty_title: Vec<u16> = OsStr::new("").encode_wide().chain(Some(0)).collect();
                SetWindowTextW(hwnd, empty_title.as_ptr());

                EnumChildWindows(hwnd, enum_child_proc, 0);

                let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut rect);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                if width > 0 && height > 0 {
                    let top_gutter = 38;
                    let side = (height - top_gutter).min(width);
                    let x_offset = (width - side) / 2;
                    let corner_radius = 24;
                    let h_rgn = CreateRoundRectRgn(
                        x_offset,
                        top_gutter,
                        x_offset + side + 1,
                        top_gutter + side + 1,
                        corner_radius * 2,
                        corner_radius * 2
                    );
                    SetWindowRgn(hwnd, h_rgn, 1);
                }
            }
        }
    }
    Ok(())
}

// Helper to enforce borderless state without aggressive style stripping
#[cfg(target_os = "windows")]
fn ensure_borderless(window: &tauri::Window) -> Result<(), String> {
    let _ = window.set_decorations(false);
    let _ = window.set_shadow(false);
    let _ = window.set_resizable(false);
    let _ = window.set_min_size(Some(Size::Logical(LogicalSize::new(MIN_WIDGET_WIDTH, MIN_WIDGET_HEIGHT))));
    let _ = window.set_title("");
    let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            let hwnd = win32_handle.hwnd.get() as isize;

            #[link(name = "user32")]
            extern "system" {
                fn GetWindowLongPtrW(hwnd: isize, n_index: i32) -> isize;
                fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
                fn SetWindowPos(hwnd: isize, hwnd_insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, u_flags: u32) -> i32;
                fn SetClassLongPtrW(hWnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
                fn SetWindowTextW(hWnd: isize, lpString: *const u16) -> i32;
                fn GetWindowRect(hwnd: isize, lpRect: *mut RECT) -> i32;
                fn SetWindowRgn(hwnd: isize, hRgn: isize, bRedraw: i32) -> i32;
                fn EnumChildWindows(hWndParent: isize, lpEnumFunc: unsafe extern "system" fn(hwnd: isize, _lparam: isize) -> i32, l_param: isize) -> i32;
            }
            #[link(name = "gdi32")]
            extern "system" {
                fn GetStockObject(i: i32) -> isize;
                fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, width: i32, height: i32) -> isize;
            }
            #[link(name = "dwmapi")]
            extern "system" {
                fn DwmExtendFrameIntoClientArea(hwnd: isize, pMarInset: *const MARGINS) -> i32;
                fn DwmSetWindowAttribute(hwnd: isize, dwAttribute: u32, pvAttribute: *const std::ffi::c_void, cbAttribute: u32) -> i32;
            }

            const GWL_STYLE: i32 = -16;
            const GWL_EXSTYLE: i32 = -20;
            const WS_POPUP: isize = 0x80000000u32 as isize;
            const WS_VISIBLE: isize = 0x10000000;
            const WS_EX_LAYERED: isize = 0x00080000;
            const WS_EX_TOOLWINDOW: isize = 0x00000080;
            const SWP_FRAMECHANGED: u32 = 0x0020;
            const SWP_NOMOVE: u32 = 0x0002;
            const SWP_NOSIZE: u32 = 0x0001;
            const SWP_NOZORDER: u32 = 0x0004;
            const GCL_HBRBACKGROUND: i32 = -10;
            const NULL_BRUSH: i32 = 5;

            const WS_CAPTION: isize = 0x00C00000;
            const WS_THICKFRAME: isize = 0x00040000;
            const WS_SYSMENU: isize = 0x00080000;
            const WS_MAXIMIZEBOX: isize = 0x00010000;
            const WS_MINIMIZEBOX: isize = 0x00020000;

            #[repr(C)]
            #[allow(non_snake_case)]
            struct MARGINS {
                cxLeftWidth: i32,
                cxRightWidth: i32,
                cyTopHeight: i32,
                cyBottomHeight: i32,
            }

            #[repr(C)]
            struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

            unsafe {
                let current_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let new_style = (current_style & !WS_CAPTION & !WS_THICKFRAME & !WS_SYSMENU & !WS_MAXIMIZEBOX & !WS_MINIMIZEBOX) | WS_POPUP | WS_VISIBLE;
                SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

                let current_exstyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let new_exstyle = current_exstyle | WS_EX_LAYERED | WS_EX_TOOLWINDOW;
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_exstyle);

                SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);

                let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 0, cyBottomHeight: 0 };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                let ncrendering_policy: u32 = 1;
                let dwmwa_ncrendering_policy = 2;
                let corner_preference: u32 = 1;
                let dwmwa_window_corner_preference = 33;
                let border_color: u32 = 0xFFFFFFFE;
                let dwmwa_border_color = 34;

                let _ = DwmSetWindowAttribute(hwnd, dwmwa_ncrendering_policy, &ncrendering_policy as *const u32 as *const std::ffi::c_void, 4);
                let _ = DwmSetWindowAttribute(hwnd, dwmwa_window_corner_preference, &corner_preference as *const u32 as *const std::ffi::c_void, 4);
                let _ = DwmSetWindowAttribute(hwnd, dwmwa_border_color, &border_color as *const u32 as *const std::ffi::c_void, 4);

                let null_brush = GetStockObject(NULL_BRUSH);
                SetClassLongPtrW(hwnd, GCL_HBRBACKGROUND, null_brush);

                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                let empty_title: Vec<u16> = OsStr::new("").encode_wide().chain(Some(0)).collect();
                SetWindowTextW(hwnd, empty_title.as_ptr());

                EnumChildWindows(hwnd, enum_child_proc, 0);

                let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut rect);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                if width > 0 && height > 0 {
                    let top_gutter = 38;
                    let side = (height - top_gutter).min(width);
                    let x_offset = (width - side) / 2;
                    let corner_radius = 24;
                    let h_rgn = CreateRoundRectRgn(
                        x_offset,
                        top_gutter,
                        x_offset + side + 1,
                        top_gutter + side + 1,
                        corner_radius * 2,
                        corner_radius * 2
                    );
                    SetWindowRgn(hwnd, h_rgn, 1);
                }
            }
        }
    }
    Ok(())
}

// Callback detecting and stripping artifacts from child windows (WebView2)
#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_child_proc(hwnd: isize, _lparam: isize) -> i32 {
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowLongPtrW(hwnd: isize, n_index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
        fn SetWindowTextW(hWnd: isize, lpString: *const u16) -> i32;
        fn SetWindowPos(hwnd: isize, hwnd_insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, u_flags: u32) -> i32;
        fn SetClassLongPtrW(hWnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn GetStockObject(i: i32) -> isize;
    }

    const GWL_STYLE: i32 = -16;
    const WS_CAPTION: isize = 0x00C00000;
    const WS_THICKFRAME: isize = 0x00040000;
    const WS_SYSMENU: isize = 0x00080000;
    const WS_VISIBLE: isize = 0x10000000;
    const WS_CHILD: isize = 0x40000000;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOZORDER: u32 = 0x0004;
    const GCL_HBRBACKGROUND: i32 = -10;
    const NULL_BRUSH: i32 = 5;

    let current_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    let new_style = (current_style & !WS_CAPTION & !WS_THICKFRAME & !WS_SYSMENU) | WS_CHILD | WS_VISIBLE;
    SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let empty_title: Vec<u16> = OsStr::new("").encode_wide().chain(Some(0)).collect();
    SetWindowTextW(hwnd, empty_title.as_ptr());

    let null_brush = GetStockObject(NULL_BRUSH);
    SetClassLongPtrW(hwnd, GCL_HBRBACKGROUND, null_brush);

    SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);

    1
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                #[cfg(target_os = "windows")]
                let _ = ensure_webview_borderless(&window);
            }
        }))
        .setup(|app| {
            const API_KEY_HELP_URL: &str = "https://pavlok.readme.io/reference/intro/authentication";

            let get_api_key_i = MenuItem::with_id(app, "get_api_key", "Get API Key", true, None::<&str>)?;
            let reset_fatigue_i = MenuItem::with_id(app, "reset_fatigue", "Reset Fatigue", true, None::<&str>)?;
            let reset_default_position_i = MenuItem::with_id(app, "reset_default_position", "Reset Default Position", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&get_api_key_i, &reset_fatigue_i, &reset_default_position_i, &quit_i])?;

            let tray_icon = app.default_window_icon().cloned();

            let mut tray_builder = TrayIconBuilder::new()
                .show_menu_on_left_click(false)
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    } else if event.id() == "get_api_key" {
                        let _ = app.opener().open_url(API_KEY_HELP_URL, None::<&str>);
                    } else if event.id() == "reset_fatigue" {
                        let _ = app.emit("reset-fatigue", ());
                    } else if event.id() == "reset_default_position" {
                        clear_saved_window_state(app);
                        if let Some(window) = app.get_webview_window("main") {
                            reset_window_to_default(&window);
                            #[cfg(target_os = "windows")]
                            let _ = ensure_webview_borderless(&window);
                            save_webview_window_state(&window);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                #[cfg(target_os = "windows")]
                                let _ = ensure_webview_borderless(&window);
                            }
                        }
                    }
                });

            if let Some(icon) = tray_icon {
                tray_builder = tray_builder.icon(icon);
            }

            let _tray = tray_builder.build(app)?;

            // Apply borderless fix on startup
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_min_size(Some(Size::Logical(LogicalSize::new(MIN_WIDGET_WIDTH, MIN_WIDGET_HEIGHT))));
                restore_window_state(&window);
                let _ = ensure_webview_borderless(&window);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    save_window_state(window);
                    window.app_handle().exit(0);
                }
                tauri::WindowEvent::Moved(_) => {
                    save_window_state(window);
                }
                tauri::WindowEvent::Resized(_) => {
                    save_window_state(window);
                    #[cfg(target_os = "windows")]
                    {
                        let win = window.clone();
                        tauri::async_runtime::spawn(async move {
                            std::thread::sleep(Duration::from_millis(100));
                            let _ = ensure_borderless(&win);
                        });
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![get_idle_seconds, send_pavlok_alert, start_drag, start_resize_drag])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
