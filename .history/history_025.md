# Commit: Fallback to cached focus state on layout updates to resolve window focus blinking

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Prevented `update_pinned_overlay_graphics` from querying the active window manager when `new_fg_hwnd` is `None` (for example, during `handle_window_moved` notifications).
  - [x] Implemented a fallback that uses the cached focus state from `overlay_focus_states` if it is present.
  - [x] Added the `test_update_pinned_overlay_graphics_uses_cached_focus_state_when_no_handle_provided` unit test verifying that focus cache values are used during graphics updates instead of querying the potentially racy active window manager.
- **Issues Found:**
  - When switching focus, target windows gained focus correctly, but intermediate OS layout transitions triggered duplicate `EVENT_OBJECT_LOCATIONCHANGE` events. These triggered `handle_window_moved` which queried `GetForegroundWindow()`. Because `GetForegroundWindow()` can be racey during foreground changes, it returned `NULL` or the old handle, incorrectly resetting the focus cache to `false` and causing the border to instantly disappear.
- **Solutions Applied:**
  - Standardized on the cached focus state as the source of truth for all geometry updates. Focus states are only recalculated when explicit `EVENT_SYSTEM_FOREGROUND` events are processed.
- **Verification Proof:**
  - Verified with 37 unit tests:
    ```
    running 37 tests
    test app::controller::tests::test_controller_initial_state ... ok
    ...
    test app::controller::tests::test_update_pinned_overlay_graphics_uses_cached_focus_state_when_no_handle_provided ... ok
    ```
