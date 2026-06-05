use windows::Win32::Foundation::HWND;
use std::sync::Mutex;

pub trait WindowManager {
    fn get_active_window(&self) -> Result<HWND, String>;
    fn set_always_on_top(&self, hwnd: HWND, enabled: bool) -> Result<(), String>;
    fn set_transparency(&self, hwnd: HWND, alpha: u8) -> Result<(), String>;
}

pub trait InputHook {
    fn capture_keyboard(&self) -> Result<(), String>;
    fn release_keyboard(&self);
}

pub struct MockWindowManager {
    pub active_window: Mutex<HWND>,
    pub always_on_top: Mutex<Option<(HWND, bool)>>,
    pub transparency: Mutex<Option<(HWND, u8)>>,
}

impl Default for MockWindowManager {
    fn default() -> Self {
        Self {
            active_window: Mutex::new(HWND(0)),
            always_on_top: Mutex::new(None),
            transparency: Mutex::new(None),
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
