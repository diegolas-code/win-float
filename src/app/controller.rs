use std::collections::{HashMap, HashSet};
use std::time::Instant;
use crate::traits::{WindowManager, InputHook, OverlayManager, EventTracker};
use crate::state_machine::{StateMachine, Mode, AdjustmentAction, Transition};
use crate::platform::hook::WM_TACTILE_KEY_EVENT;
use crate::app::tracker::{WM_TACTILE_WINDOW_MOVED, WM_TACTILE_WINDOW_CLOSED};
use windows::Win32::Foundation::{HWND, RECT, BOOL};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, GetMessageW, MSG, WM_HOTKEY, WM_TIMER, SetTimer, KillTimer,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_WIN, MOD_SHIFT, VK_F11,
};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use tiny_skia::Color;
use std::process::ChildStdin;

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
    pub watchdog_stdin: Option<ChildStdin>,
    pub original_window_states: HashMap<isize, (bool, u8, u32, u32, i32)>, // target HWND -> (was_layered, alpha, key, flags, style)
    pub pressed_keys: HashSet<u32>,
    pub slider_velocity: f32,
    pub slider_percentage: f32,
    pub last_physics_update: Option<Instant>,
}

impl<W, I, O, T> AppController<W, I, O, T>
where
    W: WindowManager,
    I: InputHook,
    O: OverlayManager,
    T: EventTracker,
{
    pub fn new(window_manager: W, input_hook: I, overlay_manager: O, event_tracker: T) -> Result<Self, String> {
        let watchdog_stdin = if cfg!(test) {
            None
        } else {
            match std::process::Command::new(std::env::current_exe().unwrap_or_default())
                .arg("--watchdog")
                .arg(std::process::id().to_string())
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                Ok(mut child) => child.stdin.take(),
                Err(_) => None,
            }
        };

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
            watchdog_stdin,
            original_window_states: HashMap::new(),
            pressed_keys: HashSet::new(),
            slider_velocity: 0.0,
            slider_percentage: 100.0,
            last_physics_update: None,
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
        println!("[Win-Float] [Info] Hotkeys registered: Ctrl+Win+F11 (Toggle Pin), Shift+Win+F11 (Transparency Mode).");

        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam.0 as i32;
                    let _ = self.handle_hotkey(id);
                } else if msg.message == WM_TACTILE_KEY_EVENT {
                    let vk_code = msg.wParam.0 as u32;
                    let event_type = msg.lParam.0 as i32;
                    let _ = self.handle_key_input(vk_code, event_type);
                } else if msg.message == WM_TIMER {
                    let _ = self.handle_timer_tick();
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
                println!("[Win-Float] [Info] Hotkey triggered: Toggle Pin");
                let active = self.window_manager.get_active_window()?;
                if active.0 == 0 {
                    return Ok(());
                }

                let is_topmost = self.window_manager.is_always_on_top(active)?;

                let new_state = !is_topmost;
                self.window_manager.set_always_on_top(active, new_state)?;

                if new_state {
                    let rect = get_window_rect_helper(active).unwrap_or(crate::hud_layout::Rect::new(0, 0, 800, 600));
                    let pin_w = 32;
                    let pin_h = 32;
                    let (px, py) = crate::hud_layout::calculate_pin_position(rect, pin_w, pin_h, 10, 10);
                    let overlay = self.overlay_manager.create_overlay(active, px, py, pin_w, pin_h)?;

                    let mut canvas = crate::ui::overlay::Canvas::new(pin_w as u32, pin_h as u32)?;
                    let accent_color = get_system_accent_color();
                    crate::ui::draw::draw_pin(&mut canvas, accent_color)?;

                    self.overlay_manager.update_overlay(overlay, canvas.pixels(), pin_w as u32, pin_h as u32)?;
                    self.event_tracker.start_tracking(active, overlay)?;
                    self.pinned_overlays.insert(active.0, overlay);
                    println!("[Win-Float] [Info] Pinned window HWND 0x{:X}. Created overlay HWND 0x{:X} (32x32 bee icon).", active.0, overlay.0);
                } else if let Some(overlay) = self.pinned_overlays.remove(&active.0) {
                    self.event_tracker.stop_tracking(active);
                    self.overlay_manager.destroy_overlay(overlay);
                    println!("[Win-Float] [Info] Unpinned window HWND 0x{:X}. Destroyed overlay HWND 0x{:X}.", active.0, overlay.0);
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
                let overlay = self.overlay_manager.create_overlay(active, hx, hy, hud_w, hud_h)?;

                let mut canvas = crate::ui::overlay::Canvas::new(hud_w as u32, hud_h as u32)?;
                let accent = get_system_accent_color();
                crate::ui::draw::draw_hud(&mut canvas, current_trans, &self.font, accent)?;

                self.overlay_manager.update_overlay(overlay, canvas.pixels(), hud_w as u32, hud_h as u32)?;
                self.hud_overlay = Some(overlay);
                self.modal_target = Some(active);
                self.event_tracker.start_tracking(active, overlay)?;
                self.slider_percentage = 100.0;
                self.slider_velocity = 0.0;
                self.last_physics_update = Some(Instant::now());
                self.pressed_keys.clear();
                unsafe { SetTimer(HWND(0), 1, 10, None); }
                println!("[Win-Float] [Info] Entering transparency modal for window HWND 0x{:X}. Created HUD overlay HWND 0x{:X}. Captured keyboard input hook.", active.0, overlay.0);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_input(&mut self, vk_code: u32, event_type: i32) -> Result<(), String> {
        let is_direction_key = matches!(vk_code, 0x25 | 0x28 | 0xBD | 0x6D | 0x27 | 0x26 | 0xBB | 0x6B);

        if is_direction_key {
            if event_type == 0 {
                self.pressed_keys.insert(vk_code);
            } else {
                self.pressed_keys.remove(&vk_code);
            }
            return Ok(());
        }

        // Non-direction key (e.g. Enter, Escape) -> commit
        if event_type == 0 {
            self.pressed_keys.clear();
            unsafe { let _ = KillTimer(HWND(0), 1); }
            self.last_physics_update = None;
            self.slider_velocity = 0.0;
            let trans = self.state_machine.handle_action(AdjustmentAction::Commit);
            self.apply_transition(trans)?;
        }
        Ok(())
    }

    fn handle_timer_tick(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let dt = if let Some(last) = self.last_physics_update {
            now.duration_since(last).as_secs_f32()
        } else {
            0.01
        };
        self.last_physics_update = Some(now);

        // Clamp dt to avoid physics explosion on lag spikes
        let dt = dt.min(0.05);

        // Calculate thrust direction from pressed keys
        let decrease_keys: &[u32] = &[0x25, 0x28, 0xBD, 0x6D]; // Left, Down, -, NumpadSubtract
        let increase_keys: &[u32] = &[0x27, 0x26, 0xBB, 0x6B]; // Right, Up, +, NumpadAdd
        let thrust_neg = self.pressed_keys.iter().any(|k| decrease_keys.contains(k));
        let thrust_pos = self.pressed_keys.iter().any(|k| increase_keys.contains(k));
        let thrust: f32 = match (thrust_neg, thrust_pos) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };
        let is_thrusting = thrust != 0.0;

        // Apply acceleration
        let acceleration = 120.0;
        let max_speed = 80.0;
        self.slider_velocity += thrust * acceleration * dt;
        self.slider_velocity = self.slider_velocity.clamp(-max_speed, max_speed);

        // Apply dual friction (low while thrusting, high when coasting)
        let friction = if is_thrusting { 0.5 } else { 8.0 };
        self.slider_velocity *= (-friction * dt).exp();

        // Threshold to zero
        if self.slider_velocity.abs() < 0.1 {
            self.slider_velocity = 0.0;
        }

        // Update percentage
        let old_pct = self.slider_percentage as u8;
        self.slider_percentage += self.slider_velocity * dt;
        if self.slider_percentage <= 60.0 {
            self.slider_percentage = 60.0;
            self.slider_velocity = 0.0;
        }
        if self.slider_percentage >= 100.0 {
            self.slider_percentage = 100.0;
            self.slider_velocity = 0.0;
        }
        let new_pct = self.slider_percentage as u8;

        // Only update when rounded percentage changes
        if new_pct != old_pct {
            if let Mode::TransparencyModal { target_hwnd, .. } = self.state_machine.mode() {
                let alpha = crate::transparency_calc::percentage_to_alpha(new_pct);

                if !self.original_window_states.contains_key(&target_hwnd.0) {
                    if let Ok(info) = self.window_manager.get_window_style_info(target_hwnd) {
                        self.original_window_states.insert(target_hwnd.0, info);
                        if let Some(ref mut stdin) = self.watchdog_stdin {
                            use std::io::Write;
                            let _ = writeln!(stdin, "ADD 0x{:X} {} {} {} {} {}", target_hwnd.0, info.0, info.1, info.2, info.3, info.4);
                            let _ = stdin.flush();
                        }
                    }
                }

                self.window_manager.set_transparency(target_hwnd, alpha)?;
                println!("[Win-Float] [Info] Transparency level adjusted to {}% (Alpha: {}).", new_pct, alpha);

                // Update the state machine's internal percentage
                self.state_machine.set_percentage(new_pct);

                if let Some(overlay) = self.hud_overlay {
                    let hud_w = 200;
                    let hud_h = 80;
                    let mut canvas = crate::ui::overlay::Canvas::new(hud_w, hud_h)?;
                    let accent = get_system_accent_color();
                    crate::ui::draw::draw_hud(&mut canvas, new_pct, &self.font, accent)?;
                    self.overlay_manager.update_overlay(overlay, canvas.pixels(), hud_w, hud_h)?;
                }
            }
        }

        Ok(())
    }

    fn apply_transition(&mut self, trans: Transition) -> Result<(), String> {
        match trans {
            Transition::Changed { target_hwnd, new_percentage } => {
                let alpha = crate::transparency_calc::percentage_to_alpha(new_percentage);

                if !self.original_window_states.contains_key(&target_hwnd.0) {
                    if let Ok(info) = self.window_manager.get_window_style_info(target_hwnd) {
                        self.original_window_states.insert(target_hwnd.0, info);
                        if let Some(ref mut stdin) = self.watchdog_stdin {
                            use std::io::Write;
                            let _ = writeln!(stdin, "ADD 0x{:X} {} {} {} {} {}", target_hwnd.0, info.0, info.1, info.2, info.3, info.4);
                            let _ = stdin.flush();
                        }
                    }
                }

                self.window_manager.set_transparency(target_hwnd, alpha)?;
                println!("[Win-Float] [Info] Transparency level adjusted to {}% (Alpha: {}).", new_percentage, alpha);

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
                unsafe { let _ = KillTimer(HWND(0), 1); }
                self.pressed_keys.clear();
                self.slider_velocity = 0.0;
                self.last_physics_update = None;
                if let Some(overlay) = self.hud_overlay.take() {
                    self.event_tracker.stop_tracking(target_hwnd);
                    self.overlay_manager.destroy_overlay(overlay);
                }
                self.modal_target = None;
                println!("[Win-Float] [Info] Committing transparency changes for window HWND 0x{:X}. Released keyboard input hook. Destroyed HUD overlay.", target_hwnd.0);
            }
            Transition::Aborted => {
                self.input_hook.release_keyboard();
                unsafe { let _ = KillTimer(HWND(0), 1); }
                self.pressed_keys.clear();
                self.slider_velocity = 0.0;
                self.last_physics_update = None;
                if let Some(target) = self.modal_target {
                    if let Some(&(was_layered, original_alpha, original_cr_key, original_flags, original_style)) = self.original_window_states.get(&target.0) {
                        let _ = self.window_manager.restore_window_style_info(
                            target,
                            was_layered,
                            original_alpha,
                            original_cr_key,
                            original_flags,
                            original_style,
                        );
                    }
                    self.original_window_states.remove(&target.0);
                    if let Some(ref mut stdin) = self.watchdog_stdin {
                        use std::io::Write;
                        let _ = writeln!(stdin, "REMOVE 0x{:X}", target.0);
                        let _ = stdin.flush();
                    }
                }
                if let Some(overlay) = self.hud_overlay.take() {
                    if let Some(target) = self.modal_target {
                        self.event_tracker.stop_tracking(target);
                    }
                    self.overlay_manager.destroy_overlay(overlay);
                }
                self.modal_target = None;
                println!("[Win-Float] [Info] Aborting transparency changes. Reverting adjustments. Released keyboard input hook. Destroyed HUD overlay.");
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
                let pin_w = 32;
                let pin_h = 32;
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
            println!("[Win-Float] [Info] Tracked window HWND 0x{:X} moved. Repositioning overlay HWND 0x{:X} to coordinates (x: {}, y: {}).", target_hwnd.0, overlay.0, ox, oy);
        }
        Ok(())
    }

    pub fn handle_window_closed_event(&mut self, target_hwnd: HWND) -> Result<(), String> {
        self.original_window_states.remove(&target_hwnd.0);
        if let Some(ref mut stdin) = self.watchdog_stdin {
            use std::io::Write;
            let _ = writeln!(stdin, "REMOVE 0x{:X}", target_hwnd.0);
            let _ = stdin.flush();
        }

        if self.modal_target == Some(target_hwnd) {
            println!("[Win-Float] [Info] Tracked window HWND 0x{:X} closed while in transparency modal.", target_hwnd.0);
            let trans = self.state_machine.handle_window_closed();
            self.apply_transition(trans)?;
        } else if let Some(overlay) = self.pinned_overlays.remove(&target_hwnd.0) {
            println!("[Win-Float] [Info] Tracked window HWND 0x{:X} closed. Stopping track and cleaning up overlay HWND 0x{:X}.", target_hwnd.0, overlay.0);
            self.event_tracker.stop_tracking(target_hwnd);
            self.overlay_manager.destroy_overlay(overlay);
        }
        Ok(())
    }
}

impl<W, I, O, T> Drop for AppController<W, I, O, T>
where
    W: WindowManager,
    I: InputHook,
    O: OverlayManager,
    T: EventTracker,
{
    fn drop(&mut self) {
        // Restore all window styles/alpha gracefully
        for (&hwnd_val, &(was_layered, original_alpha, original_cr_key, original_flags, original_style)) in &self.original_window_states {
            let hwnd = HWND(hwnd_val);
            let _ = self.window_manager.restore_window_style_info(
                hwnd,
                was_layered,
                original_alpha,
                original_cr_key,
                original_flags,
                original_style,
            );
        }
        
        // Explicitly close stdin to notify watchdog we're exiting gracefully
        self.watchdog_stdin.take();
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

        // Press left key (key-down event_type=0)
        controller.handle_key_input(0x25, 0).unwrap(); // VK_LEFT down
        assert!(controller.pressed_keys.contains(&0x25));

        // Simulate physics tick - should apply thrust and change percentage
        controller.slider_percentage = 80.0;
        controller.slider_velocity = -10.0;
        controller.last_physics_update = Some(std::time::Instant::now());
        // Manually call timer tick
        controller.handle_timer_tick().unwrap();

        // Release left key (key-up event_type=1)
        controller.handle_key_input(0x25, 1).unwrap(); // VK_LEFT up
        assert!(!controller.pressed_keys.contains(&0x25));

        // Commit via Enter key (key-down event_type=0)
        controller.handle_key_input(0x0D, 0).unwrap(); // VK_RETURN
        assert!(controller.hud_overlay.is_none());
        assert!(controller.modal_target.is_none());
    }

    #[test]
    fn test_slider_physics_acceleration() {
        let wm = MockWindowManager::default();
        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();
        let target = HWND(12345);
        *controller.window_manager.active_window.lock().unwrap() = target;

        controller.handle_hotkey(HOTKEY_MODAL_ID).unwrap();

        // Set up initial state at 80%
        controller.slider_percentage = 80.0;
        controller.slider_velocity = 0.0;

        // Simulate pressing left key (decrease direction)
        controller.pressed_keys.insert(0x25); // VK_LEFT

        // Simulate a physics tick with dt = 0.1s
        controller.last_physics_update = Some(std::time::Instant::now() - std::time::Duration::from_millis(100));
        controller.handle_timer_tick().unwrap();

        // Velocity should be negative (moving towards lower percentage)
        assert!(controller.slider_velocity < 0.0);
        // Percentage should have decreased
        assert!(controller.slider_percentage < 80.0);
    }

    #[test]
    fn test_slider_physics_friction_stops() {
        let wm = MockWindowManager::default();
        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();
        let target = HWND(12345);
        *controller.window_manager.active_window.lock().unwrap() = target;

        controller.handle_hotkey(HOTKEY_MODAL_ID).unwrap();

        // Start with some velocity but no keys pressed (coasting)
        controller.slider_percentage = 80.0;
        controller.slider_velocity = 5.0;
        controller.pressed_keys.clear(); // no thrust

        // Simulate a physics tick with dt = 0.5s (long enough for high friction to decay)
        controller.last_physics_update = Some(std::time::Instant::now() - std::time::Duration::from_millis(500));
        controller.handle_timer_tick().unwrap();

        // High friction should have decayed velocity significantly
        assert!(controller.slider_velocity.abs() < 5.0);
    }
}

