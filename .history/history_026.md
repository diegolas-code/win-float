# Commit: Climb child window ancestor chain on focus updates to prevent outline disappearance

- **Date:** 2026-06-21
- **Tasks Completed:**
  - [x] Implemented `get_root_window` helper function to resolve the top-level owner window parent of any focused window handle using Win32 `GetAncestor` with `GA_ROOTOWNER`.
  - [x] Updated `handle_focus_changed` and `update_pinned_overlay_graphics` to perform focus evaluation on the resolved top-level owner window instead of the raw event-gained handle.
  - [x] Added `test_focus_changed_identifies_child_window_focus_as_focused` verifying that parent focus state is correctly identified even when child windows/controls receive focus.
- **Issues Found:**
  - Pinned window outlines disappeared as soon as focus was gained because modern applications (e.g. Chrome, editors, dialogs) shift focus to child controls/documents immediately upon window activation. This generated focus changed notifications containing child HWNDs, which did not directly match the pinned window handle and caused focus checks to evaluate to false (unfocusing the overlay).
- **Solutions Applied:**
  - Standardized on the top-level owner window parent (`GA_ROOTOWNER`) when checking if a newly focused handle belongs to the target pinned window.
- **Verification Proof:**
  - Passed all 38 tests:
    ```
    running 38 tests
    test app::controller::tests::test_controller_initial_state ... ok
    ...
    test app::controller::tests::test_focus_changed_identifies_child_window_focus_as_focused ... ok
    ```
