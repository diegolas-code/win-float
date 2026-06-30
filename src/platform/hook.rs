use crate::traits::InputHook;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::core::PCWSTR;

pub const WM_TACTILE_KEY_EVENT: u32 = 0x8000; // WM_APP

static mut HOOK_STATE: Option<HookState> = None;

struct HookState {
    receiver_hwnd: HWND,
    target_hwnd: HWND,
}

pub struct LiveInputHook {
    receiver_hwnd: HWND,
    hook: Mutex<Option<HHOOK>>,
}

impl LiveInputHook {
    pub fn new(receiver_hwnd: HWND) -> Self {
        Self {
            receiver_hwnd,
            hook: Mutex::new(None),
        }
    }

    pub fn is_hook_active(&self) -> bool {
        self.hook.lock().unwrap().is_some()
    }
}

impl InputHook for LiveInputHook {
    fn capture_keyboard(&self, target_hwnd: HWND) -> Result<(), String> {
        let mut hook_guard = self.hook.lock().unwrap();
        if hook_guard.is_some() {
            return Ok(());
        }

        unsafe {
            HOOK_STATE = Some(HookState {
                receiver_hwnd: self.receiver_hwnd,
                target_hwnd,
            });

            let hinstance = GetModuleHandleW(PCWSTR::null())
                .map_err(|e| format!("GetModuleHandleW failed: {:?}", e))?;

            let hook_handle =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), hinstance, 0)
                    .map_err(|e| format!("SetWindowsHookExW failed: {:?}", e))?;

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

    let is_keydown = wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize;
    let is_keyup = wparam.0 == WM_KEYUP as usize || wparam.0 == WM_SYSKEYUP as usize;

    let mut should_swallow = false;

    if is_keydown || is_keyup {
        let kbd_struct = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk_code = kbd_struct.vkCode;
        let event_type: isize = if is_keydown { 0 } else { 1 };

        // Post key message to our HUD window via the side-channel.
        let _ = unsafe {
            PostMessageW(
                state.receiver_hwnd,
                WM_TACTILE_KEY_EVENT,
                WPARAM(vk_code as usize),
                LPARAM(event_type),
            )
        };

        // Determine if we should swallow the key event to prevent it from reaching the target window.
        // We only swallow keys if the active window is the target window, and it is not a modifier or Alt/Ctrl combination.
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        let fg_window = unsafe { GetForegroundWindow() };
        let is_target_active = fg_window == state.target_hwnd;
        let is_ctrl_down = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };

        should_swallow =
            should_swallow_key(vk_code, kbd_struct.flags.0, is_ctrl_down, is_target_active);
    }

    if should_swallow {
        LRESULT(1)
    } else {
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }
}

pub fn should_swallow_key(
    vk_code: u32,
    flags: u32,
    is_ctrl_down: bool,
    is_target_active: bool,
) -> bool {
    if !is_target_active {
        return false;
    }

    use windows::Win32::UI::WindowsAndMessaging::LLKHF_ALTDOWN;

    // Modifiers themselves: Shift (0x10, 0xA0, 0xA1), Ctrl (0x11, 0xA2, 0xA3), Alt (0x12, 0xA4, 0xA5), Win (0x5B, 0x5C)
    let is_modifier = matches!(
        vk_code,
        0x10 | 0xA0 | 0xA1 | // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
        0x11 | 0xA2 | 0xA3 | // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
        0x12 | 0xA4 | 0xA5 | // VK_MENU, VK_LMENU, VK_RMENU
        0x5B | 0x5C // VK_LWIN, VK_RWIN
    );

    let is_alt_down = (flags & LLKHF_ALTDOWN.0) != 0;

    if is_modifier || is_alt_down || is_ctrl_down {
        return false;
    }

    true
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
        hook.capture_keyboard(HWND(54321)).unwrap();
        assert!(hook.is_hook_active()); // should fail because dummy returns false

        // Release keyboard deactivates hook
        hook.release_keyboard();
        assert!(!hook.is_hook_active());
    }

    #[test]
    fn test_should_swallow_key_cases() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            VK_A, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
            VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
        };
        use windows::Win32::UI::WindowsAndMessaging::LLKHF_ALTDOWN;

        // Case 1: Target window not active -> Do NOT swallow (returns false)
        assert!(!should_swallow_key(VK_A.0 as u32, 0, false, false));

        // Case 2: Target window active, standard key (e.g. A, Arrows, Escape, Return) -> Swallow (returns true)
        assert!(should_swallow_key(VK_A.0 as u32, 0, false, true));
        assert!(should_swallow_key(VK_RIGHT.0 as u32, 0, false, true));
        assert!(should_swallow_key(VK_ESCAPE.0 as u32, 0, false, true));
        assert!(should_swallow_key(VK_RETURN.0 as u32, 0, false, true));

        // Case 3: Target window active, modifier key itself -> Do NOT swallow (returns false)
        assert!(!should_swallow_key(VK_CONTROL.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_LCONTROL.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_RCONTROL.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_MENU.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_LMENU.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_RMENU.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_LWIN.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_RWIN.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_SHIFT.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_LSHIFT.0 as u32, 0, false, true));
        assert!(!should_swallow_key(VK_RSHIFT.0 as u32, 0, false, true));

        // Case 4: Target window active, Alt is down (LLKHF_ALTDOWN set in flags) -> Do NOT swallow (returns false)
        assert!(!should_swallow_key(
            VK_A.0 as u32,
            LLKHF_ALTDOWN.0,
            false,
            true
        ));

        // Case 5: Target window active, Ctrl is down -> Do NOT swallow (returns false)
        assert!(!should_swallow_key(VK_A.0 as u32, 0, true, true));
    }
}
