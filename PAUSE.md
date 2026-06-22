# Project Session Handoff State

This document tracks the project state at the end of each development session. It must be updated before pausing or concluding any session to ensure smooth continuation for the next agent/developer.

## 1. Last Updated
- **Date:** 2026-06-22
- **Task Reference:** Exclude Taskbar and Start Menu from window targeting

## 2. Session Achievements
- **Exclusion of Taskbar, Start Menu, Tray & Flyouts:** Expanded the `is_taskbar_or_start_menu` method in `LiveWindowManager` to reject any active window or root window belonging to the taskbar, tray clock, tray overflow, quick settings, calendar popups, media controls, third-party start menus, and desktop background.
- **Hierarchical Detection:** Used parent/owner hierarchy climbing to catch sub-components and nested UWP wrapper panels (like `NativeHWNDHost` or `Windows.UI.Core.CoreWindow`) hosted by `explorer.exe` or `ShellExperienceHost.exe`.
- **Descriptive Warning Logging:** Added explicit console warnings outputting the Class, Process, HWND, and Root HWND of the rejected system window.
- **TDD & Unit Testing:** All 46 tests pass successfully, including live checks asserting that both the taskbar and its child elements are correctly identified and rejected.
- **Documentation Update:** Added notes to `README.md` clarifying that the utility is a learning project, and linked the MIT `LICENSE` at the bottom of the file.
- **Accent Overlay Alignment & 7px Top Offset:** Adjusted the top layout offset in `src/app/controller.rs` to start 7px higher (`y = rect.top - 7` and `height = rect.height() + 7`) to float symmetrically around the window, matching the 7-8px invisible borders of modern Windows 10/11 windows.
- **Movesize Overlay Hiding:** Registered win event hooks for `EVENT_SYSTEM_MOVESIZESTART` and `EVENT_SYSTEM_MOVESIZEEND` in `src/app/tracker.rs`. When resizing or dragging starts, the overlay is hidden. When dragging/resizing ends, the overlay is repositioned to the final rect, redrawn at the final size, and made visible again.
- **Maximized Window Overlay Hiding:** Implemented an `is_maximized` query via the Win32 `IsZoomed` API. If the window becomes maximized, the accent outline overlay is completely hidden, and it automatically reappears when restored.

## 3. Current Task State
- **Active Task:** None. Accent overlay resizing inconsistencies and the 7px top offset adjustments are fully complete.
- **Status:** All changes tested and verified.

## 4. Pending / Next Steps
- Commit and merge the changes on `fix/accent-overlay-resize` to master/main (pending user approval/request).

## 5. System State & Compile Verification
- **Code compiles:** Yes (`cargo check` and `cargo build` successful)
- **Tests passing:** Yes (`cargo test` successful: 46 tests passed)
