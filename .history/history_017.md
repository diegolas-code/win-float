# Commit: Fix multithreaded console Ctrl+C event routing

- **Date:** 2026-06-21
- **Tasks Completed:**
  - Resolved the background process termination failure by tracking the main message thread ID using `GetCurrentThreadId` and storing it statically.
  - Replaced the failing local `PostQuitMessage(0)` inside `ctrl_handler` with `PostThreadMessageW` sending a `WM_QUIT` message directly to the main thread's message queue.
  - Enabled `"Win32_System_Threading"` feature in `Cargo.toml` to import thread APIs.
  - Removed unused `PostQuitMessage` imports to ensure zero warnings.
- **Issues Found:**
  - `PostQuitMessage` posts `WM_QUIT` to the calling thread. The console Ctrl+C callback runs on a separate OS-spawned background thread, so calling `PostQuitMessage` there was posting `WM_QUIT` to the background thread itself. The main thread's `GetMessageW` loop was never notified and kept running in the background.
- **Solutions Applied:**
  - Used `PostThreadMessageW` to explicitly target the main thread with `WM_QUIT`, terminating the event loop cleanly and triggering destructor drops.
- **Verification Proof:**
  - All 28 tests pass successfully.
  - Release build compile completes cleanly.
