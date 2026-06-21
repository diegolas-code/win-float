# Commit: Cache window rects and focus states to avoid redundant overlay drawing

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Cached the target window's rect in `overlay_rects` to prevent any repositioning (`SetWindowPos`) or repainting when the coordinates do not change.
  - [x] Evaluated if size changed when repositioning, skipping costly GDI and Tiny-Skia repainting (`update_pinned_overlay_graphics`) if the width and height of the window remain identical.
  - [x] Cached focus states in `overlay_focus_states` to prevent redundant redrawing of the accent border and pin icon when focus messages are received but the focus state remains unchanged.
  - [x] Added `test_window_moved_does_not_redraw_if_size_unchanged` to verify that pure moves do not trigger redraws, while size changes do.
  - [x] Added `test_focus_changed_does_not_redraw_if_focus_state_unchanged` to verify that redundant focus updates on unfocused windows are skipped.
- **Issues Found:**
  - Standard Windows events broadcast multiple duplicate/intermediate location and focus changes, resulting in high CPU usage and visible lag as all overlays reallocated canvases and performed redraws.
- **Solutions Applied:**
  - Dual cache layer (geometry `overlay_rects` and focus `overlay_focus_states`) to short-circuit redraw operations immediately.
- **Verification Proof:**
  - Passed all 36 tests:
    ```
    running 36 tests
    test app::controller::tests::test_controller_initial_state ... ok
    test app::controller::tests::test_modal_seeds_from_existing_transparency ... ok
    ...
    test app::controller::tests::test_window_moved_does_not_redraw_if_size_unchanged ... ok
    test app::controller::tests::test_focus_changed_does_not_redraw_if_focus_state_unchanged ... ok
    ```
