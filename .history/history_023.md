# Commit: Optimize overlay focus transition speed by passing event payload directly

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Avoided asynchronous querying of `GetForegroundWindow()` on the main event thread when receiving foreground focus changed messages.
  - [x] Sent the newly focused window handle (`hwnd`) directly as `LPARAM` in `PostMessageW(..., WM_TACTILE_FOCUS_CHANGED, ...)` inside `win_event_proc`.
  - [x] Updated `handle_focus_changed` and `update_pinned_overlay_graphics` signatures to accept and evaluate `new_fg_hwnd` directly if provided.
  - [x] Updated the unit test `test_always_on_top_overlay_updates_outline_on_focus_change` to verify that passing a new focused window handle updates the overlay focus state instantly without needing to mock/change the window manager active window state.
- **Issues Found:**
  - Standard out-of-context WinEvents occur asynchronously, and querying `GetForegroundWindow()` immediately on the main thread after receiving a focus change notification could occasionally return the old foreground window handle due to OS scheduling race conditions, causing a small visible lag/delay before the overlay updated.
- **Solutions Applied:**
  - Direct comparison with the event hook payload (`hwnd`) inside the update loop, bypassing `GetForegroundWindow()` completely for event-driven updates.
- **Verification Proof:**
  - Running `cargo test` compiled and passed all 34 tests successfully:
    ```
    running 34 tests
    test app::controller::tests::test_controller_initial_state ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test hud_layout::tests::test_calculate_hud_position ... ok
    test app::controller::tests::test_slider_physics_friction_stops ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test app::controller::tests::test_modal_slider_seeds_from_existing_transparency ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_initial_state ... ok
    test app::controller::tests::test_slider_physics_acceleration ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_window_change ... ok
    test state_machine::tests::test_window_closed ... ok
    test transparency_calc::tests::test_alpha_to_percentage ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test ui::draw::tests::test_blit_pixmap ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok
    test app::controller::tests::test_always_on_top_overlay_focus_outline_thickness_and_opacity ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test app::controller::tests::test_always_on_top_overlay_updates_outline_on_focus_change ... ok

    test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
    ```
