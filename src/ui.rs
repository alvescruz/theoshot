use ab_glyph::{FontVec, PxScale};
use arboard::{Clipboard, ImageData};
use eframe::egui;
use image::{Rgba, RgbaImage, imageops};
use imageproc::drawing::{draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect as ProcRect;
use rfd::FileDialog;
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Select,
    SelectAll,
    Rectangle,
    Pen,
    Arrow,
    Blur,
    Text,
}

#[derive(Clone, Debug)]
struct Shape {
    tool: Tool,
    points: Vec<egui::Pos2>,
    color: egui::Color32,
    thickness: f32,
    text: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Handle {
    None,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Move,
}

pub struct SelectionApp {
    pub image_data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,

    texture: Option<egui::TextureHandle>,
    selection_start: Option<egui::Pos2>,
    selection_end: Option<egui::Pos2>,

    current_tool: Tool,
    shapes: Vec<Shape>,
    redo_stack: Vec<Shape>,
    current_shape: Option<Shape>,

    stroke_color: egui::Color32,
    _stroke_thickness: f32,
    active_handle: Handle,
}

impl SelectionApp {
    pub fn new(image_data: Option<Vec<u8>>, width: u32, height: u32) -> Self {
        Self {
            image_data,
            width,
            height,
            texture: None,
            selection_start: None,
            selection_end: None,
            current_tool: Tool::Select,
            shapes: Vec::new(),
            redo_stack: Vec::new(),
            current_shape: None,
            stroke_color: egui::Color32::from_rgb(0, 255, 127),
            _stroke_thickness: 3.0,
            active_handle: Handle::None,
        }
    }

    fn point_to_pixel(&self, pos: egui::Pos2, ctx: &egui::Context) -> (u32, u32) {
        let ppp = ctx.pixels_per_point();
        let x = (pos.x * ppp).clamp(0.0, self.width as f32 - 1.0) as u32;
        let y = (pos.y * ppp).clamp(0.0, self.height as f32 - 1.0) as u32;
        (x, y)
    }

    fn get_selection_rect(&self) -> Option<egui::Rect> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let r = egui::Rect::from_two_pos(start, end);
            if r.width() > 5.0 && r.height() > 5.0 {
                Some(r)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn select_all(&mut self, ctx: &egui::Context) {
        let ppp = ctx.pixels_per_point();
        self.selection_start = Some(egui::pos2(0.0, 0.0));
        self.selection_end = Some(egui::pos2(
            self.width as f32 / ppp,
            self.height as f32 / ppp,
        ));
    }

    fn draw_arrow_points(&self, start: egui::Pos2, end: egui::Pos2) -> Vec<egui::Pos2> {
        let head_length = 20.0;
        let head_angle = 0.5;
        let angle = (end.y - start.y).atan2(end.x - start.x);
        let p1 = egui::pos2(
            end.x - head_length * (angle - head_angle).cos(),
            end.y - head_length * (angle - head_angle).sin(),
        );
        let p2 = egui::pos2(
            end.x - head_length * (angle + head_angle).cos(),
            end.y - head_length * (angle + head_angle).sin(),
        );
        vec![start, end, p1, end, p2]
    }

    fn render_shapes_to_image(&self, ctx: &egui::Context) -> RgbaImage {
        let data = self
            .image_data
            .as_ref()
            .cloned()
            .unwrap_or_else(|| vec![0; (self.width * self.height * 4) as usize]);
        let mut img = RgbaImage::from_raw(self.width, self.height, data).expect("Buffer error");

        let font_data = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf")
            .ok()
            .or_else(|| std::fs::read("/usr/share/fonts/TTF/DejaVuSans-Bold.ttf").ok());
        let font = font_data.and_then(|d| FontVec::try_from_vec(d).ok());

        for shape in &self.shapes {
            let color = Rgba([
                shape.color.r(),
                shape.color.g(),
                shape.color.b(),
                shape.color.a(),
            ]);
            match shape.tool {
                Tool::Rectangle if shape.points.len() >= 2 => {
                    let (x1, y1) = self.point_to_pixel(shape.points[0], ctx);
                    let (x2, y2) = self.point_to_pixel(shape.points[1], ctx);
                    draw_hollow_rect_mut(
                        &mut img,
                        ProcRect::at(x1.min(x2) as i32, y1.min(y2) as i32)
                            .of_size(x1.abs_diff(x2), y1.abs_diff(y2)),
                        color,
                    );
                }
                Tool::Arrow if shape.points.len() >= 2 => {
                    let pts = self.draw_arrow_points(shape.points[0], shape.points[1]);
                    for i in (0..pts.len()).step_by(2).take(2) {
                        let p1 = self.point_to_pixel(pts[i], ctx);
                        let p2 = self.point_to_pixel(pts[i + 1], ctx);
                        draw_line_segment_mut(
                            &mut img,
                            (p1.0 as f32, p1.1 as f32),
                            (p2.0 as f32, p2.1 as f32),
                            color,
                        );
                    }
                }
                Tool::Pen => {
                    for i in 0..shape.points.len().saturating_sub(1) {
                        let p1 = self.point_to_pixel(shape.points[i], ctx);
                        let p2 = self.point_to_pixel(shape.points[i + 1], ctx);
                        draw_line_segment_mut(
                            &mut img,
                            (p1.0 as f32, p1.1 as f32),
                            (p2.0 as f32, p2.1 as f32),
                            color,
                        );
                    }
                }
                Tool::Blur if shape.points.len() >= 2 => {
                    let (x1, y1) = self.point_to_pixel(shape.points[0], ctx);
                    let (x2, y2) = self.point_to_pixel(shape.points[1], ctx);
                    let rx = x1.min(x2);
                    let ry = y1.min(y2);
                    let rw = x1.abs_diff(x2);
                    let rh = y1.abs_diff(y2);
                    if rw > 0 && rh > 0 {
                        let sub = imageops::crop_imm(&img, rx, ry, rw, rh).to_image();
                        // Aumentado para 48 para ocultamento absoluto
                        let block_size = 48;
                        let small = imageops::resize(
                            &sub,
                            (rw / block_size).max(1),
                            (rh / block_size).max(1),
                            imageops::FilterType::Nearest,
                        );
                        let pixelated =
                            imageops::resize(&small, rw, rh, imageops::FilterType::Nearest);
                        imageops::replace(&mut img, &pixelated, rx as i64, ry as i64);
                    }
                }
                Tool::Text if !shape.text.is_empty() => {
                    if let Some(ref f) = font {
                        let (x, y) = self.point_to_pixel(shape.points[0], ctx);
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

    fn save_file(&self, rect: egui::Rect, ctx: &egui::Context) {
        let mut img = self.render_shapes_to_image(ctx);
        let (x, y) = self.point_to_pixel(rect.min, ctx);
        let (x2, y2) = self.point_to_pixel(rect.max, ctx);
        if let Some(path) = FileDialog::new()
            .add_filter("PNG", &["png"])
            .set_file_name("theoshot.png")
            .save_file()
        {
            let cropped =
                image::imageops::crop(&mut img, x, y, x2.saturating_sub(x), y2.saturating_sub(y))
                    .to_image();
            let _ = cropped.save(path);
            std::process::exit(0);
        }
    }

    fn copy_clipboard(&self, rect: egui::Rect, ctx: &egui::Context) {
        let mut img = self.render_shapes_to_image(ctx);
        let (x, y) = self.point_to_pixel(rect.min, ctx);
        let (x2, y2) = self.point_to_pixel(rect.max, ctx);
        let cropped =
            image::imageops::crop(&mut img, x, y, x2.saturating_sub(x), y2.saturating_sub(y))
                .to_image();
        let (w, h) = (cropped.width(), cropped.height());
        let mut clipboard = Clipboard::new().unwrap();
        let data = ImageData {
            width: w as usize,
            height: h as usize,
            bytes: Cow::from(cropped.into_raw()),
        };
        let _ = clipboard.set_image(data);
        std::thread::sleep(std::time::Duration::from_millis(600));
        std::process::exit(0);
    }

    fn draw_shape_ui(
        &self,
        painter: &egui::Painter,
        shape: &Shape,
        is_current: bool,
        ctx: &egui::Context,
    ) {
        let stroke = egui::Stroke::new(shape.thickness, shape.color);
        match shape.tool {
            Tool::Rectangle if shape.points.len() >= 2 => {
                painter.rect_stroke(
                    egui::Rect::from_two_pos(shape.points[0], shape.points[1]),
                    0.0,
                    stroke,
                    egui::StrokeKind::Outside,
                );
            }
            Tool::Blur if shape.points.len() >= 2 => {
                let rect = egui::Rect::from_two_pos(shape.points[0], shape.points[1]);
                // Efeito Mosaic real na UI - Sem bordas e com blocos maiores
                let size = 48.0;
                let mut x = rect.min.x;
                while x < rect.max.x {
                    let mut y = rect.min.y;
                    while y < rect.max.y {
                        let cell =
                            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(size, size))
                                .intersect(rect);
                        painter.rect_filled(cell, 0.0, egui::Color32::from_black_alpha(240));
                        y += size;
                    }
                    x += size;
                }
            }
            Tool::Arrow if shape.points.len() >= 2 => {
                let pts = self.draw_arrow_points(shape.points[0], shape.points[1]);
                painter.line_segment([pts[0], pts[1]], stroke);
                painter.line_segment([pts[1], pts[2]], stroke);
                painter.line_segment([pts[1], pts[4]], stroke);
            }
            Tool::Pen => {
                for i in 0..shape.points.len().saturating_sub(1) {
                    painter.line_segment([shape.points[i], shape.points[i + 1]], stroke);
                }
            }
            Tool::Text => {
                let mut display_text = shape.text.clone();
                if is_current {
                    // Cursor piscante
                    if (ctx.input(|i| i.time) * 2.0) as i64 % 2 == 0 {
                        display_text.push('|');
                    }
                    // Destaque de edição
                    painter.rect_filled(
                        egui::Rect::from_two_pos(
                            shape.points[0],
                            shape.points[0] + egui::vec2(100.0, 24.0),
                        ),
                        4.0,
                        egui::Color32::from_white_alpha(20),
                    );
                }
                painter.text(
                    shape.points[0],
                    egui::Align2::LEFT_TOP,
                    display_text,
                    egui::FontId::proportional(24.0),
                    shape.color,
                );
            }
            _ => {}
        }
    }
}

impl eframe::App for SelectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.texture.is_none() && self.image_data.is_some() {
            let data = self.image_data.as_ref().unwrap();
            let pixels = data
                .chunks_exact(4)
                .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
                .collect();
            self.texture = Some(ctx.load_texture(
                "bg",
                egui::ColorImage {
                    size: [self.width as usize, self.height as usize],
                    pixels,
                },
                egui::TextureOptions::default(),
            ));
        }

        let ppp = ctx.pixels_per_point();
        let full_rect = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(self.width as f32 / ppp, self.height as f32 / ppp),
        );
        let sel_rect = self.get_selection_rect();

        // Atalhos globais
        let ctrl_c = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::C)
                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::C)
        });
        let ctrl_a = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::A)
                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)
        });
        let esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

        if esc {
            std::process::exit(0);
        }
        if ctrl_a {
            self.select_all(ctx);
        }
        if ctrl_c {
            let sel = self.get_selection_rect().unwrap_or(full_rect);
            self.copy_clipboard(sel, ctx);
            std::process::exit(0);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();
                let sel_rect = self.get_selection_rect();

                if let Some(tex) = &self.texture {
                    painter.image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                let overlay_color = egui::Color32::from_rgba_unmultiplied(10, 15, 25, 160);
                if let Some(sel) = sel_rect {
                    painter.rect_filled(
                        egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, sel.min.y)),
                        0.0,
                        overlay_color,
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(rect.min.x, sel.max.y), rect.max),
                        0.0,
                        overlay_color,
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.min.x, sel.min.y),
                            egui::pos2(sel.min.x, sel.max.y),
                        ),
                        0.0,
                        overlay_color,
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(sel.max.x, sel.min.y),
                            egui::pos2(rect.max.x, sel.max.y),
                        ),
                        0.0,
                        overlay_color,
                    );
                    painter.rect_stroke(
                        sel,
                        0.0,
                        egui::Stroke::new(2.5, egui::Color32::from_white_alpha(200)),
                        egui::StrokeKind::Outside,
                    );

                    let mouse_pos =
                        ctx.input(|i| i.pointer.interact_pos().unwrap_or(egui::Pos2::ZERO));
                    let handles = [
                        (
                            sel.left_top(),
                            Handle::TopLeft,
                            egui::CursorIcon::ResizeNwSe,
                        ),
                        (
                            sel.right_top(),
                            Handle::TopRight,
                            egui::CursorIcon::ResizeNeSw,
                        ),
                        (
                            sel.left_bottom(),
                            Handle::BottomLeft,
                            egui::CursorIcon::ResizeNeSw,
                        ),
                        (
                            sel.right_bottom(),
                            Handle::BottomRight,
                            egui::CursorIcon::ResizeNwSe,
                        ),
                    ];
                    for (pos, _handle, icon) in handles {
                        let dist = (mouse_pos - pos).length();
                        let is_hovered = dist < 12.0;
                        let size = if is_hovered { 7.0 } else { 5.0 };
                        let color = if is_hovered {
                            egui::Color32::from_rgb(0, 255, 127)
                        } else {
                            egui::Color32::WHITE
                        };
                        painter.circle_filled(pos, size, color);
                        if is_hovered {
                            ctx.set_cursor_icon(icon);
                        }
                    }
                    if sel.contains(mouse_pos)
                        && self.current_tool == Tool::Select
                        && self.active_handle == Handle::None
                    {
                        ctx.set_cursor_icon(egui::CursorIcon::Grab);
                    }
                } else {
                    painter.rect_filled(rect, 0.0, overlay_color);
                }

                for shape in &self.shapes {
                    self.draw_shape_ui(painter, shape, false, ctx);
                }
                if let Some(shape) = &self.current_shape {
                    self.draw_shape_ui(painter, shape, true, ctx);
                }

                if self.current_tool == Tool::Text {
                    let mut finished = false;
                    if let Some(shape) = &mut self.current_shape {
                        let events = ctx.input(|i| i.events.clone());
                        for event in events {
                            match event {
                                egui::Event::Text(ref t) => shape.text.push_str(t),
                                egui::Event::Key {
                                    key: egui::Key::Backspace,
                                    pressed: true,
                                    ..
                                } => {
                                    shape.text.pop();
                                }
                                egui::Event::Key {
                                    key: egui::Key::Enter,
                                    pressed: true,
                                    ..
                                } => {
                                    finished = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    if finished {
                        if let Some(shape) = self.current_shape.take() {
                            self.shapes.push(shape);
                        }
                    }
                }

                let response = ui.interact(rect, ui.id(), egui::Sense::drag());
                let pos = response.interact_pointer_pos();

                if response.drag_started() {
                    if let (Some(p), Some(sel)) = (pos, sel_rect) {
                        if (p - sel.left_top()).length() < 15.0 {
                            self.active_handle = Handle::TopLeft;
                        } else if (p - sel.right_top()).length() < 15.0 {
                            self.active_handle = Handle::TopRight;
                        } else if (p - sel.left_bottom()).length() < 15.0 {
                            self.active_handle = Handle::BottomLeft;
                        } else if (p - sel.right_bottom()).length() < 15.0 {
                            self.active_handle = Handle::BottomRight;
                        } else if sel.contains(p) && self.current_tool == Tool::Select {
                            self.active_handle = Handle::Move;
                        } else {
                            self.active_handle = Handle::None;
                        }
                    }
                    if self.active_handle == Handle::None {
                        if self.current_tool == Tool::Select {
                            self.selection_start = pos;
                            self.selection_end = pos;
                        } else if let Some(p) = pos {
                            self.current_shape = Some(Shape {
                                tool: self.current_tool,
                                points: vec![p, p],
                                color: self.stroke_color,
                                thickness: 3.0,
                                text: String::new(),
                            });
                        }
                    }
                }

                if response.dragged() {
                    if let Some(p) = pos {
                        match self.active_handle {
                            Handle::TopLeft => self.selection_start = Some(p),
                            Handle::BottomRight => self.selection_end = Some(p),
                            Handle::TopRight => {
                                self.selection_start =
                                    Some(egui::pos2(self.selection_start.unwrap().x, p.y));
                                self.selection_end =
                                    Some(egui::pos2(p.x, self.selection_end.unwrap().y));
                            }
                            Handle::BottomLeft => {
                                self.selection_start =
                                    Some(egui::pos2(p.x, self.selection_start.unwrap().y));
                                self.selection_end =
                                    Some(egui::pos2(self.selection_end.unwrap().x, p.y));
                            }
                            Handle::Move => {
                                let delta = response.drag_delta();
                                self.selection_start = Some(self.selection_start.unwrap() + delta);
                                self.selection_end = Some(self.selection_end.unwrap() + delta);
                            }
                            Handle::None => {
                                if self.current_tool == Tool::Select {
                                    self.selection_end = Some(p);
                                } else if let Some(shape) = &mut self.current_shape {
                                    if shape.tool == Tool::Pen {
                                        shape.points.push(p);
                                    } else {
                                        shape.points[1] = p;
                                    }
                                }
                            }
                        }
                    }
                }

                if response.drag_stopped() {
                    if let Some(shape) = self.current_shape.take() {
                        if shape.tool != Tool::Text {
                            self.shapes.push(shape);
                            self.redo_stack.clear();
                        } else {
                            self.current_shape = Some(shape);
                        }
                    }
                    self.active_handle = Handle::None;
                }
            });

        let toolbar_pos = if let Some(sel) = sel_rect {
            sel.max + egui::vec2(-450.0, 20.0)
        } else {
            egui::pos2(full_rect.center().x - 225.0, full_rect.max.y - 80.0)
        };

        egui::Area::new(egui::Id::new("toolbar"))
            .fixed_pos(toolbar_pos)
            .show(ctx, |ui| {
                let shadow = egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 16,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(100),
                };
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 25, 235))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(35)))
                    .corner_radius(egui::CornerRadius::same(20))
                    .shadow(shadow)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(14.0, 0.0);
                            ui.color_edit_button_srgba(&mut self.stroke_color);
                            ui.separator();
                            let tools = [
                                (Tool::Select, "⛶", "Seleção"),
                                (Tool::SelectAll, "🖥", "Selecionar Tudo"),
                                (Tool::Rectangle, "▢", "Retângulo"),
                                (Tool::Pen, "🖊", "Caneta"),
                                (Tool::Arrow, "↗", "Seta"),
                                (Tool::Blur, "💧", "Desfoque"),
                                (Tool::Text, "A", "Texto"),
                            ];
                            for (t, icon, name) in tools {
                                let is_active = self.current_tool == t;
                                if ui
                                    .selectable_label(
                                        is_active,
                                        egui::RichText::new(icon).size(20.0).color(if is_active {
                                            egui::Color32::WHITE
                                        } else {
                                            egui::Color32::from_white_alpha(140)
                                        }),
                                    )
                                    .on_hover_text(name)
                                    .clicked()
                                {
                                    if t == Tool::SelectAll {
                                        self.select_all(ctx);
                                        self.current_tool = Tool::Select;
                                    } else {
                                        self.current_tool = t;
                                    }
                                }
                            }
                            ui.separator();
                            if ui.button(egui::RichText::new("⟲").size(18.0)).clicked() {
                                if let Some(s) = self.shapes.pop() {
                                    self.redo_stack.push(s);
                                }
                            }
                            if ui.button(egui::RichText::new("⟳").size(18.0)).clicked() {
                                if let Some(s) = self.redo_stack.pop() {
                                    self.shapes.push(s);
                                }
                            }
                            ui.separator();
                            if ui.button(egui::RichText::new("📋").size(20.0)).clicked() {
                                self.copy_clipboard(sel_rect.unwrap_or(full_rect), ctx);
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("💾")
                                            .size(20.0)
                                            .color(egui::Color32::WHITE),
                                    )
                                    .corner_radius(egui::CornerRadius::same(10))
                                    .fill(egui::Color32::from_rgba_unmultiplied(60, 130, 255, 180)),
                                )
                                .clicked()
                            {
                                self.save_file(sel_rect.unwrap_or(full_rect), ctx);
                            }
                        });
                    });
            });
        ctx.request_repaint();
    }
}

pub fn run_ui(image_data: Option<Vec<u8>>, width: u32, height: u32) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "theoshot",
        options,
        Box::new(|_cc| Ok(Box::new(SelectionApp::new(image_data, width, height)))),
    );
}
