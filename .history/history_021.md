# Commit: Always-On-Top Focused Window Accent Outline

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Defined `blit_pixmap` helper in `src/ui/draw.rs` to allow blitting drawing buffers.
  - [x] Registered `EVENT_SYSTEM_FOREGROUND` in `WindowEventTracker` in `src/app/tracker.rs` to track active foreground window changes.
  - [x] Added `WM_TACTILE_FOCUS_CHANGED` to post focus change notifications to tracked window overlays.
  - [x] Sized the always-on-top pin overlay to cover the target window instead of a hardcoded 32x32 size, adjusted to start 10px higher from the top (`y = rect.top - 10`, `height = rect.height() + 10`).
  - [x] Implemented `handle_focus_changed` and `update_pinned_overlay_graphics` inside `AppController` in `src/app/controller.rs` to redraw the overlay when the target window gains/loses focus.
  - [x] Integrated conditional drawing of the system accent border in `update_pinned_overlay_graphics` only when the pinned window is focused.
  - [x] Removed unused transparency warning threshold functions and tests from `src/transparency_calc.rs`.
  - [x] Verified code compiles and passes all 33 tests.

- **Issues Found:**
  - Standard pin overlays were 32x32 pixels, preventing borders from drawing around the entire window edge.
  - Tracking focus changes required interception of `EVENT_SYSTEM_FOREGROUND` without breaking current win event hooks.

- **Solutions Applied:**
  - Resized the pin overlay to match the window bounds, extended 10px taller from the top (`rect.width()` and `rect.height() + 10`), and positioned the pin icon dynamically in the top-right corner of the canvas using `blit_pixmap`.
  - Setup a second win event hook handle in `WindowEventTracker` specifically tracking `EVENT_SYSTEM_FOREGROUND`, sending a thread message to all tracked overlay handles to redraw when focus moves.

- **Verification Proof:**
  - Output of `cargo test`:
    ```
    running 33 tests
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test hud_layout::tests::test_calculate_hud_position ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test app::controller::tests::test_controller_initial_state ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test app::controller::tests::test_modal_slider_seeds_from_existing_transparency ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test app::controller::tests::test_slider_physics_friction_stops ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test app::controller::tests::test_slider_physics_acceleration ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_window_change ... ok
    test transparency_calc::tests::test_alpha_to_percentage ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test state_machine::tests::test_window_closed ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test ui::draw::tests::test_blit_pixmap ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test app::controller::tests::test_always_on_top_overlay_updates_outline_on_focus_change ... ok

    test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
    ```
