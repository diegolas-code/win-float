# Commit: Resize bee icon to 32x32 and enable graceful Ctrl+C console handling

- **Date:** 2026-06-21
- **Tasks Completed:**
  - Configured the dimensions of the bee overlay window to 32x32 in `src/app/controller.rs` (both in the topmost hotkey handler and the active window tracking relocator).
  - Modified the bee vector drawing logic in `draw_pin` in `src/ui/draw.rs` to dynamically scale the coordinates using a global transform (`Transform::from_scale`) based on the canvas dimensions, so that the bee fits the 32x32 bounds perfectly and remains sharp.
  - Removed `#![windows_subsystem = "windows"]` from `src/main.rs` to compile the app as a standard console application. This ensures the command terminal maintains attach-control of the process when run, allowing standard `Ctrl+C` terminal signals (`CTRL_C_EVENT`) to propagate to it.
  - Re-implemented the console control handler callback in `src/main.rs` to call `PostQuitMessage(0)` on `CTRL_C_EVENT`. This posts a `WM_QUIT` thread message, which exits the `GetMessageW` message loop in `AppController::run` gracefully, allowing all Rust destructors (drops) to run and cleanly release hooks and GDI contexts before process termination.
- **Issues Found:**
  - GUI subsystem mode decoupled the process from console inputs, preventing `Ctrl+C` from sending signals to the app. Setting it as a console subsystem app solved this.
  - Immediate `std::process::exit(0)` inside signal callbacks bypassed Rust destructor drops (leaving low-level hook handles active). Exiting via `PostQuitMessage(0)` terminates the main event loop cleanly, ensuring correct cleanup.
- **Verification Proof:**
  - All 28 tests pass successfully.
  - Release build compile completes cleanly.
