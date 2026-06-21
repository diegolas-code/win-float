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

/// Draws the visual pin icon (bee) onto the canvas.
pub fn draw_pin(
    canvas: &mut Canvas,
    _accent_color: Color,
) -> Result<(), String> {
    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let pixmap = canvas.pixmap_mut();

    // Base coordinates are designed for a 24x24 canvas
    let cx = 12.0;
    let cy = 12.0;

    let scale_x = width / 24.0;
    let scale_y = height / 24.0;
    let global_transform = Transform::from_scale(scale_x, scale_y);

    // 1. Draw Wings (drawn behind the body)
    // Left Wing (translucent light blue)
    let left_wing_path = {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx - 5.0, cy - 3.0, 4.5);
        pb.finish()
    };
    // Right Wing
    let right_wing_path = {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx + 5.0, cy - 3.0, 4.5);
        pb.finish()
    };

    let mut wing_paint = Paint::default();
    wing_paint.set_color(Color::from_rgba8(200, 230, 255, 180));
    wing_paint.anti_alias = true;

    if let Some(ref path) = left_wing_path {
        pixmap.fill_path(path, &wing_paint, tiny_skia::FillRule::Winding, global_transform, None);
    }
    if let Some(ref path) = right_wing_path {
        pixmap.fill_path(path, &wing_paint, tiny_skia::FillRule::Winding, global_transform, None);
    }

    // 2. Draw Stinger (black triangle pointing down)
    let stinger_path = {
        let mut pb = PathBuilder::new();
        pb.move_to(cx - 2.0, cy + 6.0);
        pb.line_to(cx + 2.0, cy + 6.0);
        pb.line_to(cx, cy + 10.0);
        pb.close();
        pb.finish()
    };
    let mut black_paint = Paint::default();
    black_paint.set_color(Color::from_rgba8(20, 20, 20, 255));
    black_paint.anti_alias = true;

    if let Some(ref path) = stinger_path {
        pixmap.fill_path(path, &black_paint, tiny_skia::FillRule::Winding, global_transform, None);
    }

    // 3. Draw Body (yellow oval using circle + vertical scaling)
    let body_path = {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy + 1.0, 6.0);
        pb.finish()
    };
    let mut body_paint = Paint::default();
    body_paint.set_color(Color::from_rgba8(255, 205, 0, 255)); // bright yellow
    body_paint.anti_alias = true;

    if let Some(ref path) = body_path {
        let t = Transform::from_scale(0.85, 1.15).post_translate(cx * (1.0 - 0.85), (cy + 1.0) * (1.0 - 1.15));
        let t_scaled = t.post_concat(global_transform);
        pixmap.fill_path(path, &body_paint, tiny_skia::FillRule::Winding, t_scaled, None);
    }

    // 4. Draw Stripes (dark horizontal bands across the body)
    // Stripe 1
    let stripe1_rect = SkiaRect::from_ltrb(cx - 4.2, cy - 2.0, cx + 4.2, cy - 0.5)
        .ok_or_else(|| "Invalid stripe dimensions".to_string())?;
    let stripe1_path = PathBuilder::from_rect(stripe1_rect);
    pixmap.fill_path(&stripe1_path, &black_paint, tiny_skia::FillRule::Winding, global_transform, None);

    // Stripe 2
    let stripe2_rect = SkiaRect::from_ltrb(cx - 4.8, cy + 1.0, cx + 4.8, cy + 2.5)
        .ok_or_else(|| "Invalid stripe dimensions".to_string())?;
    let stripe2_path = PathBuilder::from_rect(stripe2_rect);
    pixmap.fill_path(&stripe2_path, &black_paint, tiny_skia::FillRule::Winding, global_transform, None);

    // Stripe 3
    let stripe3_rect = SkiaRect::from_ltrb(cx - 4.0, cy + 4.0, cx + 4.0, cy + 5.0)
        .ok_or_else(|| "Invalid stripe dimensions".to_string())?;
    let stripe3_path = PathBuilder::from_rect(stripe3_rect);
    pixmap.fill_path(&stripe3_path, &black_paint, tiny_skia::FillRule::Winding, global_transform, None);

    // 5. Draw Head (black circle)
    let head_path = {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy - 5.5, 3.2);
        pb.finish()
    };
    if let Some(ref path) = head_path {
        pixmap.fill_path(path, &black_paint, tiny_skia::FillRule::Winding, global_transform, None);
    }

    // 6. Draw Antennae (small black strokes)
    let antenna_left = {
        let mut pb = PathBuilder::new();
        pb.move_to(cx - 1.5, cy - 8.0);
        pb.quad_to(cx - 3.0, cy - 10.0, cx - 4.5, cy - 9.5);
        pb.finish()
    };
    let antenna_right = {
        let mut pb = PathBuilder::new();
        pb.move_to(cx + 1.5, cy - 8.0);
        pb.quad_to(cx + 3.0, cy - 10.0, cx + 4.5, cy - 9.5);
        pb.finish()
    };

    let stroke = Stroke {
        width: 1.0,
        ..Default::default()
    };

    if let Some(ref path) = antenna_left {
        pixmap.stroke_path(path, &black_paint, &stroke, global_transform, None);
    }
    if let Some(ref path) = antenna_right {
        pixmap.stroke_path(path, &black_paint, &stroke, global_transform, None);
    }

    Ok(())
}

/// Draws the border outline matching the accent color.
pub fn draw_border(
    canvas: &mut Canvas,
    accent_color: Color,
    border_thickness: f32,
    corner_radius: f32,
) -> Result<(), String> {
    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let pixmap = canvas.pixmap_mut();

    let half_t = border_thickness / 2.0;
    let l = half_t;
    let t = half_t;
    let r = width - half_t;
    let b = height - half_t;

    let path = {
        let mut pb = PathBuilder::new();
        let rad = corner_radius;
        
        pb.move_to(l + rad, t);
        pb.line_to(r - rad, t);
        pb.quad_to(r, t, r, t + rad);
        pb.line_to(r, b - rad);
        pb.quad_to(r, b, r - rad, b);
        pb.line_to(l + rad, b);
        pb.quad_to(l, b, l, b - rad);
        pb.line_to(l, t + rad);
        pb.quad_to(l, t, l + rad, t);
        pb.close();
        pb.finish()
    }.ok_or_else(|| "Failed to build rounded border path".to_string())?;

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
        draw_border(&mut canvas, accent, 2.0, 5.0).unwrap();

        let modified_sum: usize = canvas.pixels().iter().map(|&b| b as usize).sum();
        assert!(modified_sum > 0); // should fail because dummy does not draw
    }

    #[test]
    fn test_blit_pixmap() {
        let mut dest = Pixmap::new(10, 10).unwrap();
        dest.fill(Color::TRANSPARENT);
        
        let mut src = Pixmap::new(2, 2).unwrap();
        src.fill(Color::from_rgba8(255, 0, 0, 255)); // Solid Red
        
        blit_pixmap(&mut dest, &src, 4, 4);
        
        let idx_0_0 = 0;
        let idx_4_4 = ((4 * 10 + 4) * 4) as usize;
        
        assert_eq!(&dest.data()[idx_0_0..idx_0_0+4], &[0, 0, 0, 0]);
        assert_eq!(&dest.data()[idx_4_4..idx_4_4+4], &[255, 0, 0, 255]);
    }
}

/// Blits a source Pixmap onto a destination Pixmap at (dx, dy).
pub fn blit_pixmap(dest: &mut Pixmap, src: &Pixmap, dx: u32, dy: u32) {
    let dest_w = dest.width();
    let dest_h = dest.height();
    let src_w = src.width();
    let src_h = src.height();
    
    let dest_data = dest.data_mut();
    let src_data = src.data();
    
    for y in 0..src_h {
        let dest_y = dy + y;
        if dest_y >= dest_h {
            break;
        }
        for x in 0..src_w {
            let dest_x = dx + x;
            if dest_x >= dest_w {
                break;
            }
            
            let src_idx = ((y * src_w + x) * 4) as usize;
            let dest_idx = ((dest_y * dest_w + dest_x) * 4) as usize;
            
            let src_a = src_data[src_idx + 3] as f32 / 255.0;
            if src_a <= 0.0 {
                continue;
            }
            let factor = 1.0 - src_a;
            dest_data[dest_idx] = (src_data[src_idx] as f32 + dest_data[dest_idx] as f32 * factor).clamp(0.0, 255.0).round() as u8;
            dest_data[dest_idx + 1] = (src_data[src_idx + 1] as f32 + dest_data[dest_idx + 1] as f32 * factor).clamp(0.0, 255.0).round() as u8;
            dest_data[dest_idx + 2] = (src_data[src_idx + 2] as f32 + dest_data[dest_idx + 2] as f32 * factor).clamp(0.0, 255.0).round() as u8;
            dest_data[dest_idx + 3] = (src_data[src_idx + 3] as f32 + dest_data[dest_idx + 3] as f32 * factor).clamp(0.0, 255.0).round() as u8;
        }
    }
}

