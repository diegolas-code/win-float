# Win-Float App Idea

## Concept
`win-float` is a lightweight, low-overhead utility for Windows written in Rust. It enables users to manipulate and manage their workspace windows directly via global hotkeys, providing active visual feedback through transparent, click-through overlays.

## Core Features

### 1. Always-On-Top Toggle (`Ctrl+Win+F11`)
- **Behavior:** Toggles the topmost state of the active window.
- **Marker:** Draws a small, click-through pin icon/emoji in the top-right corner of the window.
- **Tracking:** The pin overlay automatically tracks the window as it moves or resizes, instantly disappearing if the window is closed.
- **Focused Accent Outline:** When a pinned always-on-top window is focused, a thin outline matching the Windows system accent color is drawn around it to indicate its priority and focus state. The outline fades out automatically when the window loses focus.

### 2. Transparency Adjustment Modal (`Shift+Win+F11`)
- **Behavior:** Enters a modal state where keyboard inputs are captured specifically to adjust the transparency of the active window.
- **Controls:**
  - `Left` / `Down` / `-`: Decrease opacity (more transparent).
  - `Right` / `Up` / `+`: Increase opacity (more opaque).
  - Any other key: Commit transparency changes, exit the modal state, and restore normal input focus.
- **Adjustment Speed:**
  - Quick tap: 5% change.
  - Hold down key: Smooth, continuous scrolling change.
- **Visual HUD:** Displays a floating widget near the window showing the current opacity percentage (e.g. `85%`) along with a visual slider bar.
