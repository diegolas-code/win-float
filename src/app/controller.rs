use std::collections::HashMap;
use crate::traits::{WindowManager, InputHook, OverlayManager, EventTracker};
use crate::state_machine::{StateMachine, Mode, AdjustmentAction, Transition};
use crate::platform::hook::WM_TACTILE_KEY_EVENT;
use crate::app::tracker::{WM_TACTILE_WINDOW_MOVED, WM_TACTILE_WINDOW_CLOSED};
use windows::Win32::Foundation::{HWND, RECT, BOOL};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, GetMessageW, MSG, WM_HOTKEY,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_WIN, MOD_SHIFT, VK_F11,
};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use tiny_skia::Color;

const HOTKEY_TOPMOST_ID: i32 = 1;
const HOTKEY_MODAL_ID: i32 = 2;

fn get_window_rect_helper(hwnd: HWND) -> Result<crate::hud_layout::Rect, String> {
    let mut rect = RECT::default();
    let res = unsafe { GetWindowRect(hwnd, &mut rect) };
    if res.is_err() {
        return Err("GetWindowRect failed".to_string());
    }
    Ok(crate::hud_layout::Rect::new(rect.left, rect.top, rect.right, rect.bottom))
}

fn load_system_font() -> Result<ab_glyph::FontArc, String> {
    let bytes = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf")
        .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\arial.ttf"))
        .map_err(|e| format!("Failed to read system font: {:?}", e))?;
    ab_glyph::FontArc::try_from_vec(bytes)
        .map_err(|e| format!("Failed to parse system font bytes: {:?}", e))
}

pub fn get_system_accent_color() -> Color {
    let mut colorization_color: u32 = 0;
    let mut opaque_blend: BOOL = Default::default();
    let res = unsafe {
        DwmGetColorizationColor(&mut colorization_color, &mut opaque_blend)
    };
    if res.is_ok() {
        let a = ((colorization_color >> 24) & 0xFF) as u8;
        let r = ((colorization_color >> 16) & 0xFF) as u8;
        let g = ((colorization_color >> 8) & 0xFF) as u8;
        let b = (colorization_color & 0xFF) as u8;
        Color::from_rgba8(r, g, b, a)
    } else {
        Color::from_rgba8(0, 120, 215, 255)
    }
}

pub struct AppController<W, I, O, T>
where
    W: WindowManager,
    I: InputHook,
    O: OverlayManager,
    T: EventTracker,
{
    pub window_manager: W,
    pub input_hook: I,
    pub overlay_manager: O,
    pub event_tracker: T,
    pub state_machine: StateMachine,
    pub font: ab_glyph::FontArc,
    pub pinned_overlays: HashMap<isize, HWND>, // target HWND -> pin overlay HWND
    pub hud_overlay: Option<HWND>,
    pub modal_target: Option<HWND>,
}

impl<W, I, O, T> AppController<W, I, O, T>
where
    W: WindowManager,
    I: InputHook,
    O: OverlayManager,
    T: EventTracker,
{
    pub fn new(window_manager: W, input_hook: I, overlay_manager: O, event_tracker: T) -> Result<Self, String> {
        let font = load_system_font()?;
        Ok(Self {
            window_manager,
            input_hook,
            overlay_manager,
            event_tracker,
            state_machine: StateMachine::new(),
            font,
            pinned_overlays: HashMap::new(),
            hud_overlay: None,
            modal_target: None,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        unsafe {
            let res1 = RegisterHotKey(
                HWND(0),
                HOTKEY_TOPMOST_ID,
                MOD_CONTROL | MOD_WIN,
                VK_F11.0 as u32,
            );
            if res1.is_err() {
                return Err("Failed to register topmost toggle hotkey".to_string());
            }

            let res2 = RegisterHotKey(
                HWND(0),
                HOTKEY_MODAL_ID,
                MOD_SHIFT | MOD_WIN,
                VK_F11.0 as u32,
            );
            if res2.is_err() {
                let _ = UnregisterHotKey(HWND(0), HOTKEY_TOPMOST_ID);
                return Err("Failed to register modal toggle hotkey".to_string());
            }
        }

        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam.0 as i32;
                    let _ = self.handle_hotkey(id);
                } else if msg.message == WM_TACTILE_KEY_EVENT {
                    let vk_code = msg.wParam.0 as u32;
                    let _ = self.handle_key_input(vk_code);
                } else if msg.message == WM_TACTILE_WINDOW_MOVED {
                    let target_hwnd = HWND(msg.wParam.0 as isize);
                    let _ = self.handle_window_moved(target_hwnd);
                } else if msg.message == WM_TACTILE_WINDOW_CLOSED {
                    let target_hwnd = HWND(msg.wParam.0 as isize);
                    let _ = self.handle_window_closed_event(target_hwnd);
                }

                use windows::Win32::UI::WindowsAndMessaging::{TranslateMessage, DispatchMessageW};
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        Ok(())
    }

    pub fn handle_hotkey(&mut self, id: i32) -> Result<(), String> {
        match id {
            HOTKEY_TOPMOST_ID => {
                let active = self.window_manager.get_active_window()?;
                if active.0 == 0 {
                    return Ok(());
                }

                let is_topmost = self.window_manager.is_always_on_top(active)?;

                let new_state = !is_topmost;
                self.window_manager.set_always_on_top(active, new_state)?;

                if new_state {
                    let rect = get_window_rect_helper(active).unwrap_or(crate::hud_layout::Rect::new(0, 0, 800, 600));
                    let pin_w = 24;
                    let pin_h = 24;
                    let (px, py) = crate::hud_layout::calculate_pin_position(rect, pin_w, pin_h, 10, 10);
                    let overlay = self.overlay_manager.create_overlay(px, py, pin_w, pin_h)?;

                    let mut canvas = crate::ui::overlay::Canvas::new(pin_w as u32, pin_h as u32)?;
                    let accent_color = get_system_accent_color();
                    crate::ui::draw::draw_pin(&mut canvas, accent_color)?;

                    self.overlay_manager.update_overlay(overlay, canvas.pixels(), pin_w as u32, pin_h as u32)?;
                    self.event_tracker.start_tracking(active, overlay)?;
                    self.pinned_overlays.insert(active.0, overlay);
                } else if let Some(overlay) = self.pinned_overlays.remove(&active.0) {
                    self.event_tracker.stop_tracking(active);
                    self.overlay_manager.destroy_overlay(overlay);
                }
            }
            HOTKEY_MODAL_ID => {
                if let Mode::TransparencyModal { .. } = self.state_machine.mode() {
                    let trans = self.state_machine.handle_action(AdjustmentAction::Commit);
                    self.apply_transition(trans)?;
                    return Ok(());
                }

                let active = self.window_manager.get_active_window()?;
                if active.0 == 0 {
                    return Ok(());
                }

                let current_trans = 100;
                let _ = self.state_machine.enter_modal(active, current_trans);
                self.input_hook.capture_keyboard()?;

                let rect = get_window_rect_helper(active).unwrap_or(crate::hud_layout::Rect::new(0, 0, 800, 600));
                let hud_w = 200;
                let hud_h = 80;
                let (hx, hy) = crate::hud_layout::calculate_hud_position(rect, hud_w, hud_h);
                let overlay = self.overlay_manager.create_overlay(hx, hy, hud_w, hud_h)?;

                let mut canvas = crate::ui::overlay::Canvas::new(hud_w as u32, hud_h as u32)?;
                let accent = get_system_accent_color();
                crate::ui::draw::draw_hud(&mut canvas, current_trans, &self.font, accent)?;

                self.overlay_manager.update_overlay(overlay, canvas.pixels(), hud_w as u32, hud_h as u32)?;
                self.hud_overlay = Some(overlay);
                self.modal_target = Some(active);
                self.event_tracker.start_tracking(active, overlay)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_input(&mut self, vk_code: u32) -> Result<(), String> {
        let action = match vk_code {
            0x25 | 0x28 | 0xBD | 0x6D => AdjustmentAction::Decrease,
            0x27 | 0x26 | 0xBB | 0x6B => AdjustmentAction::Increase,
            _ => AdjustmentAction::Commit,
        };

        let trans = self.state_machine.handle_action(action);
        self.apply_transition(trans)
    }

    fn apply_transition(&mut self, trans: Transition) -> Result<(), String> {
        match trans {
            Transition::Changed { target_hwnd, new_percentage } => {
                let alpha = crate::transparency_calc::percentage_to_alpha(new_percentage);
                self.window_manager.set_transparency(target_hwnd, alpha)?;

                if let Some(overlay) = self.hud_overlay {
                    let hud_w = 200;
                    let hud_h = 80;
                    let mut canvas = crate::ui::overlay::Canvas::new(hud_w, hud_h)?;
                    let accent = get_system_accent_color();
                    crate::ui::draw::draw_hud(&mut canvas, new_percentage, &self.font, accent)?;
                    self.overlay_manager.update_overlay(overlay, canvas.pixels(), hud_w, hud_h)?;
                }
            }
            Transition::Committed { target_hwnd, final_percentage: _ } => {
                self.input_hook.release_keyboard();
                if let Some(overlay) = self.hud_overlay.take() {
                    self.event_tracker.stop_tracking(target_hwnd);
                    self.overlay_manager.destroy_overlay(overlay);
                }
                self.modal_target = None;
            }
            Transition::Aborted => {
                self.input_hook.release_keyboard();
                if let Some(overlay) = self.hud_overlay.take() {
                    if let Some(target) = self.modal_target {
                        self.event_tracker.stop_tracking(target);
                    }
                    self.overlay_manager.destroy_overlay(overlay);
                }
                self.modal_target = None;
            }
            Transition::None => {}
        }
        Ok(())
    }

    pub fn handle_window_moved(&mut self, target_hwnd: HWND) -> Result<(), String> {
        let overlay_hwnd = if self.modal_target == Some(target_hwnd) {
            self.hud_overlay
        } else {
            self.pinned_overlays.get(&target_hwnd.0).copied()
        };

        if let Some(overlay) = overlay_hwnd {
            let rect = get_window_rect_helper(target_hwnd).unwrap_or(crate::hud_layout::Rect::new(0, 0, 800, 600));
            let (ox, oy, ow, oh) = if self.modal_target == Some(target_hwnd) {
                let hud_w = 200;
                let hud_h = 80;
                let (hx, hy) = crate::hud_layout::calculate_hud_position(rect, hud_w, hud_h);
                (hx, hy, hud_w, hud_h)
            } else {
                let pin_w = 24;
                let pin_h = 24;
                let (px, py) = crate::hud_layout::calculate_pin_position(rect, pin_w, pin_h, 10, 10);
                (px, py, pin_w, pin_h)
            };

            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOZORDER, SWP_NOACTIVATE};
                let _ = SetWindowPos(
                    overlay,
                    HWND(0),
                    ox,
                    oy,
                    ow,
                    oh,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
        Ok(())
    }

    pub fn handle_window_closed_event(&mut self, target_hwnd: HWND) -> Result<(), String> {
        if self.modal_target == Some(target_hwnd) {
            let trans = self.state_machine.handle_window_closed();
            self.apply_transition(trans)?;
        } else if let Some(overlay) = self.pinned_overlays.remove(&target_hwnd.0) {
            self.event_tracker.stop_tracking(target_hwnd);
            self.overlay_manager.destroy_overlay(overlay);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MockWindowManager, MockOverlayManager, MockEventTracker};

    struct MockInputHook;
    impl InputHook for MockInputHook {
        fn capture_keyboard(&self) -> Result<(), String> {
            Ok(())
        }
        fn release_keyboard(&self) {}
    }

    #[test]
    fn test_controller_initial_state() {
        let wm = MockWindowManager::default();
        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let controller = AppController::new(wm, hook, om, tracker).unwrap();
        
        assert!(controller.hud_overlay.is_none());
        assert!(controller.modal_target.is_none());
    }

    #[test]
    fn test_controller_topmost_toggle() {
        let wm = MockWindowManager::default();
        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();
        
        let target = HWND(12345);
        *controller.window_manager.active_window.lock().unwrap() = target;

        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        assert_eq!(*controller.window_manager.always_on_top.lock().unwrap(), Some((target, true)));
        assert_eq!(controller.pinned_overlays.len(), 1);
        
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        assert_eq!(controller.pinned_overlays.len(), 0);
    }

    #[test]
    fn test_controller_transparency_modal() {
        let wm = MockWindowManager::default();
        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();
        let target = HWND(12345);
        *controller.window_manager.active_window.lock().unwrap() = target;

        controller.handle_hotkey(HOTKEY_MODAL_ID).unwrap();
        assert!(controller.hud_overlay.is_some());
        assert_eq!(controller.modal_target, Some(target));

        controller.handle_key_input(0x25).unwrap(); // VK_LEFT
        assert_eq!(*controller.window_manager.transparency.lock().unwrap(), Some((target, 242)));

        controller.handle_key_input(0x0D).unwrap(); // VK_RETURN
        assert!(controller.hud_overlay.is_none());
        assert!(controller.modal_target.is_none());
    }
}

