# Commit: Fix Critical WH_KEYBOARD_LL Global Key Consumption Bug

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Identified `LRESULT(1)` return in `keyboard_hook_proc` as the cause of system-wide keyboard freeze.
  - [x] Removed the key-consuming `return LRESULT(1)` statement from `src/platform/hook.rs`.
  - [x] Added a detailed safety comment explaining why the hook must always delegate to `CallNextHookEx`.
  - [x] Verified all 30 unit tests pass after the fix.
  - [x] Verified release build compiles cleanly.

- **Issues Found:**
  - **CRITICAL — System-wide keyboard freeze (Win-Float):** The `keyboard_hook_proc` callback in
    `src/platform/hook.rs` returned `LRESULT(1)` for every keydown and keyup event intercepted
    while the transparency modal was active. This caused ALL keyboard input — across every running
    application — to be silently swallowed for the entire duration the hook was live.

    Root cause: The Windows low-level keyboard hook (`WH_KEYBOARD_LL`) is a global, synchronous
    chain. Any hook proc that returns a non-zero value **without** first calling `CallNextHookEx`
    consumes the event for the entire system, not just for the owning process. The intent was only
    to prevent arrow-key navigation from leaking into background windows during the transparency
    modal, but the effect was a complete keyboard blackout system-wide that persisted until the
    process exited (or crashed). In the observed incident the crash/freeze was severe enough to
    require a forced reboot, and Windows started in recovery mode as a consequence.

- **Solutions Applied:**
  - Removed the `return LRESULT(1)` line entirely. The hook proc now posts the key event to the
    HUD message window via `PostMessageW` (which the app reads as `WM_TACTILE_KEY_EVENT`) and
    then falls through to the existing `CallNextHookEx` call at the bottom of the function.
  - Added a clearly-marked `// NOTE:` comment in the hook proc explaining the danger of consuming
    keystrokes at this level, to prevent the pattern from being reintroduced in the future.
  - The transparency slider still functions correctly: directional-key presses update
    `pressed_keys` and the physics timer drives `slider_percentage`, with no need to suppress the
    underlying OS events.

- **Verification Proof:**
  - Running `cargo test`:
    ```
    running 30 tests
    test hud_layout::tests::test_calculate_hud_position ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test app::controller::tests::test_controller_initial_state ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_window_change ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_window_closed ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_is_below_warning_threshold ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test app::controller::tests::test_slider_physics_friction_stops ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test app::controller::tests::test_slider_physics_acceleration ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok

    test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
    ```
  - Running `cargo build --release`:
    ```
       Compiling win-float v0.1.0 (C:\Users\Diegolas\Code\rust\WIN-FLOAT\win-float)
        Finished `release` profile [optimized] target(s) in 4.02s
    ```
