use crate::ui::overlay::Canvas;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform, Rect as SkiaRect};
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

fn blend_pixel(pixmap: &mut Pixmap, x: u32, y: u32, color: Color, alpha: f32) {
    if x >= pixmap.width() || y >= pixmap.height() {
        return;
    }
    
    let src_a = color.alpha() * alpha;
    if src_a <= 0.0 {
        return;
    }
    
    let src_r = color.red() * src_a;
    let src_g = color.green() * src_a;
    let src_b = color.blue() * src_a;
    
    let idx = ((y * pixmap.width() + x) * 4) as usize;
    let data = pixmap.data_mut();
    
    let dst_r = data[idx] as f32 / 255.0;
    let dst_g = data[idx+1] as f32 / 255.0;
    let dst_b = data[idx+2] as f32 / 255.0;
    let dst_a = data[idx+3] as f32 / 255.0;
    
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a > 0.0 {
        let out_r = src_r + dst_r * (1.0 - src_a);
        let out_g = src_g + dst_g * (1.0 - src_a);
        let out_b = src_b + dst_b * (1.0 - src_a);
        
        data[idx] = (out_r * 255.0).clamp(0.0, 255.0).round() as u8;
        data[idx+1] = (out_g * 255.0).clamp(0.0, 255.0).round() as u8;
        data[idx+2] = (out_b * 255.0).clamp(0.0, 255.0).round() as u8;
        data[idx+3] = (out_a * 255.0).clamp(0.0, 255.0).round() as u8;
    }
}

/// Draws the visual HUD (percentage text and slider bar) onto the canvas.
pub fn draw_hud(
    canvas: &mut Canvas,
    percentage: u8,
    font: &FontArc,
    accent_color: Color,
) -> Result<(), String> {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let pixmap = canvas.pixmap_mut();

    // 1. Draw rounded background widget (glassmorphic look)
    let bg_rect = SkiaRect::from_ltrb(4.0, 4.0, w - 4.0, h - 4.0)
        .ok_or_else(|| "Invalid background dimensions".to_string())?;

    let bg_path = {
        let mut pb = PathBuilder::new();
        let r = 8.0; // corner radius
        let l = bg_rect.left();
        let t = bg_rect.top();
        let right = bg_rect.right();
        let b = bg_rect.bottom();
        
        pb.move_to(l + r, t);
        pb.line_to(right - r, t);
        pb.quad_to(right, t, right, t + r);
        pb.line_to(right, b - r);
        pb.quad_to(right, b, right - r, b);
        pb.line_to(l + r, b);
        pb.quad_to(l, b, l, b - r);
        pb.line_to(l, t + r);
        pb.quad_to(l, t, l + r, t);
        pb.close();
        pb.finish()
    };

    if let Some(ref path) = bg_path {
        let mut bg_paint = Paint::default();
        bg_paint.set_color(Color::from_rgba8(25, 25, 25, 230));
        bg_paint.anti_alias = true;
        pixmap.fill_path(path, &bg_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
        
        let mut outline_paint = Paint::default();
        outline_paint.set_color(Color::from_rgba8(255, 255, 255, 30)); // subtle outline
        outline_paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.0,
            ..Default::default()
        };
        pixmap.stroke_path(path, &outline_paint, &stroke, Transform::identity(), None);
    }

    // 2. Draw slider bar (lower half)
    let slider_x = 20.0;
    let slider_y = h - 24.0;
    let slider_w = w - 40.0;
    let slider_h = 6.0;

    // Track background
    let track_rect = SkiaRect::from_ltrb(slider_x, slider_y, slider_x + slider_w, slider_y + slider_h)
        .ok_or_else(|| "Invalid track dimensions".to_string())?;
    let track_path = PathBuilder::from_rect(track_rect);
    let mut track_paint = Paint::default();
    track_paint.set_color(Color::from_rgba8(60, 60, 60, 255));
    track_paint.anti_alias = true;
    pixmap.fill_path(&track_path, &track_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);

    // Track fill
    if percentage > 0 {
        let fill_w = slider_w * (percentage.min(100) as f32 / 100.0);
        let fill_rect = SkiaRect::from_ltrb(slider_x, slider_y, slider_x + fill_w, slider_y + slider_h)
            .ok_or_else(|| "Invalid fill dimensions".to_string())?;
        let fill_path = PathBuilder::from_rect(fill_rect);
        let mut fill_paint = Paint::default();
        fill_paint.set_color(accent_color);
        fill_paint.anti_alias = true;
        pixmap.fill_path(&fill_path, &fill_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }

    // 3. Render percentage text (upper half)
    let text = format!("{}%", percentage);
    let font_size = 28.0;
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);

    let mut total_width = 0.0;
    let mut glyphs = Vec::new();
    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let h_adv = scaled_font.h_advance(glyph_id);
        glyphs.push((glyph_id, h_adv));
        total_width += h_adv;
    }

    let start_x = (w - total_width) / 2.0;
    let baseline_y = h * 0.45;
    let mut layout_x = start_x;
    for (glyph_id, h_adv) in glyphs {
        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(layout_x, baseline_y));
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|x_pixel, y_pixel, alpha| {
                let px = (bounds.min.x as i32 + x_pixel as i32) as u32;
                let py = (bounds.min.y as i32 + y_pixel as i32) as u32;
                blend_pixel(pixmap, px, py, Color::WHITE, alpha);
            });
        }
        layout_x += h_adv;
    }

    Ok(())
}

/// Draws the visual pin icon onto the canvas.
pub fn draw_pin(
    canvas: &mut Canvas,
    accent_color: Color,
) -> Result<(), String> {
    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let pixmap = canvas.pixmap_mut();

    let cx = width / 2.0;
    let cy = height * 0.35;
    let r = width * 0.25;

    let mut paint = Paint::default();
    paint.set_color(accent_color);
    paint.anti_alias = true;

    // 1. Draw circular pinhead
    let path_head = {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        pb.finish()
    };
    if let Some(ref path) = path_head {
        pixmap.fill_path(path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }

    // 2. Draw shaft/needle pointing downwards
    let path_shaft = {
        let mut pb = PathBuilder::new();
        pb.move_to(cx - 2.0, cy);
        pb.line_to(cx + 2.0, cy);
        pb.line_to(cx, height - 2.0);
        pb.close();
        pb.finish()
    };
    if let Some(ref path) = path_shaft {
        pixmap.fill_path(path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }

    // 3. Draw a white highlight spot on the head to make it look premium/glossy
    let mut paint_hl = Paint::default();
    paint_hl.set_color(Color::from_rgba8(255, 255, 255, 180));
    paint_hl.anti_alias = true;
    let path_hl = {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx - r * 0.3, cy - r * 0.3, r * 0.25);
        pb.finish()
    };
    if let Some(ref path) = path_hl {
        pixmap.fill_path(path, &paint_hl, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }

    Ok(())
}

/// Draws the border outline matching the accent color.
pub fn draw_border(
    canvas: &mut Canvas,
    accent_color: Color,
    border_thickness: f32,
) -> Result<(), String> {
    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let pixmap = canvas.pixmap_mut();

    let half_t = border_thickness / 2.0;
    let rect = SkiaRect::from_ltrb(half_t, half_t, width - half_t, height - half_t)
        .ok_or_else(|| "Invalid border dimensions".to_string())?;
    let path = PathBuilder::from_rect(rect);

    let mut paint = Paint::default();
    paint.set_color(accent_color);
    paint.anti_alias = true;

    let stroke = Stroke {
        width: border_thickness,
        ..Default::default()
    };

    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Color;
    use ab_glyph::FontArc;

    fn load_test_font() -> FontArc {
        let font_bytes = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf")
            .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\arial.ttf"))
            .expect("Failed to load a system font for testing");
        FontArc::try_from_vec(font_bytes).expect("Failed to parse font bytes")
    }

    #[test]
    fn test_draw_hud_modifies_pixels() {
        let mut canvas = Canvas::new(200, 80).unwrap();
        canvas.clear(Color::TRANSPARENT);
        
        let initial_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert_eq!(initial_sum, 0);

        let font = load_test_font();
        let accent = Color::from_rgba8(0, 120, 215, 255);
        draw_hud(&mut canvas, 50, &font, accent).unwrap();

        let modified_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert!(modified_sum > 0); // should fail because dummy does not draw
    }

    #[test]
    fn test_draw_pin_modifies_pixels() {
        let mut canvas = Canvas::new(24, 24).unwrap();
        canvas.clear(Color::TRANSPARENT);
        
        let initial_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert_eq!(initial_sum, 0);

        let accent = Color::from_rgba8(0, 120, 215, 255);
        draw_pin(&mut canvas, accent).unwrap();

        let modified_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert!(modified_sum > 0); // should fail because dummy does not draw
    }

    #[test]
    fn test_draw_border_modifies_pixels() {
        let mut canvas = Canvas::new(100, 100).unwrap();
        canvas.clear(Color::TRANSPARENT);

        let initial_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert_eq!(initial_sum, 0);

        let accent = Color::from_rgba8(0, 120, 215, 255);
        draw_border(&mut canvas, accent, 2.0).unwrap();

        let modified_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert!(modified_sum > 0); // should fail because dummy does not draw
    }
}
