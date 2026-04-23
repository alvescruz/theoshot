use crate::ui::types::{Shape, Tool};
use crate::ui::utils::get_arrow_points;
use eframe::egui;

pub fn draw_shape(
    painter: &egui::Painter,
    shape: &Shape,
    is_preview: bool,
    ctx: &egui::Context,
    is_selected: bool,
    is_hovered: bool,
) {
    let alpha = if is_preview { 160 } else { 255 };
    let color = egui::Color32::from_rgba_unmultiplied(
        shape.color.r(),
        shape.color.g(),
        shape.color.b(),
        alpha,
    );
    let stroke_width = if is_selected || is_hovered { 2.5 } else { 1.5 };
    let stroke = egui::Stroke::new(stroke_width, color);

    if is_selected || is_hovered {
        draw_selection_highlight(painter, shape, is_selected, stroke_width);
    }

    match shape.tool {
        Tool::Rectangle if shape.points.len() >= 2 => {
            painter.rect_stroke(
                egui::Rect::from_two_pos(shape.points[0], shape.points[1]),
                0.0,
                stroke,
            );
        }
        Tool::Circle if shape.points.len() >= 2 => {
            let center = shape.points[0];
            let radius = center.distance(shape.points[1]);
            painter.circle_stroke(center, radius, stroke);
        }
        Tool::Step if !shape.points.is_empty() => {
            draw_step(painter, shape);
        }
        Tool::Pen if shape.points.len() >= 2 => {
            painter.add(egui::Shape::line(shape.points.clone(), stroke));
        }
        Tool::Arrow if shape.points.len() >= 2 => {
            let pts = get_arrow_points(shape.points[0], shape.points[1]);
            for i in (0..pts.len()).step_by(2) {
                painter.line_segment([pts[i], pts[i + 1]], stroke);
            }
        }
        Tool::Blur if shape.points.len() >= 2 => {
            painter.rect_filled(
                egui::Rect::from_two_pos(shape.points[0], shape.points[1]),
                0.0,
                egui::Color32::from_black_alpha(140),
            );
        }
        Tool::Text if !shape.text.is_empty() || is_preview => {
            draw_text(painter, shape, is_preview, ctx);
        }
        _ => {}
    }
}

fn draw_selection_highlight(
    painter: &egui::Painter,
    shape: &Shape,
    is_selected: bool,
    stroke_width: f32,
) {
    let border_color = if is_selected {
        egui::Color32::from_white_alpha(200)
    } else {
        egui::Color32::from_white_alpha(100)
    };
    let border_stroke = egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_color);
    let expand = if is_selected { 6.0 } else { 4.0 };

    match shape.tool {
        Tool::Circle if shape.points.len() >= 2 => {
            let center = shape.points[0];
            let radius = center.distance(shape.points[1]);
            painter.circle_stroke(center, radius + expand, border_stroke);
        }
        Tool::Step if !shape.points.is_empty() => {
            painter.circle_stroke(shape.points[0], 15.0 + expand, border_stroke);
        }
        Tool::Arrow if shape.points.len() >= 2 => {
            let pts = get_arrow_points(shape.points[0], shape.points[1]);
            for i in (0..pts.len()).step_by(2) {
                painter.line_segment(
                    [pts[i], pts[i + 1]],
                    egui::Stroke::new(stroke_width + expand, border_color),
                );
            }
        }
        Tool::Pen if shape.points.len() >= 2 => {
            painter.add(egui::Shape::line(
                shape.points.clone(),
                egui::Stroke::new(stroke_width + expand, border_color),
            ));
        }
        _ => {
            let rect = match shape.tool {
                Tool::Rectangle | Tool::Blur => {
                    if shape.points.len() >= 2 {
                        Some(egui::Rect::from_two_pos(shape.points[0], shape.points[1]))
                    } else {
                        None
                    }
                }
                Tool::Text if !shape.points.is_empty() => Some(egui::Rect::from_min_size(
                    shape.points[0],
                    egui::vec2(100.0, 30.0),
                )),
                _ => None,
            };

            if let Some(r) = rect {
                painter.rect_stroke(r.expand(expand), 4.0, border_stroke);
            }
        }
    }
}

fn draw_step(painter: &egui::Painter, shape: &Shape) {
    let radius = 15.0;
    let center = shape.points[0];
    painter.circle_filled(center, radius, shape.color);
    let text_color =
        if shape.color.r() as u32 + shape.color.g() as u32 + shape.color.b() as u32 > 382 {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        shape.step_number.unwrap_or(0).to_string(),
        egui::FontId::proportional(16.0),
        text_color,
    );
}

fn draw_text(painter: &egui::Painter, shape: &Shape, is_preview: bool, ctx: &egui::Context) {
    let mut display_text = shape.text.clone();
    if is_preview && (ctx.input(|i| i.time) * 2.0) as i32 % 2 == 0 {
        display_text.push('|');
    }
    painter.text(
        shape.points[0],
        egui::Align2::LEFT_TOP,
        display_text,
        egui::FontId::proportional(22.0),
        shape.color,
    );
}
