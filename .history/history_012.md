# Commit: Implement Window Event Tracker and TDD tests

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Create [src/app/tracker.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/app/tracker.rs) containing:
    - [WindowEventTracker](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/app/tracker.rs#L16-L103): registers/unregisters Win32 event listeners using `SetWinEventHook` and `UnhookWinEvent`.
    - [win_event_proc](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/app/tracker.rs#L106-L151): system event callback mapping active targets, dispatching `WM_TACTILE_WINDOW_MOVED` and `WM_TACTILE_WINDOW_CLOSED` notifications via GDI `PostMessageW`.
  - [x] Create [src/app/mod.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/app/mod.rs) to declare the `app` module.
  - [x] Write unit tests checking tracker lifecycle states.
  - [x] Register `app` module in [src/main.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/main.rs).
- **Issues Found:**
  - `OBJID_WINDOW` was incorrectly imported from `windows::Win32::UI::Accessibility` but it is located in `windows::Win32::UI::WindowsAndMessaging`.
- **Solutions Applied:**
  - Updated tracker imports to retrieve `OBJID_WINDOW` from `WindowsAndMessaging`.
- **Verification Proof:**
  - `cargo test` output shows 24 passing tests:
    ```
    running 24 tests
    test hud_layout::tests::test_calculate_pin_position ... ok
    test hud_layout::tests::test_calculate_hud_position ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_window_change ... ok
    test state_machine::tests::test_window_closed ... ok
    test transparency_calc::tests::test_is_below_warning_threshold ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok
    test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
    ```
