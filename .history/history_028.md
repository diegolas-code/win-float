# Commit: Filter transient focus windows in window focus tracking

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Implemented transient window filtering in `handle_focus_changed` and `update_pinned_overlay_graphics` inside `src/app/controller.rs` to ignore OS focus-staging or task-switching windows (`ForegroundStaging`, `XamlExplorerHostIslandWindow`, `MultitaskingViewFrame`).
  - [x] Declared `TEST_CLASS_NAMES` mock hashmap and implemented a `get_window_class_name` helper function that supports window class testing.
  - [x] Added `test_focus_changed_ignores_transient_windows` verifying that OS transient staging and switching windows are ignored during focus updates, keeping the outline border visible on the target pinned window.
- **Issues Found:**
  - When Alt+Tabbing or clicking back to a pinned window, Windows OS triggers transient foreground changes (e.g. to a staging window `ForegroundStaging` or task switcher window `XamlExplorerHostIslandWindow`). WIN-FLOAT processed these events as loss of focus, causing the accent outline border to instantly disappear right after appearing.
- **Solutions Applied:**
  - Queried the class name of foreground windows using `GetClassNameW` and ignored them if they match transient OS classes (`ForegroundStaging`, `XamlExplorerHostIslandWindow`, `MultitaskingViewFrame`).
- **Verification Proof:**
  - Passed all 40 tests successfully:
    ```
    running 40 tests
    test app::controller::tests::test_controller_initial_state ... ok
    ...
    test app::controller::tests::test_focus_changed_ignores_transient_windows ... ok
    ```
