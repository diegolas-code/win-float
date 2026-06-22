
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
use windows::Win32::Foundation::{HWND, HINSTANCE, LRESULT, WPARAM, LPARAM, BOOL, FALSE, TRUE, COLORREF};
use windows::Win32::UI::WindowsAndMessaging::{
    RegisterClassExW, CreateWindowExW, DefWindowProcW, WNDCLASSEXW, CS_HREDRAW, CS_VREDRAW,
    HWND_MESSAGE, WS_POPUP, HMENU, PostThreadMessageW, WM_QUIT,
    IsWindow, GetWindowLongW, SetWindowLongW, SetLayeredWindowAttributes, GWL_EXSTYLE,
    WS_EX_LAYERED, SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, HWND_NOTOPMOST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_C_EVENT};
use windows::Win32::System::Threading::GetCurrentThreadId;
use std::io::BufRead;
use std::collections::{HashMap, HashSet};

static mut MAIN_THREAD_ID: u32 = 0;

unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    if ctrl_type == CTRL_C_EVENT {
        println!("[Win-Float] [Info] Received Ctrl+C event. Initiating graceful shutdown...");
        let thread_id = unsafe { MAIN_THREAD_ID };
        if thread_id != 0 {
            let _ = unsafe {
                PostThreadMessageW(
                    thread_id,
                    WM_QUIT,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
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

struct RestoreState {
    was_layered: bool,
    original_alpha: u8,
    original_cr_key: u32,
    original_flags: u32,
    original_style: i32,
}

fn parse_watchdog_commands<R: BufRead>(
    reader: R,
    map: &mut HashMap<isize, RestoreState>,
    pinned: &mut HashSet<isize>,
) {
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "ADD" if parts.len() >= 7 => {
                let hwnd_hex = parts[1].trim_start_matches("0x");
                if let (Ok(hwnd), Ok(was_layered), Ok(alpha), Ok(cr_key), Ok(flags), Ok(style)) = (
                    isize::from_str_radix(hwnd_hex, 16),
                    parts[2].parse::<bool>(),
                    parts[3].parse::<u8>(),
                    parts[4].parse::<u32>(),
                    parts[5].parse::<u32>(),
                    parts[6].parse::<i32>(),
                ) {
                    map.insert(hwnd, RestoreState {
                        was_layered,
                        original_alpha: alpha,
                        original_cr_key: cr_key,
                        original_flags: flags,
                        original_style: style,
                    });
                }
            }
            "REMOVE" if parts.len() >= 2 => {
                let hwnd_hex = parts[1].trim_start_matches("0x");
                if let Ok(hwnd) = isize::from_str_radix(hwnd_hex, 16) {
                    map.remove(&hwnd);
                    pinned.remove(&hwnd);
                }
            }
            "PIN" if parts.len() >= 2 => {
                let hwnd_hex = parts[1].trim_start_matches("0x");
                if let Ok(hwnd) = isize::from_str_radix(hwnd_hex, 16) {
                    pinned.insert(hwnd);
                }
            }
            "UNPIN" if parts.len() >= 2 => {
                let hwnd_hex = parts[1].trim_start_matches("0x");
                if let Ok(hwnd) = isize::from_str_radix(hwnd_hex, 16) {
                    pinned.remove(&hwnd);
                }
            }
            _ => {}
        }
    }
}

fn run_watchdog(parent_pid: u32) {
    println!("[Win-Float] [Info] Watchdog started for parent PID {}.", parent_pid);
    let stdin = std::io::stdin();
    let mut map = HashMap::<isize, RestoreState>::new();
    let mut pinned = HashSet::<isize>::new();

    parse_watchdog_commands(stdin.lock(), &mut map, &mut pinned);

    println!("[Win-Float] [Info] Watchdog restoring transparency styles on EOF.");
    for (hwnd_val, state) in map {
        let hwnd = HWND(hwnd_val);
        unsafe {
            if IsWindow(hwnd).as_bool() {
                if state.was_layered {
                    let _ = SetLayeredWindowAttributes(
                        hwnd,
                        COLORREF(state.original_cr_key),
                        state.original_alpha,
                        windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS(state.original_flags),
                    );
                    let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, state.original_style);
                } else {
                    let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                    let new_style = style & !(WS_EX_LAYERED.0 as i32);
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
        }
    }

    println!("[Win-Float] [Info] Watchdog restoring always-on-top states on EOF.");
    for hwnd_val in pinned {
        let hwnd = HWND(hwnd_val);
        unsafe {
            if IsWindow(hwnd).as_bool() {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_NOTOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
            }
        }
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--watchdog" {
        if let Ok(parent_pid) = args[2].parse::<u32>() {
            run_watchdog(parent_pid);
        }
        return Ok(());
    }

    println!("[Win-Float] [Info] Win-Float daemon started.");
    unsafe {
        MAIN_THREAD_ID = GetCurrentThreadId();
        let _ = SetConsoleCtrlHandler(Some(ctrl_handler), true);
    }
    let msg_hwnd = create_message_window()?;
    let wm = LiveWindowManager;
    let hook = LiveInputHook::new(msg_hwnd);
    let om = LiveOverlayManager;
    let tracker = WindowEventTracker::new();

    println!("[Win-Float] [Info] Entering message loop. Registering global hotkeys...");
    let mut controller = AppController::new(wm, hook, om, tracker)?;
    let res = controller.run();
    println!("[Win-Float] [Info] Message loop exited. Cleaning up resource handles.");
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_parser_pin_unpin() {
        let input = "PIN 0x1A2B\nADD 0x3C4D false 255 0 0 1234\nPIN 0x5E6F\nUNPIN 0x1A2B\nREMOVE 0x3C4D\n";
        let mut map = HashMap::new();
        let mut pinned = HashSet::new();
        parse_watchdog_commands(input.as_bytes(), &mut map, &mut pinned);

        // PIN 0x1A2B, then UNPIN 0x1A2B -> pinned should not contain 0x1A2B.
        assert!(!pinned.contains(&0x1A2B));

        // ADD 0x3C4D, then REMOVE 0x3C4D -> map should not contain 0x3C4D.
        assert!(!map.contains_key(&0x3C4D));

        // PIN 0x5E6F -> pinned should contain 0x5E6F.
        assert!(pinned.contains(&0x5E6F));
    }
}
