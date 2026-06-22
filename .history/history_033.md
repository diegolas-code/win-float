# Commit: Hide overlay when maximized and set 7px top offset

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Implemented `is_maximized` method in `WindowManager` trait and implemented it in both `MockWindowManager` and `LiveWindowManager` (via Win32 `IsZoomed` API).
  - [x] Updated `handle_window_moved` and `handle_movesize_end` in `AppController` to check `is_maximized` and hide/show overlays accordingly.
  - [x] Sized the accent overlay to be 7px taller at the top (`y = rect.top - 7` and `height = rect.height() + 7`) to float symmetrically, matching the 7-8px invisible borders of modern Windows 10/11 windows on other sides.
  - [x] Added `test_always_on_top_overlay_hidden_when_maximized` unit test.
  - [x] Adjusted test coordinate assertions to reflect the 7px offset.
  - [x] Verified code passes all 46 tests.
- **Issues Found:**
  - When the target window is maximized, the outline borders overlap with the monitor edges, creating an unsightly double-frame visual effect.
  - Sizing the overlay with a 0-offset top led to an asymmetrical outline since modern windows have invisible resizing borders (7-8px gap) on the left, right, and bottom, but no gap at the top.
- **Solutions Applied:**
  - Completely hide the overlay whenever the window is maximized.
  - Offset the top layout boundary by 7px upward so the outline floats symmetrically around the visible client area on all four sides.
- **Verification Proof:**
  - `cargo test` succeeded: 46 passed; 0 failed; 0 ignored.
