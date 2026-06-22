# Win-Float

`win-float` is a lightweight workspace utility for Windows written in Rust. It allows you to toggle "Always-On-Top" states and adjust transparency for any window using global hotkeys. The app draws click-through overlays using 2D vector graphics (`tiny-skia`) for real-time visual HUD feedback.

**This is mostly a learning project to explore Windows APIs and Rust desktop utility development.**

---

## Features

### 1. Always-On-Top Toggle (`Ctrl + Win + F11`)
* Toggles the topmost state of the active foreground window.
* Draws a transparent, click-through pin overlay in the top-right corner of the target window.
* **Focused Accent Outline:** When a pinned always-on-top window is focused, an outline matching the Windows system accent color is drawn around it to indicate its focus state. The outline automatically disappears when it loses focus.

### 2. Transparency Adjustment Modal (`Shift + Win + F11`)
* Enters a dedicated block-input state targeting the active foreground window.
* **Keyboard Navigation:**
  - `Left` / `Down` / `-`: Decrease opacity (more transparent).
  - `Right` / `Up` / `+`: Increase opacity (more opaque).
  - **Any other key:** Commits the current transparency setting, exits the modal, and restores normal keyboard input.
* **Physics-Based Slider HUD:** Displays a floating HUD overlay near the window showing the current opacity percentage with a smooth, physics-animated slider bar.
* **Seeding from Current State:** Re-entering the modal automatically queries the window's existing opacity and seeds the slider, avoiding visual jumps.

### 3. Shell & System Window Exclusion
* **Safety Block:** Automatically rejects operations on system components like the Taskbar, system tray notification area, tray clock, overflow tray popup (`^`), Quick Settings panel, Action Center flyouts, volume indicators, desktop backgrounds, and third-party Start menus.
* **Hierarchical Scanning:** Climbs window owner and ancestor chains (including popups, nested browser canvas elements, and owned popups) to ensure focus outline status is correctly inherited and sub-components are not targeted.
* **Operational Logging:** Prints clear console warnings detailing the Class, Process name, and HWND of the rejected system window.

---

## Tech Stack & Windows APIs

* **Core Language:** Rust (Edition 2024)
* **Graphics Rendering:** `tiny-skia` for zero-allocation 2D vector drawings.
* **Platform APIs (`windows` crate):**
  - `GetForegroundWindow` / `SetWindowPos` (Topmost state).
  - `GetWindowLongW` / `SetWindowLongW` / `SetLayeredWindowAttributes` (Window layering and alpha opacity).
  - `SetWindowsHookExW` (`WH_KEYBOARD_LL` low-level keyboard hook) for capturing layout adjustments without swallowing system-wide keys.
  - `SetWinEventHook` (`EVENT_OBJECT_LOCATIONCHANGE`, `EVENT_OBJECT_DESTROY`, `EVENT_SYSTEM_FOREGROUND`, `EVENT_SYSTEM_MOVESIZESTART`, `EVENT_SYSTEM_MOVESIZEEND`) for location tracking, focus change handler triggers, and state cleanup.
  - `DwmGetColorizationColor` (Accent color query).
  - `SetConsoleCtrlHandler` + `PostThreadMessageW` (Graceful daemon shutdown routing).
  - `GetAncestor` / `GetWindow` (Window owner and ancestor tree climbing).
  - `OpenProcess` / `QueryFullProcessImageNameW` (Process executable name query for system window exclusion).
  - `IsZoomed` (Check if the target window is maximized).
  - `SetTimer` / `KillTimer` (Win32 window message timers for snapping animations debouncing).

---

## Architecture

Following Test-Driven Development (TDD) principles, all platform-dependent layers are fully decoupled behind abstract Rust traits. This enables comprehensive mocking and testing of the application controller without making real Win32 OS calls.

```
win-float/
├── Cargo.toml
└── src/
    ├── traits.rs           # Decoupling traits (WindowManager, InputHook, etc.) and Mocks
    ├── transparency_calc.rs # Pure mathematical functions for opacity-to-alpha mapping
    ├── hud_layout.rs       # Coordinate/boundary layout math for pins & HUD boxes
    ├── state_machine.rs    # Core transition state machine (Idle <-> Modal)
    ├── app/
    │   ├── mod.rs
    │   ├── controller.rs   # Application event controller & physics loop
    │   └── tracker.rs      # WinEvent hook tracker implementation
    ├── platform/
    │   ├── mod.rs
    │   ├── hook.rs         # Live keyboard capture hook wrapper
    │   └── window.rs       # Live window/overlay management implementation
    ├── ui/
    │   ├── mod.rs
    │   ├── draw.rs         # 2D HUD widget and pin renderer
    │   └── overlay.rs      # Overlay canvas pixels buffer wrapper
    └── main.rs             # Daemon entry point and lifecycle watchdog
```

---

## Setup & Installation

### Requirements
* Windows 10 or Windows 11.
* [Rust toolchain](https://rustup.rs/) (Edition 2024).

### Build & Run
To compile the release binary:
```powershell
cargo build --release
```

To run the utility:
```powershell
cargo run --release
```

### Running Tests
To run the full decoupled test suite (47 unit & integration tests):
```powershell
cargo test
```

---

## Reliability & Safety Design

* **Safety Hook Proc:** The low-level keyboard hook proc resolves keyboard captures using `CallNextHookEx` to ensure key event chains are never dropped system-wide.
* **Crash-Resilient Watchdog:** To ensure user windows are not left permanently transparent or locked if the app crashes, the binary launches a background watchdog process. In the event of a main process crash, the watchdog intercepts the state, recovers the target windows, and restores their original styling.
* **State Reset on Termination:** Pinned always-on-top states and transparency adjustments are automatically reverted back to their original styles and layers when the application is terminated, closed, or exited (either gracefully or recovered via the watchdog).
* **Graceful Exit:** Responds to Ctrl+C or Console Exit events by safely routing thread notifications, tearing down hook handlers, and cleaning up window style modifications immediately.

---

## License

This project is licensed under the [MIT License](./LICENSE).