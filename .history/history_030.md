# Commit: Exclude Taskbar, Start Menu, Tray Area, and Shell Flyouts from targeting

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Exclude Taskbar, Start Menu, and System Tray areas from window targeting
  - [x] Implement `is_taskbar_or_start_menu` method in `WindowManager` trait and mock it in tests
  - [x] Implement comprehensive process name and class name checks in `LiveWindowManager`
  - [x] Handle child sub-components and top-level transient flyouts (like clock, battery, volume settings, and quick settings) by climbing ancestor/owner hierarchies
  - [x] Print detailed console warnings with target window class, process, HWND, and Root HWND when rejecting system elements

- **Issues Found:**
  - Standard taskbar checks missed elements like the System Tray notification container, overflow area (`^` popup), clock flyouts, calendar indicators, volume/brightness/media indicators, and Quick Settings/Control Center popups.
  - Many of these widgets are UWP/XAML-based, identifying as `Windows.UI.Core.CoreWindow` or `NativeHWNDHost` and hosted by `explorer.exe` or `ShellExperienceHost.exe`, which requires context-sensitive filtering.

- **Solutions Applied:**
  - Added a private helper function `get_root_window` in `src/platform/window.rs` that climbs the ancestor/owner hierarchy using `GetAncestor(hwnd, GA_ROOTOWNER)` and `GetWindow(hwnd, GW_OWNER)`.
  - Updated `LiveWindowManager::is_taskbar_or_start_menu` to query process image name and class name for both the active window handle and its resolved root owner window.
  - Excluded processes: `startmenuexperiencehost.exe`, `searchhost.exe`, `shellexperiencehost.exe`.
  - Excluded class names: `Shell_TrayWnd`, `Shell_SecondaryTrayWnd`, `TrayNotifyWnd`, `NotifyIconOverflowWindow`, `TrayClockWClass`, `ClockFlyoutWindow`, `ControlCenterWindow`, `Shell_LightDismissOverlay`, `ClassicShell.CMenuContainer`, `OpenShell.CMenuContainer`, `DV2ControlHost`, `XamlExplorerHostIslandWindow`, `Progman`, `WorkerW`.
  - Excluded UWP/XAML containers hosted by `explorer.exe` (matching class names `Windows.UI.Core.CoreWindow` or `NativeHWNDHost`).
  - Added descriptive console warnings printed directly from the Win32 implementation (detailing Class, Process, HWND, and Root HWND) whenever system windows are targeted.

- **Verification Proof:**
  - Output of `cargo test`:
    ```
    running 44 tests
    test app::controller::tests::test_controller_initial_state ... ok
    test app::controller::tests::test_controller_ignores_taskbar_and_start_menu ... ok
    test app::controller::tests::test_controller_transparency_modal ... ok
    test app::controller::tests::test_always_on_top_overlay_focus_outline_thickness_and_opacity ... ok
    test app::controller::tests::test_controller_always_on_top_reset_on_drop ... ok
    test app::controller::tests::test_modal_slider_seeds_from_existing_transparency ... ok
    test app::controller::tests::test_controller_topmost_toggle ... ok
    test app::controller::tests::test_slider_physics_acceleration ... ok
    test app::controller::tests::test_slider_physics_friction_stops ... ok
    test app::controller::tests::test_always_on_top_overlay_updates_outline_on_focus_change ... ok
    test app::tracker::tests::test_tracker_lifecycle_states ... ok
    test hud_layout::tests::test_calculate_hud_position ... ok
    test platform::hook::tests::test_hook_lifecycle_states ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test platform::window::tests::test_live_window_manager_get_active_handles_errors ... ok
    test app::controller::tests::test_window_moved_does_not_redraw_if_size_unchanged ... ok
    test platform::window::tests::test_live_window_manager_rejects_fake_hwnd ... ok
    test platform::window::tests::test_live_window_manager_rejects_null_hwnd ... ok
    test platform::window::tests::test_live_overlay_manager_lifecycle ... ok
    test state_machine::tests::test_adjust_clamping ... ok
    test platform::window::tests::test_live_window_manager_is_taskbar_or_start_menu ... ok
    test state_machine::tests::test_adjust_transparency ... ok
    test state_machine::tests::test_commit_action ... ok
    test state_machine::tests::test_enter_modal ... ok
    test state_machine::tests::test_window_closed ... ok
    test state_machine::tests::test_initial_state ... ok
    test state_machine::tests::test_window_change ... ok
    test tests::test_watchdog_parser_pin_unpin ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_alpha_to_percentage ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test ui::draw::tests::test_blit_pixmap ... ok
    test ui::draw::tests::test_draw_pin_modifies_pixels ... ok
    test ui::draw::tests::test_draw_border_modifies_pixels ... ok
    test ui::overlay::tests::test_canvas_clear ... ok
    test app::controller::tests::test_focus_changed_ignores_transient_windows ... ok
    test ui::overlay::tests::test_canvas_creation ... ok
    test ui::overlay::tests::test_canvas_invalid_creation ... ok
    test ui::draw::tests::test_draw_hud_modifies_pixels ... ok
    test app::controller::tests::test_focus_changed_identifies_child_window_focus_as_focused ... ok
    test app::controller::tests::test_focus_changed_identifies_owned_popup_focus_as_focused ... ok
    test app::controller::tests::test_focus_changed_does_not_redraw_if_focus_state_unchanged ... ok
    test app::controller::tests::test_update_pinned_overlay_graphics_uses_cached_focus_state_when_no_handle_provided ... ok

    test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
    ```
