# Commit: Pinned window accent overlay repositioning during transparency mode

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Fix tracking in WindowEventTracker to support multiple overlays per target window.
  - [x] Reposition/hide both pinned and HUD overlays during window moves, snaps, and movesize drag loops.
  - [x] Verify with new unit test `test_pinned_transparency_accent_overlay_moves_correctly`.
- **Issues Found:**
  - When entering transparency mode on a focused pinned window, the WindowEventTracker's static map `TRACKED_WINDOWS` would overwrite the target window's mapped overlay from the pinned overlay to the HUD overlay.
  - While in transparency mode, moving the window only updated or hid the HUD overlay, leaving the pinned overlay fixed at the coordinates where the transparency hotkey was pressed.
  - When exiting transparency mode, `stop_tracking` was called on the target window handle, removing it completely from `TRACKED_WINDOWS` and preventing subsequent window movements or events from updating the pinned overlay.
- **Solutions Applied:**
  - Modified `EventTracker` signature and its implementations (`WindowEventTracker`, `MockEventTracker`) to accept both the target window handle and the overlay window handle in `stop_tracking(&self, target_hwnd, overlay_hwnd)`.
  - Modified `TRACKED_WINDOWS` to map each target window to a list (`Vec<HWND>`) of active overlays.
  - Modified `AppController`'s move, movesize, and debouncing handlers to look up, hide, and reposition all active overlays (both pinned and HUD overlays) when they co-exist for the target window.
- **Verification Proof:**
  - All 48 unit tests (including the new regression test `test_pinned_transparency_accent_overlay_moves_correctly`) compile and pass successfully:
    ```
    running 48 tests
    test app::controller::tests::test_controller_ignores_taskbar_and_start_menu ... ok
    test app::controller::tests::test_controller_initial_state ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test app::controller::tests::test_always_on_top_overlay_focus_outline_thickness_and_opacity ... ok
    test app::controller::tests::test_controller_always_on_top_reset_on_drop ... ok
    test app::controller::tests::test_always_on_top_overlay_updates_outline_on_focus_change ... ok
    test app::controller::tests::test_always_on_top_overlay_hidden_during_movesize ... ok
    test app::controller::tests::test_modal_slider_seeds_from_existing_transparency ... ok
    test app::controller::tests::test_focus_changed_ignores_transient_windows ... ok
    test app::controller::tests::test_always_on_top_overlay_hidden_when_maximized ... ok
    test app::controller::tests::test_focus_changed_does_not_redraw_if_focus_state_unchanged ... ok
    test app::controller::tests::test_focus_changed_identifies_owned_popup_focus_as_focused ... ok
    test app::controller::tests::test_slider_physics_friction_stops ... ok
    test app::controller::tests::test_slider_physics_acceleration ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test hud_layout::tests::test_calculate_hud_position ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test platform::window::tests::test_live_window_manager_is_taskbar_or_start_menu ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_window_change ... ok
    test state_machine::tests::test_window_closed ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test tests::test_watchdog_parser_pin_unpin ... ok
    test transparency_calc::tests::test_alpha_to_percentage ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test ui::draw::tests::test_blit_pixmap ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test app::controller::tests::test_focus_changed_identifies_child_window_focus_as_focused ... ok
    test app::controller::tests::test_pinned_transparency_accent_overlay_moves_correctly ... ok
    test app::controller::tests::test_window_moved_debounces_and_updates_on_timer ... ok
    test app::controller::tests::test_window_moved_does_not_redraw_if_size_unchanged ... ok
    test app::controller::tests::test_update_pinned_overlay_graphics_uses_cached_focus_state_when_no_handle_provided ... ok

    test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
    ```
