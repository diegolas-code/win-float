# Commit: Reset always-on-top status of program-managed windows on termination

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Reset always-on-top status for all program-managed (pinned) windows during `AppController::drop` in `src/app/controller.rs`.
  - [x] Sent `PIN`/`UNPIN` commands to the watchdog process via `watchdog_stdin` in `src/app/controller.rs` when windows are pinned/unpinned.
  - [x] Updated the watchdog process in `src/main.rs` to track pinned windows and reset their always-on-top status using `HWND_NOTOPMOST` on EOF (abrupt exit/crash of the main process).
  - [x] Added unit tests verifying always-on-top status reset on `AppController` drop and verifying watchdog command parsing.
- **Issues Found:**
  - Standard Rust unit tests run in parallel by default, which caused test race conditions on global mock statics (`TEST_CLASS_NAMES` / `TEST_OWNERS`) due to overlapping HWND handles (e.g. `HWND(12345)`).
- **Solutions Applied:**
  - Modified test HWND handles to be unique across tests (e.g., `12301`, `12302`, `77701`), completely isolating parallel unit test executions.
- **Verification Proof:**
  - Output of `cargo test` verifying 42 tests passed:
    ```
    running 42 tests
    test app::controller::tests::test_controller_always_on_top_reset_on_drop ... ok
    test tests::test_watchdog_parser_pin_unpin ... ok
    ...
    test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
    ```
