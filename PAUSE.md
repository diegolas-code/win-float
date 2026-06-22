# Project Session Handoff State

This document tracks the project state at the end of each development session. It must be updated before pausing or concluding any session to ensure smooth continuation for the next agent/developer.

## 1. Last Updated
- **Date:** 2026-06-22
- **Task Reference:** Exclude Taskbar and Start Menu from window targeting

## 2. Session Achievements
- **Exclusion of Taskbar, Start Menu, Tray & Flyouts:** Expanded the `is_taskbar_or_start_menu` method in `LiveWindowManager` to reject any active window or root window belonging to the taskbar, tray clock, tray overflow, quick settings, calendar popups, media controls, third-party start menus, and desktop background.
- **Hierarchical Detection:** Used parent/owner hierarchy climbing to catch sub-components and nested UWP wrapper panels (like `NativeHWNDHost` or `Windows.UI.Core.CoreWindow`) hosted by `explorer.exe` or `ShellExperienceHost.exe`.
- **Descriptive Warning Logging:** Added explicit console warnings outputting the Class, Process, HWND, and Root HWND of the rejected system window.
- **TDD & Unit Testing:** All 44 tests pass successfully, including live checks asserting that both the taskbar and its child elements are correctly identified and rejected.
- **Documentation Update:** Added notes to `README.md` clarifying that the utility is a learning project, and linked the MIT `LICENSE` at the bottom of the file.

## 3. Current Task State
- **Active Task:** Documentation additions and code explanation complete.
- **Status:** Uncommitted changes ready to be committed on branch `feature/exclude-taskbar-startmenu`.

## 4. Pending / Next Steps
- Commit the changes on `feature/exclude-taskbar-startmenu` once the user approves.

## 5. System State & Compile Verification
- **Code compiles:** Yes (`cargo check` and `cargo build` successful)
- **Tests passing:** Yes (`cargo test` successful: 44 tests passed)
