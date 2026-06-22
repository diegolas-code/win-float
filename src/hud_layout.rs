#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// Calculates the (x, y) coordinates to place the pin overlay at the top-right corner of the target window.
pub fn calculate_pin_position(
    target: Rect,
    pin_width: i32,
    _pin_height: i32,
    margin_x: i32,
    margin_y: i32,
) -> (i32, i32) {
    let x = target.right - pin_width - margin_x;
    let y = target.top + margin_y;
    (x, y)
}

/// Calculates the (x, y) coordinates to center the HUD overlay within the target window.
pub fn calculate_hud_position(target: Rect, hud_width: i32, hud_height: i32) -> (i32, i32) {
    let x = target.left + (target.width() - hud_width) / 2;
    let y = target.top + (target.height() - hud_height) / 2;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_pin_position() {
        let target = Rect::new(100, 100, 500, 400); // 400x300 window starting at (100, 100)
        let pin_width = 24;
        let pin_height = 24;
        let margin_x = 10;
        let margin_y = 10;

        let expected_x = 500 - 24 - 10; // 466
        let expected_y = 100 + 10; // 110

        assert_eq!(
            calculate_pin_position(target, pin_width, pin_height, margin_x, margin_y),
            (expected_x, expected_y)
        );
    }

    #[test]
    fn test_calculate_hud_position() {
        let target = Rect::new(100, 100, 500, 400); // 400x300 window starting at (100, 100)
        let hud_width = 200;
        let hud_height = 80;

        let expected_x = 100 + (400 - 200) / 2; // 200
        let expected_y = 100 + (300 - 80) / 2; // 210

        assert_eq!(
            calculate_hud_position(target, hud_width, hud_height),
            (expected_x, expected_y)
        );
    }
}
