# Commit: Resolve Overlay Z-Order, Clamping, Jumps, Icon and Escape Issues

- **Date:** 2026-06-21
- **Tasks Completed:**
  - Resolved the Z-order issue of the transparent overlay windows. Added a parent HWND parameter to `OverlayManager::create_overlay` to link the overlay with the target window, which guarantees in Win32 that the owned window stays in front of its owner.
  - Replaced the standard pin vector icon with a highly stylized, vector-drawn bee icon (🐝) inside `src/ui/draw.rs` using Skia shapes.
  - Adjusted transparency step size (jumps) from 5% to 2% to make transitions smooth.
  - Clamped target window transparency to a minimum of 60% inside the state machine and transitions.
  - Registered a console control handler (`SetConsoleCtrlHandler`) inside `src/main.rs` to trap and process standard `Ctrl+C` events for graceful process termination.
- **Issues Found:**
  - Standard `DefWindowProcW` was generics-overloaded causing mismatch error; resolved using a direct `overlay_wnd_proc` wrapper.
  - Test assertions failed due to changes in transition step size; updated assertions inside `src/state_machine.rs` and `src/app/controller.rs` to reflect the new 2% step size and 60% minimum clamp.
- **Verification Proof:**
  - All 28 unit tests pass successfully.
  - Release target builds cleanly with optimized profiles.
