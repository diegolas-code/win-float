# Project Session Handoff State

This document tracks the project state at the end of each development session. It must be updated before pausing or concluding any session to ensure smooth continuation for the next agent/developer.

## 1. Last Updated
- **Date:** 2026-06-23
- **Task Reference:** Documentation of System UI Exclusions

## 2. Session Achievements
- **System UI Exclusions Documentation**: Documented the architectural safeguards, platform-specific filters (such as `get_root_window` traversing ancestry/owners, process checks, class checks, and Explorer UWP controls), and test validations inside [README.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/README.md) and historical log [.history/history_030.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/.history/history_030.md).
- **Multiple Overlays Support in WindowEventTracker**: Updated `TRACKED_WINDOWS` global map to track a list (`Vec<HWND>`) of overlays per target window, allowing the pinned overlay and the HUD overlay to be tracked concurrently.
- **Selective stop_tracking**: Updated the `EventTracker` signature and its implementations to take both the target window handle and the overlay window handle, preventing un-tracking the pinned overlay when exiting the transparency modal.
- **Co-existent Overlay Repositioning**: Modified `AppController`'s move, movesize, and debouncing handlers to correctly hide and reposition all active overlays (both pinned and HUD overlays) when they co-exist for the target window.
- **TDD & Unit Testing:** Wrote a regression test `test_pinned_transparency_accent_overlay_moves_correctly` which reproduces the issue and verifies the fix. All 48 tests pass successfully.

## 3. Current Task State
- **Active Task:** Documentation of exclusions.
- **Status:** Complete.

## 4. Pending / Next Steps
- Commit the changes on branch `fix/pinned-transparency-accent-overlay` (if requested).

## 5. System State & Compile Verification
- **Code compiles:** Yes (`cargo check` and `cargo build` successful)
- **Tests passing:** Yes (`cargo test` successful: 48 tests passed)

