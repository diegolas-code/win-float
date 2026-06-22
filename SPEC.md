# Win-Float Technical Specification

## Technical Stack & Libraries
- **Language:** Rust (edition 2024)
- **Windows API Bindings:** `windows` crate (v0.52.0)
- **2D Graphics Rendering:** `tiny-skia` (v0.11)

## Architecture & Interface Decoupling (TDD Design)
To support a strict Test-Driven Development workflow, all operating system boundaries are abstracted behind traits. This permits mock implementations to be used during unit testing of core business logic.

### Traits (`src/traits.rs`)
```rust
use windows::Win32::Foundation::HWND;

pub trait WindowManager {
    fn get_active_window(&self) -> Result<HWND, String>;
    fn set_always_on_top(&self, hwnd: HWND, enabled: bool) -> Result<(), String>;
    fn set_transparency(&self, hwnd: HWND, alpha: u8) -> Result<(), String>;
    fn is_always_on_top(&self, hwnd: HWND) -> Result<bool, String>;
    fn get_window_style_info(&self, hwnd: HWND) -> Result<(bool, u8, u32, u32, i32), String>;
    fn restore_window_style_info(
        &self,
        hwnd: HWND,
        was_layered: bool,
        alpha: u8,
        cr_key: u32,
        flags: u32,
        style: i32,
    ) -> Result<(), String>;
    fn is_taskbar_or_start_menu(&self, hwnd: HWND) -> Result<bool, String>;
    fn is_maximized(&self, hwnd: HWND) -> Result<bool, String>;
}

pub trait InputHook {
    fn capture_keyboard(&self) -> Result<(), String>;
    fn release_keyboard(&self);
}

pub trait OverlayManager {
    fn create_overlay(
        &self,
        parent: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<HWND, String>;
    fn update_overlay(
        &self,
        hwnd: HWND,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), String>;
    fn destroy_overlay(&self, hwnd: HWND);
    fn set_visibility(&self, hwnd: HWND, visible: bool);
    fn reposition_overlay(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), String>;
}

pub trait EventTracker {
    fn start_tracking(&self, target_hwnd: HWND, overlay_hwnd: HWND) -> Result<(), String>;
    fn stop_tracking(&self, target_hwnd: HWND);
    fn is_tracking(&self, target_hwnd: HWND) -> bool;
}
```

## Win32 APIs Utilized
1. **Window Management:**
   - `GetForegroundWindow`: Get active target window HWND.
   - `SetWindowPos` (with `HWND_TOPMOST`/`HWND_NOTOPMOST` & `SWP_NOMOVE | SWP_NOSIZE`): Pin/unpin windows.
   - `SetWindowLongW` / `GetWindowLongW` (extended style `GWL_EXSTYLE`): Set/query extended style flags like `WS_EX_LAYERED` for transparency support.
   - `SetLayeredWindowAttributes` / `GetLayeredWindowAttributes`: Manipulate and seed the alpha channel (0 to 255) of layered windows.
   - `IsZoomed`: Check if the window is maximized to hide the accent outline.
2. **Keyboard Hook (Modal Blocking):**
   - `SetWindowsHookExW` (with `WH_KEYBOARD_LL`): Installs low-level hook to capture and post arrow keys and `+`/`-` keys during adjustment state.
3. **Passive Window Events Tracking:**
   - `SetWinEventHook`: Registers passive event listeners.
     - `EVENT_OBJECT_LOCATIONCHANGE`: Triggers movement updates (with 150ms debouncing for snapping).
     - `EVENT_OBJECT_DESTROY`: Triggers automatic cleanup when target window is closed.
     - `EVENT_SYSTEM_FOREGROUND`: Triggers focus outline toggle transitions.
     - `EVENT_SYSTEM_MOVESIZESTART` & `EVENT_SYSTEM_MOVESIZEEND`: Hides outline during manual mouse drag loops and reveals it upon release.
4. **Rendering Overlays:**
   - Overlay windows created with styles: `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST`.
   - `tiny-skia` draws to a pixel buffer (`Pixmap`), which is loaded/rendered into the transparent overlay window surface via `UpdateLayeredWindow`.
5. **System Accent Color:**
   - Queries system accent values utilizing the `DwmGetColorizationColor` Win32 API.
6. **Window Hierarchy Resolution:**
   - `GetAncestor` (with `GA_ROOTOWNER`) and `GetWindow` (with `GW_OWNER`) to climb parent-owner chains for transient browser/editor child elements.

## Error Handling & Debouncing
- **Snapping Debounce Timer:** Win32 `SetTimer`/`KillTimer` attached to the overlay window handle debounces location updates by 150ms during snapping (e.g. Aero Snap via `Win + Arrow`), immediately hiding the overlay and restoring it only after movement has settled.
- **UAC/UIPI Restriction:** If target is Admin/Elevated, UIPI blocks modifications from user-level applications. Catch `ERROR_ACCESS_DENIED` and log/fail gracefully.
- **DPI Updates:** Listen for `WM_DPICHANGED` messages, recalculate visual sizes, and scale overlays accordingly.
