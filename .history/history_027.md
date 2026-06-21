# Commit: Prevent overlay focus stealing and resolve owned popup parent relationships

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Intercepted `WM_MOUSEACTIVATE` in `overlay_wnd_proc` inside `src/platform/window.rs` and returned `MA_NOACTIVATE` (3) to prevent overlay click interactions from stealing focus from target windows.
  - [x] Implemented manual owner chain walking using `GetWindow(hwnd, GW_OWNER)` inside `get_root_window` to ensure popup and owned windows resolve to their root owner parent.
  - [x] Added `test_focus_changed_identifies_owned_popup_focus_as_focused` verifying that owned popup window focus is correctly mapped back to the target top-level window.
- **Issues Found:**
  - Standard popups and owned windows (like the overlay window itself or target application popups) did not climb parent/child structures using `GetAncestor` alone. Interaction with overlays could cause them to briefly activate, stealing focus from the underlying window, causing the outline to blink/disappear.
- **Solutions Applied:**
  - Intercepted mouse activation on overlays with `MA_NOACTIVATE`.
  - Looped ownership resolution (`GW_OWNER`) in `get_root_window` to find top-most parents of popup windows.
- **Verification Proof:**
  - Passed all 39 tests successfully:
    ```
    running 39 tests
    test app::controller::tests::test_controller_initial_state ... ok
    ...
    test app::controller::tests::test_focus_changed_identifies_owned_popup_focus_as_focused ... ok
    ```
