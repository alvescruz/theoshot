use crate::ui::types::{Shape, Tool};
use crate::ui::utils::{get_arrow_points, point_to_pixel};
use ab_glyph::{FontVec, PxScale};
use eframe::egui;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use image::{Rgba, RgbaImage, imageops};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_hollow_circle_mut, draw_hollow_rect_mut, draw_line_segment_mut,
    draw_text_mut,
};
use imageproc::rect::Rect as ProcRect;

pub fn render_to_image(
    shapes: &[Shape],
    width: u32,
    height: u32,
    image_data: &[u8],
    canvas_rect: egui::Rect,
) -> RgbaImage {
    let mut img = RgbaImage::from_raw(width, height, image_data.to_vec()).unwrap();
    let font = get_system_font();

    for shape in shapes {
        let color = Rgba([
            shape.color.r(),
            shape.color.g(),
            shape.color.b(),
            shape.color.a(),
        ]);

        match shape.tool {
            Tool::Rectangle if shape.points.len() >= 2 => {
                let (p1, p2) = (
                    point_to_pixel(shape.points[0], canvas_rect, width, height),
                    point_to_pixel(shape.points[1], canvas_rect, width, height),
                );
                draw_hollow_rect_mut(
                    &mut img,
                    ProcRect::at(p1.0.min(p2.0) as i32, p1.1.min(p2.1) as i32)
                        .of_size(p1.0.abs_diff(p2.0), p1.1.abs_diff(p2.1)),
                    color,
                );
            }
            Tool::Circle if shape.points.len() >= 2 => {
                let (p1, p2) = (
                    point_to_pixel(shape.points[0], canvas_rect, width, height),
                    point_to_pixel(shape.points[1], canvas_rect, width, height),
                );
                let center_x = p1.0 as i32;
                let center_y = p1.1 as i32;
                let radius = (((p1.0 as f32 - p2.0 as f32).powi(2)
                    + (p1.1 as f32 - p2.1 as f32).powi(2))
                .sqrt()) as i32;
                if radius > 0 {
                    draw_hollow_circle_mut(&mut img, (center_x, center_y), radius, color);
                }
            }
            Tool::Step if !shape.points.is_empty() => {
                render_step(&mut img, shape, &font, width, height, canvas_rect, color);
            }
            Tool::Pen => {
                for i in 0..shape.points.len().saturating_sub(1) {
                    let (p1, p2) = (
                        point_to_pixel(shape.points[i], canvas_rect, width, height),
                        point_to_pixel(shape.points[i + 1], canvas_rect, width, height),
                    );
                    draw_line_segment_mut(
                        &mut img,
                        (p1.0 as f32, p1.1 as f32),
                        (p2.0 as f32, p2.1 as f32),
                        color,
                    );
                }
            }
            Tool::Arrow if shape.points.len() >= 2 => {
                let pts = get_arrow_points(shape.points[0], shape.points[1]);
                for i in (0..pts.len()).step_by(2) {
                    let (p1, p2) = (
                        point_to_pixel(pts[i], canvas_rect, width, height),
                        point_to_pixel(pts[i + 1], canvas_rect, width, height),
                    );
                    draw_line_segment_mut(
                        &mut img,
                        (p1.0 as f32, p1.1 as f32),
                        (p2.0 as f32, p2.1 as f32),
                        color,
                    );
                }
            }
            Tool::Blur if shape.points.len() >= 2 => {
                render_blur(&mut img, shape, canvas_rect, width, height);
            }
            Tool::Text if !shape.text.is_empty() => {
                if let Some(ref f) = font {
                    let (x, y) = point_to_pixel(shape.points[0], canvas_rect, width, height);
                    draw_text_mut(
                        &mut img,
                        color,
                        x as i32,
                        y as i32,
                        PxScale::from(32.0),
                        f,
                        &shape.text,
                    );
                }
            }
            _ => {}
        }
    }
    img
}

fn render_step(
    img: &mut RgbaImage,
    shape: &Shape,
    font: &Option<FontVec>,
    img_w: u32,
    img_h: u32,
    canvas_rect: egui::Rect,
    color: Rgba<u8>,
) {
    let (px, py) = point_to_pixel(shape.points[0], canvas_rect, img_w, img_h);
    let base_radius = 15.0;
    let scaled_radius = (base_radius * (img_w as f32 / canvas_rect.width())) as i32;

    draw_filled_circle_mut(img, (px as i32, py as i32), scaled_radius, color);
    if let (Some(f), Some(num)) = (font, shape.step_number) {
        let text_color =
            if shape.color.r() as u32 + shape.color.g() as u32 + shape.color.b() as u32 > 382 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            };
        let text = num.to_string();
        let text_scale = scaled_radius as f32 * 1.2;
        let tx = px as i32 - (text_scale * 0.3) as i32;
        let ty = py as i32 - (text_scale * 0.5) as i32;
        draw_text_mut(img, text_color, tx, ty, PxScale::from(text_scale), f, &text);
    }
}

fn render_blur(
    img: &mut RgbaImage,
    shape: &Shape,
    canvas_rect: egui::Rect,
    img_w: u32,
    img_h: u32,
) {
    let (p1, p2) = (
        point_to_pixel(shape.points[0], canvas_rect, img_w, img_h),
        point_to_pixel(shape.points[1], canvas_rect, img_w, img_h),
    );
    let (rx, ry, rw, rh) = (
        p1.0.min(p2.0),
        p1.1.min(p2.1),
        p1.0.abs_diff(p2.0),
        p1.1.abs_diff(p2.1),
    );
    if rw > 0 && rh > 0 {
        let sub = imageops::crop_imm(img, rx, ry, rw, rh).to_image();
        let block_size = 64;
        let small = imageops::resize(
            &sub,
            (rw / block_size).max(1),
            (rh / block_size).max(1),
            imageops::FilterType::Nearest,
        );
        let pixelated = imageops::resize(&small, rw, rh, imageops::FilterType::Nearest);
        imageops::replace(img, &pixelated, rx as i64, ry as i64);
    }
}

pub fn get_system_font() -> Option<FontVec> {
    let source = SystemSource::new();
    let families = [
        FamilyName::SansSerif,
        FamilyName::Serif,
        FamilyName::Monospace,
    ];

    for family in families {
        if let Ok(handle) = source.select_best_match(&[family], &Properties::new()) {
            if let Ok(font) = handle.load() {
                if let Some(data) = font.copy_font_data() {
                    if let Ok(font_vec) = FontVec::try_from_vec((*data).clone()) {
                        return Some(font_vec);
                    }
                }
            }
        }
    }
    None
}
