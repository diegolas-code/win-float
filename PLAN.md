# Win-Float High-Level Phase Plan

This document outlines the high-level development phases of `win-float`. Each phase must be completed and verified before proceeding to the next.

## Phase 1: Infrastructure & Decoupling Setup
- **Objective:** Configure dependencies and build mock structures to enable testability.
- **Tasks:**
  - Setup `Cargo.toml` with `windows` and `tiny-skia`.
  - Design traits for `WindowManager` and `InputHook`.
  - Implement mock types to simulate Windows OS state.

## Phase 2: Core Logical Engines (TDD)
- **Objective:** Build math and state logic entirely covered by unit tests.
- **Tasks:**
  - Build `transparency_calc` (percentage to alpha mapping, clamping, warning threshold).
  - Build `hud_layout` (coordinate mapping for pin overlays and HUD offsets).
  - Build the state machine (`AppState`) to transition between `Idle` and `TransparencyModal`.

## Phase 3: Skia Canvas UI rendering
- **Objective:** Set up the visual drawing layer for the HUD overlays.
- **Tasks:**
  - Implement Skia rendering wrapper utilizing `tiny-skia`.
  - Draw pin emoji/icon representation.
  - Draw HUD box (percentage text and horizontal slider bar).
  - Draw border outline based on system accent color queries.

## Phase 4: Live Win32 Platform Integrations
- **Objective:** Bridge the mocked layers with real Win32 system APIs.
- **Tasks:**
  - Implement `LiveWindowManager` using `windows` crate API endpoints.
  - Set up low-level keyboard hooks (`WH_KEYBOARD_LL`) for modal capture.
  - Implement accent color fetcher query (`DwmGetColorizationColor`).

## Phase 5: Message Loop, Passive Tracking & Verification
- **Objective:** Unify all components into a running utility.
- **Tasks:**
  - Implement `AppController` message loop (`GetMessageW`).
  - Wire up `SetWinEventHook` to passively track window movement and closure.
  - Build and perform manual verification checks.
