use tiny_skia::Pixmap;

pub struct Canvas {
    pixmap: Pixmap,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("Canvas dimensions must be greater than zero".to_string());
        }
        let pixmap =
            Pixmap::new(width, height).ok_or_else(|| "Failed to allocate Pixmap".to_string())?;
        Ok(Self { pixmap })
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    pub fn clear(&mut self, color: tiny_skia::Color) {
        self.pixmap.fill(color);
    }

    pub fn pixels(&self) -> &[u8] {
        self.pixmap.data()
    }

    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
    }

    pub fn pixmap_mut(&mut self) -> &mut Pixmap {
        &mut self.pixmap
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Color;

    #[test]
    fn test_canvas_creation() {
        let canvas = Canvas::new(100, 200);
        assert!(canvas.is_ok());
        let c = canvas.unwrap();
        assert_eq!(c.width(), 100);
        assert_eq!(c.height(), 200);
    }

    #[test]
    fn test_canvas_invalid_creation() {
        let canvas = Canvas::new(0, 200);
        assert!(canvas.is_err());
        let canvas2 = Canvas::new(100, 0);
        assert!(canvas2.is_err());
    }

    #[test]
    fn test_canvas_clear() {
        let mut canvas = Canvas::new(10, 10).unwrap();
        let color = Color::from_rgba8(255, 0, 0, 255); // Red
        canvas.clear(color);

        let pixels = canvas.pixels();
        assert_eq!(pixels.len(), 10 * 10 * 4);
        for chunk in pixels.chunks_exact(4) {
            assert_eq!(chunk, &[255, 0, 0, 255]);
        }
    }
}
