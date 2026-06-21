use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_SYSKEYDOWN, KBDLLHOOKSTRUCT, PostMessageW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::PCWSTR;
use crate::traits::InputHook;

pub const WM_TACTILE_KEY_EVENT: u32 = 0x8000; // WM_APP

static mut HOOK_STATE: Option<HookState> = None;

struct HookState {
    target_hwnd: HWND,
}

pub struct LiveInputHook {
    target_hwnd: HWND,
    hook: Mutex<Option<HHOOK>>,
}

impl LiveInputHook {
    pub fn new(target_hwnd: HWND) -> Self {
        Self {
            target_hwnd,
            hook: Mutex::new(None),
        }
    }

    pub fn is_hook_active(&self) -> bool {
        self.hook.lock().unwrap().is_some()
    }
}

impl InputHook for LiveInputHook {
    fn capture_keyboard(&self) -> Result<(), String> {
        let mut hook_guard = self.hook.lock().unwrap();
        if hook_guard.is_some() {
            return Ok(());
        }

        unsafe {
            HOOK_STATE = Some(HookState {
                target_hwnd: self.target_hwnd,
            });

            let hinstance = GetModuleHandleW(PCWSTR::null())
                .map_err(|e| format!("GetModuleHandleW failed: {:?}", e))?;

            let hook_handle = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                hinstance,
                0,
            ).map_err(|e| format!("SetWindowsHookExW failed: {:?}", e))?;

            *hook_guard = Some(hook_handle);
        }

        Ok(())
    }

    fn release_keyboard(&self) {
        let mut hook_guard = self.hook.lock().unwrap();
        if let Some(hook_handle) = hook_guard.take() {
            unsafe {
                let _ = UnhookWindowsHookEx(hook_handle);
                HOOK_STATE = None;
            }
        }
    }
}

impl Drop for LiveInputHook {
    fn drop(&mut self) {
        self.release_keyboard();
    }
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let state = match unsafe { &*std::ptr::addr_of!(HOOK_STATE) } {
        Some(state) => state,
        None => return unsafe { CallNextHookEx(None, code, wparam, lparam) },
    };

    if wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize {
        let kbd_struct = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk_code = kbd_struct.vkCode;

        // Post key message to our HUD window
        let _ = unsafe {
            PostMessageW(
                state.target_hwnd,
                WM_TACTILE_KEY_EVENT,
                WPARAM(vk_code as usize),
                LPARAM(0),
            )
        };

        // Consume the key to prevent background window interaction
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_lifecycle_states() {
        let hook = LiveInputHook::new(HWND(12345));
        
        // Starts inactive
        assert!(!hook.is_hook_active());

        // Capture keyboard activates hook
        hook.capture_keyboard().unwrap();
        assert!(hook.is_hook_active()); // should fail because dummy returns false

        // Release keyboard deactivates hook
        hook.release_keyboard();
        assert!(!hook.is_hook_active());
    }
}
