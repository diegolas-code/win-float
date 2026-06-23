use std::collections::HashMap;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART, OBJID_WINDOW, PostMessageW,
    WINEVENT_OUTOFCONTEXT,
};

pub const WM_TACTILE_WINDOW_MOVED: u32 = 0x8001;
pub const WM_TACTILE_WINDOW_CLOSED: u32 = 0x8002;
pub const WM_TACTILE_FOCUS_CHANGED: u32 = 0x8003;
pub const WM_TACTILE_MOVESIZE_START: u32 = 0x8004;
pub const WM_TACTILE_MOVESIZE_END: u32 = 0x8005;

static TRACKED_WINDOWS: Mutex<Option<HashMap<isize, Vec<HWND>>>> = Mutex::new(None);

pub struct WindowEventTracker {
    hook: Mutex<Option<HWINEVENTHOOK>>,
    fg_hook: Mutex<Option<HWINEVENTHOOK>>,
    movesize_hook: Mutex<Option<HWINEVENTHOOK>>,
}

impl Default for WindowEventTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowEventTracker {
    pub fn new() -> Self {
        Self {
            hook: Mutex::new(None),
            fg_hook: Mutex::new(None),
            movesize_hook: Mutex::new(None),
        }
    }
}

impl crate::traits::EventTracker for WindowEventTracker {
    fn start_tracking(&self, target_hwnd: HWND, overlay_hwnd: HWND) -> Result<(), String> {
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
                let list = map.entry(target_hwnd.0).or_insert_with(Vec::new);
                if !list.contains(&overlay_hwnd) {
                    list.push(overlay_hwnd);
                }
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

        // Setup fg_hook if not active
        let mut fg_hook_guard = self.fg_hook.lock().unwrap();
        if fg_hook_guard.is_none() {
            let fg_hook_handle = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                )
            };
            if fg_hook_handle.0 == 0 {
                if let Some(h) = hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(h);
                    }
                }
                return Err("SetWinEventHook for foreground failed".to_string());
            }
            *fg_hook_guard = Some(fg_hook_handle);
        }

        // Setup movesize_hook if not active
        let mut movesize_hook_guard = self.movesize_hook.lock().unwrap();
        if movesize_hook_guard.is_none() {
            let movesize_hook_handle = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_MOVESIZESTART,
                    EVENT_SYSTEM_MOVESIZEEND,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                )
            };
            if movesize_hook_handle.0 == 0 {
                if let Some(h) = hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(h);
                    }
                }
                if let Some(h) = fg_hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(h);
                    }
                }
                return Err("SetWinEventHook for movesize failed".to_string());
            }
            *movesize_hook_guard = Some(movesize_hook_handle);
        }

        Ok(())
    }

    fn stop_tracking(&self, target_hwnd: HWND, overlay_hwnd: HWND) {
        let mut map_guard = TRACKED_WINDOWS.lock().unwrap();
        if let Some(ref mut map) = *map_guard {
            if let Some(list) = map.get_mut(&target_hwnd.0) {
                list.retain(|&h| h != overlay_hwnd);
                if list.is_empty() {
                    map.remove(&target_hwnd.0);
                }
            }

            if map.is_empty() {
                let mut hook_guard = self.hook.lock().unwrap();
                if let Some(hook_handle) = hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(hook_handle);
                    }
                }
                let mut fg_hook_guard = self.fg_hook.lock().unwrap();
                if let Some(fg_hook_handle) = fg_hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(fg_hook_handle);
                    }
                }
                let mut movesize_hook_guard = self.movesize_hook.lock().unwrap();
                if let Some(movesize_hook_handle) = movesize_hook_guard.take() {
                    unsafe {
                        let _ = UnhookWinEvent(movesize_hook_handle);
                    }
                }
            }
        }
    }

    fn is_tracking(&self, target_hwnd: HWND) -> bool {
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
        let mut fg_hook_guard = self.fg_hook.lock().unwrap();
        if let Some(fg_hook_handle) = fg_hook_guard.take() {
            unsafe {
                let _ = UnhookWinEvent(fg_hook_handle);
            }
        }
        let mut movesize_hook_guard = self.movesize_hook.lock().unwrap();
        if let Some(movesize_hook_handle) = movesize_hook_guard.take() {
            unsafe {
                let _ = UnhookWinEvent(movesize_hook_handle);
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
    if event == EVENT_SYSTEM_FOREGROUND {
        if hwnd.0 != 0 {
            println!(
                "[Win-Float] [Debug] win_event_proc EVENT_SYSTEM_FOREGROUND: hwnd = 0x{:X}",
                hwnd.0
            );
            let map_guard = match TRACKED_WINDOWS.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            if let Some(ref map) = *map_guard {
                for (&target_hwnd_val, overlays) in map {
                    if let Some(&overlay) = overlays.first() {
                        let _ = unsafe {
                            PostMessageW(
                                overlay,
                                WM_TACTILE_FOCUS_CHANGED,
                                WPARAM(target_hwnd_val as usize),
                                LPARAM(hwnd.0),
                            )
                        };
                    }
                }
            }
        }
        return;
    }

    if id_object != OBJID_WINDOW.0 || id_child != 0 || hwnd.0 == 0 {
        return;
    }

    let overlay_hwnd = {
        let map_guard = match TRACKED_WINDOWS.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(ref map) = *map_guard {
            map.get(&hwnd.0).and_then(|v| v.first().copied())
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
            EVENT_SYSTEM_MOVESIZESTART => {
                let _ = unsafe {
                    PostMessageW(
                        overlay,
                        WM_TACTILE_MOVESIZE_START,
                        WPARAM(hwnd.0 as usize),
                        LPARAM(0),
                    )
                };
            }
            EVENT_SYSTEM_MOVESIZEEND => {
                let _ = unsafe {
                    PostMessageW(
                        overlay,
                        WM_TACTILE_MOVESIZE_END,
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
    use crate::traits::EventTracker;

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
        tracker.stop_tracking(target, overlay);
        assert!(!tracker.is_tracking(target));
    }
}
