# Commit: Implement AppController and LiveOverlayManager

- **Date:** 2026-06-21
- **Tasks Completed:**
  - Implemented `AppController` logic inside `src/app/controller.rs` running `GetMessageW` loop to handle hotkeys, keyboard events, and window tracking.
  - Implemented `LiveOverlayManager` inside `src/platform/window.rs` using `CreateWindowExW` and `UpdateLayeredWindow` swizzling RGBA pixels to BGRA DIB sections.
  - Added `is_always_on_top` decoupling method to `WindowManager` trait and both `MockWindowManager` and `LiveWindowManager` implementations.
  - Enabled `Win32_Graphics_Gdi` feature in `Cargo.toml` to support the GDI memory context and DIB section drawing operations.
  - Added unit test `test_live_overlay_manager_lifecycle` verifying the live creation, update, and deletion of overlay windows.
- **Issues Found:**
  - `ab_glyph::FontArc::default()` does not exist; replaced by returning `Result<Self, String>` from `AppController::new` to propagate system font errors properly.
  - Windows API constants like `RegisterHotKey`, `MOD_CONTROL`, etc., belong to `Win32::UI::Input::KeyboardAndMouse`, not `Win32::UI::WindowsAndMessaging`.
  - Direct Win32 `GetWindowLongW` call in `AppController` violated traits decoupling rules and caused mock tests to fail; fixed by moving topmost state query behind the `WindowManager` trait.
  - Rust 2024 edition requires explicit `unsafe` blocks inside `unsafe fn` body to call external Win32 functions like `DefWindowProcW`.
- **Solutions Applied:**
  - Propagated font loading errors up using `Result`.
  - Corrected imports for hotkeys and modifiers.
  - Created `is_always_on_top` trait method and mocked it.
  - Wrapped `DefWindowProcW` inside `unsafe` block in the callback.
- **Verification Proof:**
  - Compiles cleanly with zero warnings/errors.
  - `cargo test` results:
    ```
    running 28 tests
    test hud_layout::tests::test_calculate_hud_position ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test app::controller::tests::test_controller_initial_state ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_window_change ... ok
    test state_machine::tests::test_window_closed ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_is_below_warning_threshold ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok

    test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
    ```
