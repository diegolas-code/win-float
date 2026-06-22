use windows::Win32::Foundation::HWND;
use std::sync::Mutex;

pub trait WindowManager {
    fn get_active_window(&self) -> Result<HWND, String>;
    fn set_always_on_top(&self, hwnd: HWND, enabled: bool) -> Result<(), String>;
    fn set_transparency(&self, hwnd: HWND, alpha: u8) -> Result<(), String>;
    fn is_always_on_top(&self, hwnd: HWND) -> Result<bool, String>;
    fn get_window_style_info(&self, hwnd: HWND) -> Result<(bool, u8, u32, u32, i32), String>;
    fn restore_window_style_info(&self, hwnd: HWND, was_layered: bool, alpha: u8, cr_key: u32, flags: u32, style: i32) -> Result<(), String>;
    fn is_taskbar_or_start_menu(&self, hwnd: HWND) -> Result<bool, String>;
}

pub trait InputHook {
    fn capture_keyboard(&self) -> Result<(), String>;
    fn release_keyboard(&self);
}

pub trait OverlayManager {
    fn create_overlay(&self, parent: HWND, x: i32, y: i32, width: i32, height: i32) -> Result<HWND, String>;
    fn update_overlay(&self, hwnd: HWND, pixels: &[u8], width: u32, height: u32) -> Result<(), String>;
    fn destroy_overlay(&self, hwnd: HWND);
}

pub trait EventTracker {
    fn start_tracking(&self, target_hwnd: HWND, overlay_hwnd: HWND) -> Result<(), String>;
    fn stop_tracking(&self, target_hwnd: HWND);
    fn is_tracking(&self, target_hwnd: HWND) -> bool;
}

pub struct MockWindowManager {
    pub active_window: Mutex<HWND>,
    pub always_on_top: Mutex<Option<(HWND, bool)>>,
    pub transparency: Mutex<Option<(HWND, u8)>>,
    /// Pre-set style info returned by get_window_style_info.
    /// Tuple: (was_layered, alpha, cr_key, flags, style)
    pub preset_style_info: Mutex<(bool, u8, u32, u32, i32)>,
    pub taskbar_or_start_menu: Mutex<std::collections::HashSet<isize>>,
}

impl Default for MockWindowManager {
    fn default() -> Self {
        Self {
            active_window: Mutex::new(HWND(0)),
            always_on_top: Mutex::new(None),
            transparency: Mutex::new(None),
            preset_style_info: Mutex::new((false, 255, 0, 0, 0)),
            taskbar_or_start_menu: Mutex::new(std::collections::HashSet::new()),
        }
    }
}

impl WindowManager for MockWindowManager {
    fn get_active_window(&self) -> Result<HWND, String> {
        Ok(*self.active_window.lock().unwrap())
    }

    fn set_always_on_top(&self, hwnd: HWND, enabled: bool) -> Result<(), String> {
        *self.always_on_top.lock().unwrap() = Some((hwnd, enabled));
        Ok(())
    }

    fn set_transparency(&self, hwnd: HWND, alpha: u8) -> Result<(), String> {
        *self.transparency.lock().unwrap() = Some((hwnd, alpha));
        Ok(())
    }

    fn is_always_on_top(&self, hwnd: HWND) -> Result<bool, String> {
        let guard = self.always_on_top.lock().unwrap();
        if let Some((h, enabled)) = *guard {
            if h == hwnd {
                return Ok(enabled);
            }
        }
        Ok(false)
    }

    fn get_window_style_info(&self, _hwnd: HWND) -> Result<(bool, u8, u32, u32, i32), String> {
        Ok(*self.preset_style_info.lock().unwrap())
    }

    fn restore_window_style_info(&self, _hwnd: HWND, _was_layered: bool, _alpha: u8, _cr_key: u32, _flags: u32, _style: i32) -> Result<(), String> {
        Ok(())
    }

    fn is_taskbar_or_start_menu(&self, hwnd: HWND) -> Result<bool, String> {
        Ok(self.taskbar_or_start_menu.lock().unwrap().contains(&hwnd.0))
    }
}

pub struct MockOverlayManager {
    pub overlays: Mutex<Vec<(HWND, i32, i32, i32, i32)>>,
    pub last_updated: Mutex<Option<(HWND, usize)>>, // (HWND, size of pixels)
    pub last_pixel_sum: Mutex<Option<(HWND, usize)>>,
    pub last_pixels: Mutex<Option<Vec<u8>>>,
}

impl Default for MockOverlayManager {
    fn default() -> Self {
        Self {
            overlays: Mutex::new(Vec::new()),
            last_updated: Mutex::new(None),
            last_pixel_sum: Mutex::new(None),
            last_pixels: Mutex::new(None),
        }
    }
}

impl OverlayManager for MockOverlayManager {
    fn create_overlay(&self, _parent: HWND, x: i32, y: i32, width: i32, height: i32) -> Result<HWND, String> {
        let fake_hwnd = HWND(1000 + self.overlays.lock().unwrap().len() as isize);
        self.overlays.lock().unwrap().push((fake_hwnd, x, y, width, height));
        Ok(fake_hwnd)
    }

    fn update_overlay(&self, hwnd: HWND, pixels: &[u8], _width: u32, _height: u32) -> Result<(), String> {
        *self.last_updated.lock().unwrap() = Some((hwnd, pixels.len()));
        let sum: usize = pixels.iter().map(|&b| b as usize).sum();
        *self.last_pixel_sum.lock().unwrap() = Some((hwnd, sum));
        *self.last_pixels.lock().unwrap() = Some(pixels.to_vec());
        Ok(())
    }

    fn destroy_overlay(&self, hwnd: HWND) {
        self.overlays.lock().unwrap().retain(|&(h, _, _, _, _)| h != hwnd);
    }
}

pub struct MockEventTracker {
    pub tracked: Mutex<Vec<(HWND, HWND)>>,
}

impl Default for MockEventTracker {
    fn default() -> Self {
        Self {
            tracked: Mutex::new(Vec::new()),
        }
    }
}

impl EventTracker for MockEventTracker {
    fn start_tracking(&self, target_hwnd: HWND, overlay_hwnd: HWND) -> Result<(), String> {
        self.tracked.lock().unwrap().push((target_hwnd, overlay_hwnd));
        Ok(())
    }

    fn stop_tracking(&self, target_hwnd: HWND) {
        self.tracked.lock().unwrap().retain(|&(t, _)| t != target_hwnd);
    }

    fn is_tracking(&self, target_hwnd: HWND) -> bool {
        self.tracked.lock().unwrap().iter().any(|&(t, _)| t == target_hwnd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_window_manager_records_calls() {
        let mock = MockWindowManager::default();
        let hwnd = HWND(12345);
        
        mock.set_always_on_top(hwnd, true).unwrap();
        mock.set_transparency(hwnd, 128).unwrap();
        
        assert_eq!(*mock.always_on_top.lock().unwrap(), Some((hwnd, true)));
        assert_eq!(*mock.transparency.lock().unwrap(), Some((hwnd, 128)));
    }
}
