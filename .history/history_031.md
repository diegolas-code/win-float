# Commit: Align accent overlay exactly with target window bounds

- **Date:** 2026-06-22
- **Tasks Completed:**
  - [x] Removed the asymmetrical 8px top layout offset in `src/app/controller.rs` when creating and updating overlays.
  - [x] Updated coordinates and size assertions in unit tests to align with 0-offset top and height.
  - [x] Formatted and checked the project files using `cargo fmt` and `cargo clippy`.
- **Issues Found:**
  - The accent outline border was asymmetrical: it started 8px higher from the top (`y = rect.top - 8`) but aligned exactly at the left, right, and bottom edges of the window. This resulted in the top outline edge floating 8px above the window, while the other edges sat exactly on the window border.
- **Solutions Applied:**
  - Standardized the overlay coordinates to match the target window exactly (`y = rect.top`, `height = rect.height()`). The accent border now draws symmetrically around all four sides of the window.
- **Verification Proof:**
  - `cargo test` succeeded: 44 passed; 0 failed; 0 ignored.
