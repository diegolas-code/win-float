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
}

pub trait InputHook {
    fn capture_keyboard(&self) -> Result<(), String>;
    fn release_keyboard(&self);
}
```

## Win32 APIs Utilized
1. **Window Management:**
   - `GetForegroundWindow`: Get active target window HWND.
   - `SetWindowPos` (with `HWND_TOPMOST`/`HWND_NOTOPMOST` & `SWP_NOMOVE | SWP_NOSIZE`): Pin/unpin windows.
   - `SetWindowLongW` / `GetWindowLongW` (extended style `GWL_EXSTYLE`): Set extended flags like `WS_EX_LAYERED` for transparency support.
   - `SetLayeredWindowAttributes`: Manipulate the alpha channel (0 to 255) of layered windows.
2. **Keyboard Hook (Modal Blocking):**
   - `SetWindowsHookExW` (with `WH_KEYBOARD_LL`): Installs low-level hook to capture and block arrow keys and `+`/`-` keys during adjustment state.
3. **Passive Window Tracking:**
   - `SetWinEventHook` (with `EVENT_OBJECT_LOCATIONCHANGE` and `EVENT_OBJECT_DESTROY`): Registers passive event listeners. Updates overlay position on window move/resize, and automatically cleans up if the window is closed.
4. **Rendering Overlays:**
   - Overlay windows created with styles: `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST`.
   - `tiny-skia` draws to a pixel buffer (`Pixmap`), which is loaded/rendered into the transparent overlay window surface.
5. **System Accent Color:**
   - Queries system accent values utilizing the `DwmGetColorizationColor` Win32 API.

## Error Handling & Mitigation
- **UAC/UIPI Restriction:** If target is Admin/Elevated, UIPI blocks modifications from user-level applications. Catch `ERROR_ACCESS_DENIED` and log/fail gracefully.
- **Window Closure:** If target window is closed, `EVENT_OBJECT_DESTROY` event triggers immediate cleanup of overlays.
- **DPI Updates:** Listen for `WM_DPICHANGED` messages, recalculate visual sizes, and scale fonts/slider bar overlays accordingly.
