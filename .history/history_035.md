# Commit: Implement Debouncing and Hiding for Snapping and Layout Moves

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Hide the overlay immediately on the first `WM_TACTILE_WINDOW_MOVED` event to avoid wrong positions during snapping animations.
  - [x] Use a Win32 event loop timer associated with the overlay window to debounce the actual overlay repositioning and redrawing by 150ms of inactivity.
  - [x] Add a configurable `synchronous_window_moves` flag to `AppController` so tests can run synchronously without relying on the message pump timers.
  - [x] Verify with new unit test `test_window_moved_debounces_and_updates_on_timer` verifying immediate hiding, old position maintenance during debounce, and correct update on timer event.
- **Issues Found:**
  - Standard Win32 Aero Snapping (e.g. `Win + Arrow keys`) does not trigger a modal movesize loop, causing immediate repositioning and redrawing. This made the outline float in the wrong place or mismatch visual transitions during the snap animation.
  - Setting a timer with a `NULL` window handle in `SetTimer(HWND(0), target_hwnd.0 as usize, ...)` causes the OS to ignore the custom timer ID and generate its own, preventing `WM_TIMER` messages from being routed back to the correct target window.
- **Solutions Applied:**
  - Associated `SetTimer`/`KillTimer` calls with the overlay window handle instead of `HWND(0)`. This preserves custom timer IDs (like `target_hwnd.0 as usize` and `1` for the physics slider).
  - Routed non-1 timer messages to `handle_debounced_window_moved` in the loop to execute the final repositioning, graphics update, and visibility restore.
- **Verification Proof:**
  - All 47 tests passed successfully.
  - `cargo clippy` and `cargo fmt` complete with no errors.
