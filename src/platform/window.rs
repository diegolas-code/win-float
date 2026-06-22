use crate::traits::{WindowManager, OverlayManager};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, POINT, SIZE, HINSTANCE, LRESULT, WPARAM, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsWindow, SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST,
    SWP_NOMOVE, SWP_NOSIZE, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TOPMOST, LWA_ALPHA,
    GetWindowLongW, SetWindowLongW, SetLayeredWindowAttributes,
    CreateWindowExW, DestroyWindow, UpdateLayeredWindow, RegisterClassExW,
    DefWindowProcW, WNDCLASSEXW, CS_HREDRAW, CS_VREDRAW, WS_EX_TRANSPARENT,
    WS_EX_NOACTIVATE, WS_POPUP, ULW_ALPHA, HMENU, GetLayeredWindowAttributes,
    SWP_NOZORDER, SWP_FRAMECHANGED, SWP_NOACTIVATE, WM_MOUSEACTIVATE,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, CreateCompatibleDC, CreateDIBSection, SelectObject,
    DeleteObject, DeleteDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    AC_SRC_OVER, AC_SRC_ALPHA, BLENDFUNCTION,
};

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_MOUSEACTIVATE {
        // Return MA_NOACTIVATE (3) to prevent the overlay from stealing focus/activation
        return LRESULT(3);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

static REGISTER_CLASS: std::sync::Once = std::sync::Once::new();

fn register_overlay_class() -> Result<(), String> {
    let mut err = None;
    REGISTER_CLASS.call_once(|| {
        let hinstance = unsafe { GetModuleHandleW(None) };
        let hinstance = match hinstance {
            Ok(h) => h,
            Err(e) => {
                err = Some(format!("GetModuleHandleW failed: {:?}", e));
                return;
            }
        };
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: w!("WinFloatOverlayClass"),
            ..Default::default()
        };
        let atom = unsafe { RegisterClassExW(&wnd_class) };
        if atom == 0 {
            err = Some("RegisterClassExW failed".to_string());
        }
    });
    if let Some(e) = err {
        Err(e)
    } else {
        Ok(())
    }
}

pub struct LiveWindowManager;

impl WindowManager for LiveWindowManager {
    fn get_active_window(&self) -> Result<HWND, String> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err("No active window or invalid foreground window handle".to_string());
        }
        Ok(hwnd)
    }

    fn set_always_on_top(&self, hwnd: HWND, enabled: bool) -> Result<(), String> {
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err("Invalid window handle".to_string());
        }
        
        let target = if enabled { HWND_TOPMOST } else { HWND_NOTOPMOST };
        let res = unsafe {
            SetWindowPos(
                hwnd,
                target,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            )
        };
        if res.is_err() {
            return Err(format!("SetWindowPos failed: {:?}", res));
        }
        Ok(())
    }

    fn is_always_on_top(&self, hwnd: HWND) -> Result<bool, String> {
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err("Invalid window handle".to_string());
        }
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        Ok((ex_style & WS_EX_TOPMOST.0 as i32) != 0)
    }

    fn set_transparency(&self, hwnd: HWND, alpha: u8) -> Result<(), String> {
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err("Invalid window handle".to_string());
        }

        // Retrieve current extended styles
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        if ex_style == 0 {
            // Retrieve last error if needed, but since it could be 0 legitimately or on error,
            // we can try to proceed or verify. For simplicity we check if setting style fails.
        }
        
        // Add WS_EX_LAYERED style
        let new_style = ex_style | WS_EX_LAYERED.0 as i32;
        let res_style = unsafe { SetWindowLongW(hwnd, GWL_EXSTYLE, new_style) };
        if res_style == 0 {
            // Note: If setting style fails, we can check or report, but we'll try to set attributes anyway
        }

        // Set the layered window attributes
        let res_alpha = unsafe {
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)
        };
        if res_alpha.is_err() {
            return Err(format!("SetLayeredWindowAttributes failed: {:?}", res_alpha));
        }

        Ok(())
    }

    fn get_window_style_info(&self, hwnd: HWND) -> Result<(bool, u8, u32, u32, i32), String> {
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err("Invalid window handle".to_string());
        }
        unsafe {
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let was_layered = (style & WS_EX_LAYERED.0 as i32) != 0;
            let mut alpha = 255u8;
            let mut key = COLORREF(0);
            let mut flags = windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
            if was_layered {
                let _ = GetLayeredWindowAttributes(hwnd, Some(&mut key), Some(&mut alpha), Some(&mut flags));
            }
            Ok((was_layered, alpha, key.0, flags.0, style))
        }
    }

    fn restore_window_style_info(&self, hwnd: HWND, was_layered: bool, alpha: u8, cr_key: u32, flags: u32, style: i32) -> Result<(), String> {
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err("Invalid window handle".to_string());
        }
        unsafe {
            if was_layered {
                let _ = SetLayeredWindowAttributes(
                    hwnd,
                    COLORREF(cr_key),
                    alpha,
                    windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS(flags),
                );
                let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, style);
            } else {
                let current_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let new_style = current_style & !(WS_EX_LAYERED.0 as i32);
                let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
                let _ = SetWindowPos(
                    hwnd,
                    HWND(0),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                );
            }
        }
        Ok(())
    }

    fn is_taskbar_or_start_menu(&self, hwnd: HWND) -> Result<bool, String> {
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Ok(false);
        }

        // We check both the active window handle itself and its root owner window.
        let root_hwnd = get_root_window(hwnd);

        for target in [hwnd, root_hwnd] {
            if target.0 == 0 || unsafe { !IsWindow(target).as_bool() } {
                continue;
            }

            // Get class name
            let mut class_buf = [0u16; 256];
            let class_len = unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetClassNameW(target, &mut class_buf)
            };
            let class_name = if class_len > 0 {
                String::from_utf16_lossy(&class_buf[..class_len as usize])
            } else {
                String::new()
            };

            // Get process name
            use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, PROCESS_NAME_WIN32};
            use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
            use windows::Win32::Foundation::CloseHandle;

            let mut pid = 0u32;
            let _thread_id = unsafe { GetWindowThreadProcessId(target, Some(&mut pid)) };
            let mut filename = String::new();
            if pid != 0 {
                if let Ok(handle) = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
                    let mut path_buf = [0u16; 1024];
                    let mut size = path_buf.len() as u32;
                    let res = unsafe {
                        QueryFullProcessImageNameW(
                            handle,
                            PROCESS_NAME_WIN32,
                            windows::core::PWSTR(path_buf.as_mut_ptr()),
                            &mut size,
                        )
                    };
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                    if res.is_ok() {
                        let path = String::from_utf16_lossy(&path_buf[..size as usize]);
                        filename = std::path::Path::new(&path)
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                    }
                }
            }

            let lower_filename = filename.to_lowercase();

            // 1. Direct process match (Start Menu, Search, Action Center host components)
            if lower_filename == "startmenuexperiencehost.exe"
                || lower_filename == "searchhost.exe"
                || lower_filename == "shellexperiencehost.exe"
            {
                println!("[Win-Float] [Warning] Rejected operation on system window HWND 0x{:X} (Class: \"{}\", Process: \"{}\", Root HWND: 0x{:X}). Taskbar and Start Menu are excluded.", 
                    hwnd.0, class_name, filename, root_hwnd.0);
                return Ok(true);
            }

            // 2. Direct class name match (Taskbar, tray widgets, classic start menus, desktop)
            if class_name == "Shell_TrayWnd"
                || class_name == "Shell_SecondaryTrayWnd"
                || class_name == "TrayNotifyWnd"
                || class_name == "NotifyIconOverflowWindow"
                || class_name == "TrayClockWClass"
                || class_name == "ClockFlyoutWindow"
                || class_name == "ControlCenterWindow"
                || class_name == "Shell_LightDismissOverlay"
                || class_name == "ClassicShell.CMenuContainer"
                || class_name == "OpenShell.CMenuContainer"
                || class_name == "DV2ControlHost"
                || class_name == "XamlExplorerHostIslandWindow"
                || class_name == "Progman"
                || class_name == "WorkerW"
            {
                println!("[Win-Float] [Warning] Rejected operation on system window HWND 0x{:X} (Class: \"{}\", Process: \"{}\", Root HWND: 0x{:X}). Taskbar and Start Menu are excluded.", 
                    hwnd.0, class_name, filename, root_hwnd.0);
                return Ok(true);
            }

            // 3. explorer.exe modern UI components (Volume flyouts, Network popups, Notifications, etc.)
            if lower_filename == "explorer.exe" && (
                class_name == "Windows.UI.Core.CoreWindow"
                || class_name == "NativeHWNDHost"
            ) {
                println!("[Win-Float] [Warning] Rejected operation on system window HWND 0x{:X} (Class: \"{}\", Process: \"{}\", Root HWND: 0x{:X}). Taskbar and Start Menu are excluded.", 
                    hwnd.0, class_name, filename, root_hwnd.0);
                return Ok(true);
            }
        }

        Ok(false)
    }
}

fn get_root_window(mut hwnd: HWND) -> HWND {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetWindow, GA_ROOTOWNER, GW_OWNER};
    if hwnd.0 == 0 {
        return hwnd;
    }
    unsafe {
        loop {
            let root = GetAncestor(hwnd, GA_ROOTOWNER);
            if root.0 == 0 || root == hwnd {
                let owner = GetWindow(hwnd, GW_OWNER);
                if owner.0 == 0 {
                    break;
                }
                hwnd = owner;
            } else {
                hwnd = root;
            }
        }
        hwnd
    }
}

pub struct LiveOverlayManager;

impl OverlayManager for LiveOverlayManager {
    fn create_overlay(&self, parent: HWND, x: i32, y: i32, width: i32, height: i32) -> Result<HWND, String> {
        register_overlay_class()?;

        let hinstance = unsafe { GetModuleHandleW(None) }
            .map_err(|e| format!("GetModuleHandleW failed: {:?}", e))?;

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                w!("WinFloatOverlayClass"),
                w!("WinFloatOverlay"),
                WS_POPUP,
                x,
                y,
                width,
                height,
                parent,
                HMENU(0),
                HINSTANCE(hinstance.0),
                None,
            )
        };

        if hwnd.0 == 0 {
            return Err("CreateWindowExW returned NULL handle".to_string());
        }

        // Show window but without activating it
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }

        Ok(hwnd)
    }

    fn update_overlay(&self, hwnd: HWND, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        if hwnd.0 == 0 {
            return Err("Invalid window handle".to_string());
        }
        if pixels.len() != (width * height * 4) as usize {
            return Err("Pixel data length does not match width * height * 4".to_string());
        }

        unsafe {
            let screen_dc = GetDC(HWND(0));
            if screen_dc.0 == 0 {
                return Err("GetDC failed".to_string());
            }

            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.0 == 0 {
                ReleaseDC(HWND(0), screen_dc);
                return Err("CreateCompatibleDC failed".to_string());
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0, // BI_RGB
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: Default::default(),
            };

            let mut bits_ptr = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                windows::Win32::Foundation::HANDLE(0),
                0,
            ) {
                Ok(b) => b,
                Err(e) => {
                    let _ = DeleteDC(mem_dc);
                    ReleaseDC(HWND(0), screen_dc);
                    return Err(format!("CreateDIBSection failed: {:?}", e));
                }
            };

            if bits_ptr.is_null() {
                let _ = DeleteObject(bitmap);
                let _ = DeleteDC(mem_dc);
                ReleaseDC(HWND(0), screen_dc);
                return Err("DIB section bits pointer is null".to_string());
            }

            // Copy and swizzle RGBA to BGRA
            let total_pixels = (width * height) as usize;
            let bits_slice = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, total_pixels * 4);
            for i in 0..total_pixels {
                let src_idx = i * 4;
                bits_slice[src_idx] = pixels[src_idx + 2];     // B
                bits_slice[src_idx + 1] = pixels[src_idx + 1]; // G
                bits_slice[src_idx + 2] = pixels[src_idx];     // R
                bits_slice[src_idx + 3] = pixels[src_idx + 3]; // A
            }

            let old_bitmap = SelectObject(mem_dc, bitmap);

            let src_point = POINT { x: 0, y: 0 };
            let size = SIZE { cx: width as i32, cy: height as i32 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let res = UpdateLayeredWindow(
                hwnd,
                screen_dc,
                None,
                Some(&size),
                mem_dc,
                Some(&src_point),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            // Clean up GDI objects
            if old_bitmap.0 != 0 {
                SelectObject(mem_dc, old_bitmap);
            }
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND(0), screen_dc);

            if res.is_err() {
                return Err(format!("UpdateLayeredWindow failed: {:?}", res));
            }
        }
        Ok(())
    }

    fn destroy_overlay(&self, hwnd: HWND) {
        if hwnd.0 != 0 {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_window_manager_rejects_null_hwnd() {
        let wm = LiveWindowManager;
        
        // A null HWND should be rejected as invalid
        assert!(wm.set_always_on_top(HWND(0), true).is_err());
        assert!(wm.set_transparency(HWND(0), 128).is_err());
    }

    #[test]
    fn test_live_window_manager_rejects_fake_hwnd() {
        let wm = LiveWindowManager;
        
        // A fake HWND that does not map to a real window should be rejected
        assert!(wm.set_always_on_top(HWND(999999), true).is_err());
        assert!(wm.set_transparency(HWND(999999), 128).is_err());
    }

    #[test]
    fn test_live_window_manager_get_active_handles_errors() {
        let wm = LiveWindowManager;
        // In head-less testing environments, there might not be a valid foreground window,
        // or if there is, it must be valid. But since our dummy returns HWND(0), we want it to fail
        // when checking validity. Let's assert that the active window is verified as valid.
        let active = wm.get_active_window();
        if let Ok(hwnd) = active {
            assert_ne!(hwnd.0, 0);
        } else {
            // If it returns an error because there is no foreground window, that is also acceptable
        }
    }

    #[test]
    fn test_live_overlay_manager_lifecycle() {
        let om = LiveOverlayManager;
        let overlay = om.create_overlay(HWND(0), 10, 10, 100, 100);
        assert!(overlay.is_ok());
        let hwnd = overlay.unwrap();
        assert_ne!(hwnd.0, 0);

        let pixels = vec![255u8; 100 * 100 * 4];
        let res = om.update_overlay(hwnd, &pixels, 100, 100);
        assert!(res.is_ok());

        om.destroy_overlay(hwnd);
    }

    #[test]
    fn test_live_window_manager_is_taskbar_or_start_menu() {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, FindWindowExW};
        use windows::core::w;

        let wm = LiveWindowManager;

        // Find the real Windows Taskbar
        let taskbar_hwnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
        if taskbar_hwnd.0 != 0 {
            let res = wm.is_taskbar_or_start_menu(taskbar_hwnd);
            assert_eq!(res, Ok(true), "Shell_TrayWnd should be identified as taskbar");

            // Find a child of the taskbar (e.g. system tray, start button, clock, etc.)
            let child_hwnd = unsafe { FindWindowExW(taskbar_hwnd, HWND(0), None, None) };
            if child_hwnd.0 != 0 {
                let res_child = wm.is_taskbar_or_start_menu(child_hwnd);
                assert_eq!(res_child, Ok(true), "Child of Shell_TrayWnd should be identified as taskbar/startmenu");
            }
        }

        // Test with a dummy window handle (e.g. a real overlay)
        let om = LiveOverlayManager;
        let overlay = om.create_overlay(HWND(0), 10, 10, 100, 100).unwrap();
        let res = wm.is_taskbar_or_start_menu(overlay);
        assert_eq!(res, Ok(false), "A normal overlay window should not be identified as taskbar/startmenu");
        om.destroy_overlay(overlay);
    }
}
