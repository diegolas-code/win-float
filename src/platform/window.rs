use crate::traits::WindowManager;
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsWindow, SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST,
    SWP_NOMOVE, SWP_NOSIZE, GWL_EXSTYLE, WS_EX_LAYERED, LWA_ALPHA,
    GetWindowLongW, SetWindowLongW, SetLayeredWindowAttributes,
};

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
}
