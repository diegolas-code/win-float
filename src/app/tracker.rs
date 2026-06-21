use std::collections::HashMap;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, PostMessageW, WINEVENT_OUTOFCONTEXT,
    OBJID_WINDOW,
};

pub const WM_TACTILE_WINDOW_MOVED: u32 = 0x8001;
pub const WM_TACTILE_WINDOW_CLOSED: u32 = 0x8002;

static TRACKED_WINDOWS: Mutex<Option<HashMap<isize, HWND>>> = Mutex::new(None);

pub struct WindowEventTracker {
    hook: Mutex<Option<HWINEVENTHOOK>>,
}

impl WindowEventTracker {
    pub fn new() -> Self {
        Self {
            hook: Mutex::new(None),
        }
    }

    pub fn start_tracking(&self, target_hwnd: HWND, overlay_hwnd: HWND) -> Result<(), String> {
        if target_hwnd.0 == 0 || overlay_hwnd.0 == 0 {
            return Err("Invalid HWND handles".to_string());
        }

        // Initialize global map
        {
            let mut map_guard = TRACKED_WINDOWS.lock().unwrap();
            if map_guard.is_none() {
                *map_guard = Some(HashMap::new());
            }
            if let Some(ref mut map) = *map_guard {
                map.insert(target_hwnd.0, overlay_hwnd);
            }
        }

        // Setup hook if not active
        let mut hook_guard = self.hook.lock().unwrap();
        if hook_guard.is_none() {
            let hook_handle = unsafe {
                SetWinEventHook(
                    EVENT_OBJECT_DESTROY,
                    EVENT_OBJECT_LOCATIONCHANGE,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                )
            };
            if hook_handle.0 == 0 {
                return Err("SetWinEventHook failed".to_string());
            }
            *hook_guard = Some(hook_handle);
        }

        Ok(())
    }

    pub fn stop_tracking(&self, target_hwnd: HWND) {
        let mut map_guard = TRACKED_WINDOWS.lock().unwrap();
        if let Some(ref mut map) = *map_guard {
            map.remove(&target_hwnd.0);
            
            if map.is_empty() {
                let mut hook_guard = self.hook.lock().unwrap();
                if let Some(hook_handle) = hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(hook_handle);
                    }
                }
            }
        }
    }

    pub fn is_tracking(&self, target_hwnd: HWND) -> bool {
        let map_guard = TRACKED_WINDOWS.lock().unwrap();
        if let Some(ref map) = *map_guard {
            map.contains_key(&target_hwnd.0)
        } else {
            false
        }
    }
}

impl Drop for WindowEventTracker {
    fn drop(&mut self) {
        let mut hook_guard = self.hook.lock().unwrap();
        if let Some(hook_handle) = hook_guard.take() {
            unsafe {
                let _ = UnhookWinEvent(hook_handle);
            }
        }
        let mut map_guard = TRACKED_WINDOWS.lock().unwrap();
        *map_guard = None;
    }
}

unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if id_object != OBJID_WINDOW.0 || id_child != 0 || hwnd.0 == 0 {
        return;
    }

    let overlay_hwnd = {
        let map_guard = match TRACKED_WINDOWS.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(ref map) = *map_guard {
            map.get(&hwnd.0).copied()
        } else {
            None
        }
    };

    if let Some(overlay) = overlay_hwnd {
        match event {
            EVENT_OBJECT_LOCATIONCHANGE => {
                let _ = unsafe {
                    PostMessageW(
                        overlay,
                        WM_TACTILE_WINDOW_MOVED,
                        WPARAM(hwnd.0 as usize),
                        LPARAM(0),
                    )
                };
            }
            EVENT_OBJECT_DESTROY => {
                let _ = unsafe {
                    PostMessageW(
                        overlay,
                        WM_TACTILE_WINDOW_CLOSED,
                        WPARAM(hwnd.0 as usize),
                        LPARAM(0),
                    )
                };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_lifecycle_states() {
        let tracker = WindowEventTracker::new();
        let target = HWND(12345);
        let overlay = HWND(67890);

        // Starts untracked
        assert!(!tracker.is_tracking(target));

        // Start tracking makes it active
        tracker.start_tracking(target, overlay).unwrap();
        assert!(tracker.is_tracking(target)); // should fail because dummy returns false

        // Stop tracking deactivates it
        tracker.stop_tracking(target);
        assert!(!tracker.is_tracking(target));
    }
}
