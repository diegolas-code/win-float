# Design Plan: Suppress Target Window Key Inputs During Transparency Mode

- **Date:** 2026-06-30
- **Feature Description:** When the transparency adjustment modal is active (the slider HUD is visible), intercept and swallow keyboard inputs so they are not sent to the target window being adjusted. This prevents accidental typing or cursor movement in the target application (e.g. typing arrow keys, space, enter) while interacting with the slider.
- **Scope:**
  - Swallow all standard keyboard events (letters, digits, space, arrow keys, escape, enter, etc.) when the active window is the target window.
  - Do NOT swallow modifier keys themselves (Ctrl, Alt, Win).
  - Do NOT swallow key combinations involving Alt (e.g., `Alt + Tab`) or Ctrl (e.g., `Ctrl + C`), ensuring normal system operations and window-switching shortcuts still function.
  - Do NOT swallow any keys if the target window is not the active/foreground window.

## Architecture & Components

We will introduce a pure, testable function `should_swallow_key` in `src/platform/hook.rs`:

```rust
pub fn should_swallow_key(
    vk_code: u32,
    flags: u32,
    is_ctrl_down: bool,
    is_target_active: bool,
) -> bool {
    if !is_target_active {
        return false;
    }

    use windows::Win32::UI::WindowsAndMessaging::LLKHF_ALTDOWN;

    // Modifiers themselves: Ctrl (0x11, 0xA2, 0xA3), Alt (0x12, 0xA4, 0xA5), Win (0x5B, 0x5C)
    let is_modifier = matches!(
        vk_code,
        0x11 | 0xA2 | 0xA3 | // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
        0x12 | 0xA4 | 0xA5 | // VK_MENU, VK_LMENU, VK_RMENU
        0x5B | 0x5C          // VK_LWIN, VK_RWIN
    );

    let is_alt_down = (flags & LLKHF_ALTDOWN.0) != 0;

    if is_modifier || is_alt_down || is_ctrl_down {
        return false;
    }

    true
}
```

This helper will be called within `keyboard_hook_proc` to determine if we return `LRESULT(1)` to swallow the event:

```rust
let fg_window = unsafe { GetForegroundWindow() };
let is_target_active = fg_window == state.target_hwnd;
let is_ctrl_down = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };

if should_swallow_key(vk_code, kbd_struct.flags.0, is_ctrl_down, is_target_active) {
    return LRESULT(1);
}
```

## Testing & Verification Plan

Following the project's strict TDD rules:
1. Write unit tests for `should_swallow_key` first (verifying normal swallow, inactive target pass, modifier pass, alt pass, ctrl pass).
2. Verify tests fail (or fail to compile because the function doesn't exist yet).
3. Implement `should_swallow_key`.
4. Verify tests pass.
5. Integrate `should_swallow_key` into `keyboard_hook_proc`.
