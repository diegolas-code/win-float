# Commit: Implement Layout Mathematics logic and TDD tests

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Create [src/hud_layout.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/hud_layout.rs) containing:
    - [Rect](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/hud_layout.rs#L1-L19): geometric shape structure for window bounds representing left, top, right, bottom coordinate points.
    - [calculate_pin_position](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/hud_layout.rs#L22-L33): determines overlay coordinate alignment for the top-right corner of the target window.
    - [calculate_hud_position](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/hud_layout.rs#L36-L45): determines centered coordinate alignment within target window bounds for the slider overlay.
  - [x] Write TDD failing test asserts, verify failure, and complete calculations to pass tests.
  - [x] Register `hud_layout` module in [src/main.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/main.rs).
- **Issues Found:**
  - None.
- **Solutions Applied:**
  - None.
- **Verification Proof:**
  - `cargo test` run:
    ```
    running 6 tests
    test hud_layout::tests::test_calculate_hud_position ... ok
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test hud_layout::tests::test_calculate_pin_position ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test transparency_calc::tests::test_is_below_warning_threshold ... ok
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    ```
