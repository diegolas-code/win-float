
pub mod traits;
pub mod transparency_calc;
pub mod hud_layout;
pub mod state_machine;
pub mod ui;
pub mod platform;
pub mod app;

use platform::window::{LiveWindowManager, LiveOverlayManager};
use platform::hook::LiveInputHook;
use app::tracker::WindowEventTracker;
use app::controller::AppController;

use windows::core::w;
use windows::Win32::Foundation::{HWND, HINSTANCE, LRESULT, WPARAM, LPARAM, BOOL, FALSE, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    RegisterClassExW, CreateWindowExW, DefWindowProcW, WNDCLASSEXW, CS_HREDRAW, CS_VREDRAW,
    HWND_MESSAGE, WS_POPUP, HMENU, PostQuitMessage,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_C_EVENT};

unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    if ctrl_type == CTRL_C_EVENT {
        unsafe {
            PostQuitMessage(0);
        }
        return TRUE;
    }
    FALSE
}

unsafe extern "system" fn message_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn create_message_window() -> Result<HWND, String> {
    unsafe {
        let hinstance = GetModuleHandleW(None)
            .map_err(|e| format!("GetModuleHandleW failed: {:?}", e))?;

        let class_name = w!("WinFloatMessageClass");

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(message_wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: class_name,
            ..Default::default()
        };

        // Ignore error if class is already registered
        let _ = RegisterClassExW(&wnd_class);

        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("WinFloatMessageWindow"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            HMENU(0),
            HINSTANCE(hinstance.0),
            None,
        );

        if hwnd.0 == 0 {
            return Err("Failed to create message-only window".to_string());
        }

        Ok(hwnd)
    }
}

fn main() -> Result<(), String> {
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(ctrl_handler), true);
    }
    let msg_hwnd = create_message_window()?;
    let wm = LiveWindowManager;
    let hook = LiveInputHook::new(msg_hwnd);
    let om = LiveOverlayManager;
    let tracker = WindowEventTracker::new();

    let mut controller = AppController::new(wm, hook, om, tracker)?;
    controller.run()
}
