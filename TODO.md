# Win-Float Implementation TODO

Follow the TDD cycle strictly: write failing test -> verify -> write minimal implementation -> verify green -> commit -> repeat.

## Phase 1: Infrastructure & Decoupling
- [x] **Task 1: Cargo Configuration**
  - Add `tiny-skia` and `windows` with features (`Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_Graphics_Dwm`, `Win32_UI_Accessibility`, `Win32_System_LibraryLoader`) to `Cargo.toml`.
  - Verify compile with `cargo check`.
- [x] **Task 2: Abstraction Traits & Mocks**
  - Create `src/traits.rs`.
  - Write test verifying mock structures compile and record state mutations.
  - Implement `WindowManager` trait.

## Phase 2: Core Logics (TDD)
- [x] **Task 3: Transparency Math (`src/transparency_calc.rs`)**
  - Write failing unit tests for percentage-to-alpha conversions, clamping, 15% warning thresholds.
  - Implement minimal functions to pass.
  - Verify with `cargo test`.
- [x] **Task 4: Layout Mathematics (`src/hud_layout.rs`)**
  - Write failing unit tests calculating pin overlays positions (top-right corner of target RECT) and HUD centering layout.
  - Implement layout coordinate logic.
  - Verify with `cargo test`.
- [x] **Task 5: State Machine (`src/state_machine.rs`)**
  - Write failing unit tests for `Idle` <-> `TransparencyModal` transitions, and handling target window changes.
  - Implement `AppState` enum and transition methods.
  - Verify with `cargo test`.

## Phase 3: Skia UI Renderer Core
- [x] **Task 6: Pixmap Overlay Canvas (`src/ui/overlay.rs`)**
  - Write failing tests verifying canvas allocation and pixel boundaries.
  - Implement `Canvas` wrapper around `tiny-skia::Pixmap`.
- [x] **Task 7: UI Drawing Helpers (`src/ui/draw.rs`)**
  - Write tests or validation scripts for drawing a progress bar and text markers into Pixmaps.
  - Implement HUD rendering (percentage text and slider bar), pin icon, and outline border.

## Phase 4: Win32 Integrations
- [x] **Task 8: Win32 Window Manager Wrapper (`src/platform/window.rs`)**
  - Write tests asserting safety checks on null or invalid window handles.
  - Implement `LiveWindowManager` using `windows` crate wrappers for layering and topmost state adjustments.
- [x] **Task 9: Low-Level Input Hook Wrapper (`src/platform/hook.rs`)**
  - Write tests verifying keyboard hook states (hooked vs unhooked).
  - Implement keyboard capture using Win32 low-level hook (`WH_KEYBOARD_LL`).

## Phase 5: App Controller & Event Loop
- [x] **Task 10: Event Tracker (`src/app/tracker.rs`)**
  - Implement `WindowEventTracker` using `SetWinEventHook` to handle `EVENT_OBJECT_LOCATIONCHANGE` and `EVENT_OBJECT_DESTROY`.
- [x] **Task 11: Main App Loop & Controller (`src/app/controller.rs`)**
  - Integrate all components inside `AppController` running `GetMessageW` loop.
- [x] **Task 12: Application Entry (`src/main.rs`)**
  - Connect all layers, register global hotkeys, run final end-to-end tests, and packaging.
- [x] **Task 13: Console Logging Feature**
  - Print structured operational trace logs to standard output during startup, hotkey usage, pin/unpin, transparency adjustment, window events, and shutdown.

## Phase 6: Post-Release Fixes
- [x] **Task 14: Slider seeds from existing window transparency**
  - Read existing window layered style/alpha to seed the slider percentage when entering the transparency modal.
  - Verify with unit tests.
- [x] **Task 15: Always-On-Top focused window accent outline**
  - Size pin overlay window to cover the target window.
  - Listen for active/foreground window changes via `EVENT_SYSTEM_FOREGROUND` and dispatch `WM_TACTILE_FOCUS_CHANGED` events.
  - Draw system accent outline around the overlay only when the target window is focused.
  - Remove unused transparency warning threshold functions.
  - Verify with unit tests.
- [x] **Task 16: Adjust focus accent outline thickness and opacity**
  - Increase outline thickness to 3.0px (1px thicker).
  - Set outline opacity to 75% (alpha 191).
  - Add test verifying thickness and opacity of outline pixels in mock overlay manager.
  - Verify with unit tests.
- [x] **Task 17: Optimize overlay focus transition speed**
  - Avoid race condition/delay when querying `GetForegroundWindow()` by passing the new foreground window handle directly from the `EVENT_SYSTEM_FOREGROUND` event hook payload.
  - Verify with unit tests.
- [x] **Task 18: Cache window rects and focus states to avoid redundant overlays drawing**
  - Skip redraw and movement when window coordinates did not change.
  - Skip redraw when window size did not change.
  - Skip redraw when window focus state did not change.
  - Fallback to cached focus state when updating graphics without a provided handle, preventing race conditions from active window manager queries during layout adjustments.
  - Verify with unit tests.
- [x] **Task 19: Climb child window ancestor chain on focus updates**
  - Use `GetAncestor(hwnd, GA_ROOTOWNER)` to resolve top-level root window parent of focused window handle.
  - Correctly identify parent window focus when child elements (like browser canvas, editor controls, etc.) are focused.
  - Verify with unit tests.
- [x] **Task 20: Prevent overlay window from stealing focus on click/interaction**
  - Handle `WM_MOUSEACTIVATE` in `overlay_wnd_proc` and return `MA_NOACTIVATE` (3).
  - Climb owner window relationships inside `get_root_window` to correctly resolve owned popups/overlays.
  - Verify with unit tests.
- [x] **Task 21: Filter transient staging and switcher window handles during focus events**
  - Ignore OS focus-staging and task-switching window classes (`ForegroundStaging`, `XamlExplorerHostIslandWindow`, `MultitaskingViewFrame`) when tracking foreground window changes.
  - Verify with unit tests.
- [x] **Task 22: Reset always-on-top status of pinned windows on program termination**
  - Reset always-on-top status during AppController drop.
  - Send PIN/UNPIN commands to watchdog to ensure robust cleanup during abrupt crashes/terminations.
  - Verify with unit tests.
- [x] **Task 23: Exclude Taskbar and Start Menu from window targeting**
  - Implement `is_taskbar_or_start_menu` method in `WindowManager` trait and mock it in tests.
  - Implement process name and class name checks in `LiveWindowManager`.
  - Verify with mock unit tests and live Win32 unit tests.
- [x] **Task 24: Accent overlay resizing inconsistencies and 7px top offset**
  - Hide overlay during mouse movesize drag & resize loop (`EVENT_SYSTEM_MOVESIZESTART` and `EVENT_SYSTEM_MOVESIZEEND`).
  - Hide overlay completely when the target window is maximized (`is_maximized` via `IsZoomed` API).
  - Position overlay 7px taller from the top side (`y = rect.top - 7` and `height = rect.height() + 7`) to float symmetrically over Windows 10/11 invisible borders.
  - Decouple direct Win32 `SetWindowPos` calls using `reposition_overlay` method in `OverlayManager` trait.
  - Verify with unit tests (`test_always_on_top_overlay_hidden_when_maximized`, `test_window_moved_does_not_redraw_if_size_unchanged`, etc.).
- [x] **Task 25: Window snap (Win+Arrow) debouncing and overlay animations hiding**
  - Immediately hide overlay on window movement to prevent misalignment during transition animations.
  - Debounce layout repositioning and redrawing by 150ms using Win32 event timers on target window handle IDs.
  - Implement a `synchronous_window_moves` flag to support test synchronization.
  - Verify with new unit test `test_window_moved_debounces_and_updates_on_timer`.
