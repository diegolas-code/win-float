# Commit: Implement Transparency Math logic and TDD tests

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Create [src/transparency_calc.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/transparency_calc.rs) containing:
    - [clamp_percentage](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/transparency_calc.rs#L2-L10): clamps percentage value to range `0..=100`.
    - [percentage_to_alpha](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/transparency_calc.rs#L13-L16): converts percentage (0-100) to Windows alpha byte (0-255).
    - [is_below_warning_threshold](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/transparency_calc.rs#L19-L21): returns true if percentage < 15.
  - [x] Write comprehensive unit tests for clamping, alpha conversion, and warning threshold logic.
  - [x] Register `transparency_calc` in [src/main.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/main.rs).
- **Issues Found:**
  - None.
- **Solutions Applied:**
  - None.
- **Verification Proof:**
  - `cargo test` run:
    ```
    running 4 tests
    test traits::tests::test_mock_window_manager_records_calls ... ok
    test transparency_calc::tests::test_clamp_percentage ... ok
    test transparency_calc::tests::test_is_below_warning_threshold ... ok
    test transparency_calc::tests::test_percentage_to_alpha ... ok
    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    ```
