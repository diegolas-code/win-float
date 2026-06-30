# Commit: Suppress target window key inputs during transparency modal

- **Date:** 2026-06-30
- **Tasks Completed:**
  - [x] Implement pure `should_swallow_key` function in `src/platform/hook.rs` to decide if keys should be swallowed.
  - [x] Write comprehensive unit tests for `should_swallow_key` verifying modifier keys, Ctrl/Alt combinations, active target window state, and general keys.
  - [x] Integrate `should_swallow_key` in `keyboard_hook_proc` to return `LRESULT(1)` for swallowed keys, blocking them from the active target window.
  - [x] Defer releasing the keyboard hook on modal exit until all physical direction/commit keys are released to prevent rogue repeating key leaks.
  - [x] Write unit test `test_controller_transparency_modal_deferred_unhook` to verify this behavior.
- **Issues Found:**
  - Standard low-level keyboard hooks can block keyboard events system-wide if not carefully filtered.
  - User shortcuts like `Alt + Tab`, modifier keys, and `Ctrl` combinations must be preserved.
  - **Bug:** The active target window handle was not passed to the input hook. The hook evaluated the foreground window against the message-only receiver window handle (`msg_hwnd`), which is never foreground. Consequently, `should_swallow` always returned `false`, and key inputs (like arrow keys) leaked through to the target window.
  - **Bug:** Releasing the keyboard hook immediately on modal commit/abort while the user is still physically holding down direction or commit keys (Enter/Escape) causes those keys to repeat and get stuck inside the OS (e.g. leaking to the Alt+Tab app switcher).
- **Solutions Applied:**
  - Configured `should_swallow_key` to fail-safe and return `false` (do not swallow) if the target window is not active, or if modifiers, Alt combinations, or Ctrl combinations are detected.
  - Integrated `GetForegroundWindow` and `GetKeyState` checks in the hook callback to safely determine active state and Ctrl key state.
  - **Fix:** Modified the `InputHook` trait and `LiveInputHook`'s `capture_keyboard` method to accept `target_hwnd` from the controller. Renamed the hook's internal tracking field to distinguish between `receiver_hwnd` (destination of posted messages) and `target_hwnd` (the window being transparented). The hook now correctly compares the foreground window against the window being transparented.
  - **Fix:** Added `exiting_modal` state to `AppController` and deferred the unhooking of the keyboard hook inside `Transition::Committed` and `Transition::Aborted` if keys are still physically held down. Released the hook on subsequent keyup events once the physical keys are fully released.
- **Verification Proof:**
  - Output of `cargo test`:
    ```
    running 49 tests
    test app::controller::tests::test_controller_ignores_taskbar_and_start_menu ... ok
    test app::controller::tests::test_controller_initial_state ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test app::controller::tests::test_always_on_top_overlay_focus_outline_thickness_and_opacity ... ok
    test app::controller::tests::test_controller_always_on_top_reset_on_drop ... ok
    test app::controller::tests::test_always_on_top_overlay_updates_outline_on_focus_change ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test app::controller::tests::test_modal_slider_seeds_from_existing_transparency ... ok
    test app::controller::tests::test_always_on_top_overlay_hidden_during_movesize ... ok
    test app::controller::tests::test_focus_changed_identifies_owned_popup_focus_as_focused ... ok
    test app::controller::tests::test_focus_changed_ignores_transient_windows ... ok
    test app::always_on_top_overlay_hidden_when_maximized ... ok
    test app::controller::tests::test_slider_physics_acceleration ... ok
    test app::controller::tests::test_focus_changed_identifies_child_window_focus_as_focused ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test app::controller::tests::test_slider_physics_friction_stops ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test platform::hook::tests::test_should_swallow_key_cases ... ok
    test app::controller::tests::test_pinned_transparency_accent_overlay_moves_correctly ... ok
    test hud_layout::tests::test_calculate_hud_position ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_window_change ... ok
    test state_machine::tests::test_window_closed ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test tests::test_watchdog_parser_pin_unpin ... ok
    test transparency_calc::tests::test_alpha_to_percentage ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test app::controller::tests::test_window_moved_does_not_redraw_if_size_unchanged ... ok
    test ui::draw::tests::test_blit_pixmap ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test platform::window::tests::test_live_window_manager_is_taskbar_or_start_menu ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test app::controller::tests::test_window_moved_debounces_and_updates_on_timer ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok
    test app::controller::tests::test_update_pinned_overlay_graphics_uses_cached_focus_state_when_no_handle_provided ... ok
    test app::controller::tests::test_focus_changed_does_not_redraw_if_focus_state_unchanged ... ok

    test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
    ```
