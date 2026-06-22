# Commit: Hide overlay during window resize/move loops

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Registered win event hooks for `EVENT_SYSTEM_MOVESIZESTART` and `EVENT_SYSTEM_MOVESIZEEND` in `src/app/tracker.rs`.
  - [x] Dispatched `WM_TACTILE_MOVESIZE_START` and `WM_TACTILE_MOVESIZE_END` events to the active overlay handles when a move/size loop starts or ends.
  - [x] Implemented `handle_movesize_start` and `handle_movesize_end` in `AppController` to hide overlays and set tracking flags.
  - [x] Added `movesize_targets` set to prevent repositioning/redrawing overlays mid-drag.
  - [x] Serialized parallel tests in `src/app/controller.rs` using a static test mutex `TEST_SERIALIZATION_MUTEX` to prevent race conditions on shared static variables.
  - [x] Verified code is formatted, clippy warning-free, and passes all 45 tests.
- **Issues Found:**
  - When the user drag-resized or dragged to snap a window, the overlay lagged behind, causing visual jitter and heavy rendering overhead (continuous canvas updates and `UpdateLayeredWindow` invocations).
  - Test executions raced on the shared static `TEST_RECT` variable, causing intermittent failures.
- **Solutions Applied:**
  - Added visibility controls to `OverlayManager` trait and implemented it in both `MockOverlayManager` and `LiveOverlayManager` using Win32 `ShowWindow` (`SW_HIDE` / `SW_SHOWNOACTIVATE`).
  - Overlay is completely hidden when a movesize operation starts, ignoring all position/size changes. When movesize ends, the overlay is repositioned, redrawn to the final coordinates, and made visible again.
  - Added a synchronization mutex in tests to serialize tests that access/modify `TEST_RECT`.
- **Verification Proof:**
  - `cargo test` succeeded: 45 passed; 0 failed; 0 ignored.
