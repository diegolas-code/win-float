use std::collections::{HashMap, HashSet};
use std::time::Instant;
use crate::traits::{WindowManager, InputHook, OverlayManager, EventTracker};
use crate::state_machine::{StateMachine, Mode, AdjustmentAction, Transition};
use crate::platform::hook::WM_TACTILE_KEY_EVENT;
use crate::app::tracker::{WM_TACTILE_WINDOW_MOVED, WM_TACTILE_WINDOW_CLOSED, WM_TACTILE_FOCUS_CHANGED};
use windows::Win32::Foundation::{HWND, RECT, BOOL};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, GetMessageW, MSG, WM_HOTKEY, WM_TIMER, SetTimer, KillTimer,
    GetAncestor, GetWindow, GA_ROOTOWNER, GW_OWNER,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_WIN, MOD_SHIFT, VK_F11,
};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use tiny_skia::Color;
use std::process::ChildStdin;

const HOTKEY_TOPMOST_ID: i32 = 1;
const HOTKEY_MODAL_ID: i32 = 2;

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
static TEST_RECT: Mutex<Option<crate::hud_layout::Rect>> = Mutex::new(None);

fn get_window_rect_helper(hwnd: HWND) -> Result<crate::hud_layout::Rect, String> {
    #[cfg(test)]
    {
        if let Some(r) = *TEST_RECT.lock().unwrap() {
            return Ok(r);
        }
    }

    let mut rect = RECT::default();
    let res = unsafe { GetWindowRect(hwnd, &mut rect) };
    if res.is_err() {
        return Err("GetWindowRect failed".to_string());
    }
    Ok(crate::hud_layout::Rect::new(rect.left, rect.top, rect.right, rect.bottom))
}

#[cfg(test)]
static TEST_ROOT_ANCESTORS: Mutex<Option<HashMap<isize, isize>>> = Mutex::new(None);
#[cfg(test)]
static TEST_OWNERS: Mutex<Option<HashMap<isize, isize>>> = Mutex::new(None);

fn get_root_window(mut hwnd: HWND) -> HWND {
    #[cfg(test)]
    {
        if let Some(ref map) = *TEST_ROOT_ANCESTORS.lock().unwrap() {
            if let Some(&root) = map.get(&hwnd.0) {
                return HWND(root);
            }
        }
    }

    if hwnd.0 == 0 {
        return hwnd;
    }

    unsafe {
        loop {
            #[cfg(test)]
            {
                if let Some(ref map) = *TEST_OWNERS.lock().unwrap() {
                    if let Some(&owner) = map.get(&hwnd.0) {
                        hwnd = HWND(owner);
                        continue;
                    }
                }
            }

            let root = GetAncestor(hwnd, GA_ROOTOWNER);
            if root.0 == 0 || root == hwnd {
                let owner = GetWindow(hwnd, GW_OWNER);
                if owner.0 == 0 {
                    break;
                }
                hwnd = owner;
            } else {
                hwnd = root;
            }
        }
        hwnd
    }
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
    pub overlay_rects: HashMap<isize, crate::hud_layout::Rect>, // target HWND -> last known target rect
    pub overlay_focus_states: HashMap<isize, bool>, // target HWND -> is_focused
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
            overlay_rects: HashMap::new(),
            overlay_focus_states: HashMap::new(),
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
                } else if msg.message == WM_TACTILE_FOCUS_CHANGED {
                    let target_hwnd = HWND(msg.wParam.0 as isize);
                    let new_fg_hwnd = HWND(msg.lParam.0 as isize);
                    let _ = self.handle_focus_changed(target_hwnd, new_fg_hwnd);
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
                    let width = rect.width();
                    let height = rect.height() + 8;
                    let overlay = self.overlay_manager.create_overlay(active, rect.left, rect.top - 8, width, height)?;

                    self.update_pinned_overlay_graphics(active, overlay, rect, None)?;
                    self.event_tracker.start_tracking(active, overlay)?;
                    self.pinned_overlays.insert(active.0, overlay);
                    self.overlay_rects.insert(active.0, rect);
                    println!("[Win-Float] [Info] Pinned window HWND 0x{:X}. Created overlay HWND 0x{:X} spanning the entire window.", active.0, overlay.0);
                } else if let Some(overlay) = self.pinned_overlays.remove(&active.0) {
                    self.event_tracker.stop_tracking(active);
                    self.overlay_manager.destroy_overlay(overlay);
                    self.overlay_rects.remove(&active.0);
                    self.overlay_focus_states.remove(&active.0);
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

                // Read the window's current transparency so the slider starts at the real value.
                // get_window_style_info returns (was_layered, alpha, cr_key, flags, style).
                // If the window is already layered with LWA_ALPHA, alpha is its current opacity;
                // otherwise alpha will be 255 (fully opaque), which maps to 100%.
                let current_trans = match self.window_manager.get_window_style_info(active) {
                    Ok((was_layered, alpha, _, flags, _)) => {
                        use windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
                        // flags bit 2 (LWA_ALPHA = 0x2) indicates the alpha channel is active
                        if was_layered && (flags & LWA_ALPHA.0 != 0) {
                            crate::transparency_calc::alpha_to_percentage(alpha)
                        } else {
                            100
                        }
                    }
                    Err(_) => 100,
                };
                // Clamp to slider bounds [60, 100]
                let current_trans = current_trans.max(60);

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
                self.slider_percentage = current_trans as f32;
                self.slider_velocity = 0.0;
                self.last_physics_update = Some(Instant::now());
                self.pressed_keys.clear();
                unsafe { SetTimer(HWND(0), 1, 10, None); }
                println!("[Win-Float] [Info] Entering transparency modal for window HWND 0x{:X}. Initial transparency: {}%. Created HUD overlay HWND 0x{:X}. Captured keyboard input hook.", active.0, current_trans, overlay.0);
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
            
            let last_rect = self.overlay_rects.get(&target_hwnd.0).copied();
            
            if let Some(lr) = last_rect {
                if lr.left == rect.left && lr.top == rect.top && lr.width() == rect.width() && lr.height() == rect.height() {
                    // Nothing changed. Avoid duplicate repositioning and drawing.
                    return Ok(());
                }
            }

            let size_changed = last_rect
                .map(|lr| lr.width() != rect.width() || lr.height() != rect.height())
                .unwrap_or(true);

            self.overlay_rects.insert(target_hwnd.0, rect);

            let is_modal = self.modal_target == Some(target_hwnd);

            let (ox, oy, ow, oh) = if is_modal {
                let hud_w = 200;
                let hud_h = 80;
                let (hx, hy) = crate::hud_layout::calculate_hud_position(rect, hud_w, hud_h);
                (hx, hy, hud_w, hud_h)
            } else {
                (rect.left, rect.top - 8, rect.width(), rect.height() + 8)
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

            if !is_modal && size_changed {
                self.update_pinned_overlay_graphics(target_hwnd, overlay, rect, None)?;
            }

            println!("[Win-Float] [Info] Tracked window HWND 0x{:X} moved. Repositioning overlay HWND 0x{:X} to coordinates (x: {}, y: {}). Size changed: {}.", target_hwnd.0, overlay.0, ox, oy, size_changed);
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

    pub fn handle_focus_changed(&mut self, target_hwnd: HWND, new_fg_hwnd: HWND) -> Result<(), String> {
        if let Some(&overlay) = self.pinned_overlays.get(&target_hwnd.0) {
            let root_fg = get_root_window(new_fg_hwnd);
            let is_focused = target_hwnd == root_fg;
            
            let last_focus = self.overlay_focus_states.get(&target_hwnd.0).copied();
            if last_focus == Some(is_focused) {
                // Focus state did not change. Skip redraw.
                return Ok(());
            }

            self.overlay_focus_states.insert(target_hwnd.0, is_focused);
            let rect = get_window_rect_helper(target_hwnd).unwrap_or(crate::hud_layout::Rect::new(0, 0, 800, 600));
            self.update_pinned_overlay_graphics(target_hwnd, overlay, rect, Some(root_fg))?;
        }
        Ok(())
    }

    pub fn update_pinned_overlay_graphics(
        &mut self,
        target_hwnd: HWND,
        overlay_hwnd: HWND,
        rect: crate::hud_layout::Rect,
        new_fg_hwnd: Option<HWND>,
    ) -> Result<(), String> {
        let width = rect.width();
        let height = rect.height() + 8;
        if width <= 0 || height <= 0 {
            return Ok(());
        }

        let is_focused = if let Some(fg) = new_fg_hwnd {
            let root_fg = get_root_window(fg);
            root_fg == target_hwnd
        } else {
            self.overlay_focus_states.get(&target_hwnd.0).copied().unwrap_or_else(|| {
                self.window_manager.get_active_window()
                    .map(|act| get_root_window(act) == target_hwnd)
                    .unwrap_or(false)
            })
        };
        self.overlay_focus_states.insert(target_hwnd.0, is_focused);

        let mut canvas = crate::ui::overlay::Canvas::new(width as u32, height as u32)?;
        canvas.clear(Color::TRANSPARENT);

        let accent_color = get_system_accent_color();
        let r = (accent_color.red() * 255.0).round() as u8;
        let g = (accent_color.green() * 255.0).round() as u8;
        let b = (accent_color.blue() * 255.0).round() as u8;
        let accent_75 = Color::from_rgba8(r, g, b, 191); // 191 is 75% of 255

        // If the window has focus, draw the accent outline border with an 8px corner radius
        if is_focused {
            crate::ui::draw::draw_border(&mut canvas, accent_75, 3.0, 8.0)?;
        }

        // Draw the pin icon in the top-right corner
        let pin_w = 32;
        let pin_h = 32;
        let margin_x = 10;
        let margin_y = 10;
        let px = width - pin_w - margin_x;
        let py = margin_y;

        if px >= 0 && py >= 0 {
            let mut pin_canvas = crate::ui::overlay::Canvas::new(pin_w as u32, pin_h as u32)?;
            crate::ui::draw::draw_pin(&mut pin_canvas, accent_color)?;
            crate::ui::draw::blit_pixmap(canvas.pixmap_mut(), pin_canvas.pixmap(), px as u32, py as u32);
        }

        self.overlay_manager.update_overlay(overlay_hwnd, canvas.pixels(), width as u32, height as u32)?;
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
        
        {
            let overlays = controller.overlay_manager.overlays.lock().unwrap();
            assert_eq!(overlays.len(), 1);
            let (_, _ox, oy, _ow, oh) = overlays[0];
            assert_eq!(oy, -8, "overlay y should start at top - 8");
            assert_eq!(oh, 608, "overlay height should be rect.height() + 8");
        }
        
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

    #[test]
    fn test_modal_slider_seeds_from_existing_transparency() {
        // Simulate a window that already has 75% transparency applied
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        *wm.active_window.lock().unwrap() = target;
        // was_layered=true, alpha = percentage_to_alpha(75) = 191, flags = LWA_ALPHA (0x2)
        let alpha_75 = crate::transparency_calc::percentage_to_alpha(75);
        *wm.preset_style_info.lock().unwrap() = (true, alpha_75, 0, 0x2, 0);

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        controller.handle_hotkey(HOTKEY_MODAL_ID).unwrap();

        // slider_percentage must be seeded at 75, not 100
        assert_eq!(controller.slider_percentage as u8, 75,
            "slider should start at the window's existing transparency (75%), not 100%");
    }

    #[test]
    fn test_always_on_top_overlay_updates_outline_on_focus_change() {
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        *wm.active_window.lock().unwrap() = target;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Pin the window
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        assert_eq!(controller.pinned_overlays.len(), 1);
        let overlay_hwnd = controller.pinned_overlays.get(&target.0).copied().unwrap();

        // 1. Initially focused (since target was active) -> overlay should have higher pixel sum due to outline
        let last_sum_focused = {
            let last_update = controller.overlay_manager.last_pixel_sum.lock().unwrap();
            assert!(last_update.is_some());
            let (h, sum) = last_update.unwrap();
            assert_eq!(h, overlay_hwnd);
            sum
        };

        // 2. Pass a different focused window handle (HWND(999)) to handle_focus_changed.
        // We do NOT change active_window mock (it is still target HWND(12345)).
        // Since we pass HWND(999) as new_fg_hwnd, it must update to unfocused state directly.
        controller.handle_focus_changed(target, HWND(999)).unwrap();

        let last_sum_unfocused = {
            let last_update = controller.overlay_manager.last_pixel_sum.lock().unwrap();
            let (h, sum) = last_update.unwrap();
            assert_eq!(h, overlay_hwnd);
            sum
        };
        
        // Unfocused should have less colored pixels than focused because the outline is removed
        assert!(last_sum_focused > last_sum_unfocused, "Focused sum: {}, Unfocused sum: {}", last_sum_focused, last_sum_unfocused);
    }

    #[test]
    fn test_always_on_top_overlay_focus_outline_thickness_and_opacity() {
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        *wm.active_window.lock().unwrap() = target;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Pin the window
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        
        let pixels_opt = controller.overlay_manager.last_pixels.lock().unwrap().clone();
        assert!(pixels_opt.is_some());
        let pixels = pixels_opt.unwrap();
        
        let width = 800;
        
        // Assert alpha channel (75% opacity = 191) at a pixel fully inside the stroke
        let idx = (1 * width + 400) * 4 + 3;
        let alpha = pixels[idx];
        assert_eq!(alpha, 191, "Expected border alpha to be 191 (75% opacity), got {}", alpha);

        // Assert thickness: at y = 2, with 3.0 thickness, it should still be colored
        let idx_y2 = (2 * width + 400) * 4 + 3;
        assert!(pixels[idx_y2] > 0, "Expected pixel at y=2 to be colored for 3.0px thickness");
    }

    #[test]
    fn test_window_moved_does_not_redraw_if_size_unchanged() {
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        *wm.active_window.lock().unwrap() = target;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Preset initial mock rect (size: 200x100)
        {
            *TEST_RECT.lock().unwrap() = Some(crate::hud_layout::Rect::new(100, 100, 300, 200));
        }

        // Pin the window
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        
        {
            let overlays = controller.overlay_manager.overlays.lock().unwrap();
            assert_eq!(overlays.len(), 1);
            let (_, ox, oy, ow, oh) = overlays[0];
            assert_eq!(ox, 100);
            assert_eq!(oy, 92);
            assert_eq!(ow, 200);
            assert_eq!(oh, 108);
        }

        // Clear last_pixels tracking record
        *controller.overlay_manager.last_pixels.lock().unwrap() = None;

        // 1. Move position, keep size same
        {
            *TEST_RECT.lock().unwrap() = Some(crate::hud_layout::Rect::new(150, 180, 350, 280));
        }
        controller.handle_window_moved(target).unwrap();

        assert!(controller.overlay_manager.last_pixels.lock().unwrap().is_none(),
            "Overlay should not redraw when size is unchanged!");

        // 2. Change size
        {
            *TEST_RECT.lock().unwrap() = Some(crate::hud_layout::Rect::new(150, 180, 450, 280));
        }
        controller.handle_window_moved(target).unwrap();

        assert!(controller.overlay_manager.last_pixels.lock().unwrap().is_some(),
            "Overlay should redraw when size changes!");

        // Cleanup
        *TEST_RECT.lock().unwrap() = None;
    }

    #[test]
    fn test_focus_changed_does_not_redraw_if_focus_state_unchanged() {
        let wm = MockWindowManager::default();
        let target_a = HWND(1111);
        let target_b = HWND(2222);
        
        *wm.active_window.lock().unwrap() = target_a;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Pin target_a
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        
        // Change active to target_b and pin it
        *controller.window_manager.active_window.lock().unwrap() = target_b;
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        controller.handle_focus_changed(target_a, target_b).unwrap();

        // Clear last_pixels
        *controller.overlay_manager.last_pixels.lock().unwrap() = None;

        // Move focus to a different window (HWND(999))
        // target_a remains unfocused -> should NOT redraw
        controller.handle_focus_changed(target_a, HWND(999)).unwrap();
        
        assert!(controller.overlay_manager.last_pixels.lock().unwrap().is_none(),
            "Overlay A should not redraw when remaining unfocused");

        // target_b changes from focused to unfocused -> should redraw
        controller.handle_focus_changed(target_b, HWND(999)).unwrap();

        assert!(controller.overlay_manager.last_pixels.lock().unwrap().is_some(),
            "Overlay B should redraw when focus state changes");
    }

    #[test]
    fn test_update_pinned_overlay_graphics_uses_cached_focus_state_when_no_handle_provided() {
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        *wm.active_window.lock().unwrap() = target;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Pin the window (gets focused initially)
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        
        let overlay_hwnd = controller.pinned_overlays.get(&target.0).copied().unwrap();
        
        // Assert initial cached focus state is true (since target was active)
        assert_eq!(controller.overlay_focus_states.get(&target.0), Some(&true));

        // Change the mock active window to something else (HWND(999))
        *controller.window_manager.active_window.lock().unwrap() = HWND(999);

        // Clear last_pixels
        *controller.overlay_manager.last_pixels.lock().unwrap() = None;

        // Trigger graphics update with None (no fg hwnd provided).
        // Since focus state is cached as true, it must use the cache and draw the focus outline (meaning last_pixels sum matches focused overlay)
        let rect = crate::hud_layout::Rect::new(0, 0, 800, 600);
        controller.update_pinned_overlay_graphics(target, overlay_hwnd, rect, None).unwrap();

        let pixels = controller.overlay_manager.last_pixels.lock().unwrap().clone().unwrap();
        let width = 800;
        
        // Check that the outline border is indeed drawn (alpha at y=1 should be 191)
        let idx = (1 * width + 400) * 4 + 3;
        let alpha = pixels[idx];
        assert_eq!(alpha, 191, "Expected outline border to be drawn using cached focus state");
    }

    #[test]
    fn test_focus_changed_identifies_child_window_focus_as_focused() {
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        let child = HWND(98765);
        
        *wm.active_window.lock().unwrap() = target;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Pin target window
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        let _overlay_hwnd = controller.pinned_overlays.get(&target.0).copied().unwrap();

        // 1. Move focus to HWND(999) -> unfocused
        controller.handle_focus_changed(target, HWND(999)).unwrap();
        assert_eq!(controller.overlay_focus_states.get(&target.0), Some(&false));

        // Setup mock root ancestor: child's root is target
        {
            let mut map = HashMap::new();
            map.insert(98765, 12345);
            *TEST_ROOT_ANCESTORS.lock().unwrap() = Some(map);
        }

        // Clear last_pixels
        *controller.overlay_manager.last_pixels.lock().unwrap() = None;

        // 2. Focus moves to child window. It must resolve root to target and redraw focused outline!
        controller.handle_focus_changed(target, child).unwrap();

        // Assert focus cache became true
        assert_eq!(controller.overlay_focus_states.get(&target.0), Some(&true));
        
        // Assert B did redraw
        assert!(controller.overlay_manager.last_pixels.lock().unwrap().is_some(),
            "Overlay should redraw when child window gains focus");

        // Clean up
        *TEST_ROOT_ANCESTORS.lock().unwrap() = None;
    }

    #[test]
    fn test_focus_changed_identifies_owned_popup_focus_as_focused() {
        let wm = MockWindowManager::default();
        let target = HWND(12345);
        let popup = HWND(77777);
        
        *wm.active_window.lock().unwrap() = target;

        let hook = MockInputHook;
        let om = MockOverlayManager::default();
        let tracker = MockEventTracker::default();
        let mut controller = AppController::new(wm, hook, om, tracker).unwrap();

        // Pin target window
        controller.handle_hotkey(HOTKEY_TOPMOST_ID).unwrap();
        let _overlay_hwnd = controller.pinned_overlays.get(&target.0).copied().unwrap();

        // 1. Move focus to HWND(999) -> unfocused
        controller.handle_focus_changed(target, HWND(999)).unwrap();
        assert_eq!(controller.overlay_focus_states.get(&target.0), Some(&false));

        // Setup mock owner relationship: popup's owner is target
        {
            let mut map = HashMap::new();
            map.insert(77777, 12345);
            *TEST_OWNERS.lock().unwrap() = Some(map);
        }

        // Clear last_pixels
        *controller.overlay_manager.last_pixels.lock().unwrap() = None;

        // 2. Focus moves to popup window. It must resolve owner to target and redraw focused outline!
        controller.handle_focus_changed(target, popup).unwrap();

        // Assert focus cache became true
        assert_eq!(controller.overlay_focus_states.get(&target.0), Some(&true));
        
        // Assert B did redraw
        assert!(controller.overlay_manager.last_pixels.lock().unwrap().is_some(),
            "Overlay should redraw when owned popup window gains focus");

        // Clean up
        *TEST_OWNERS.lock().unwrap() = None;
    }
}

