# Commit: Correct Pinned Overlay Position Offset in handle_window_moved

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Apply the 7px top offset (`rect.top - 7` and `rect.height() + 7`) during coordinate updates inside `handle_window_moved`.
  - [x] Decouple direct Win32 `SetWindowPos` calls by adding `reposition_overlay` to the `OverlayManager` trait and implementing it for `LiveOverlayManager` and `MockOverlayManager`.
  - [x] Verify that the test `test_window_moved_does_not_redraw_if_size_unchanged` now updates mock coordinates and passes.
- **Issues Found:**
  - The pinned accent overlay was 7px taller but reverted to 0-offset coordinates when updated via `handle_window_moved` event.
  - The test `test_window_moved_does_not_redraw_if_size_unchanged` asserted mock coordinate updates but they remained at the initial coordinates because `SetWindowPos` was called directly as a Win32 function and not intercepted or mockable.
- **Solutions Applied:**
  - Added `reposition_overlay` to `OverlayManager` and updated it in the mock manager to correctly track coordinate shifts.
  - Replaced the direct Win32 `SetWindowPos` calls with `self.overlay_manager.reposition_overlay(overlay, ox, oy, ow, oh)`.
  - Applied the offset coordinates `(rect.left, rect.top - 7, rect.width(), rect.height() + 7)` in `handle_window_moved` for the non-modal case.
- **Verification Proof:**
  - All 46 tests now pass successfully (including `test_window_moved_does_not_redraw_if_size_unchanged`).
  - `cargo fmt` and `cargo clippy` run successfully with zero warnings/errors.
