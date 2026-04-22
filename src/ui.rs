use ab_glyph::{FontVec, PxScale};
use arboard::{Clipboard, ImageData};
use eframe::egui;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use image::{Rgba, RgbaImage, imageops};
use imageproc::drawing::{draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect as ProcRect;
use rfd::FileDialog;
use std::borrow::Cow;

// Icons from Phosphor
use egui_phosphor::regular as icons;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
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
    _thickness: f32,
    text: String,
}

pub struct SelectionApp {
    pub image_data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,

    texture: Option<egui::TextureHandle>,
    current_tool: Tool,
    shapes: Vec<Shape>,
    redo_stack: Vec<Shape>,
    current_shape: Option<Shape>,

    stroke_color: egui::Color32,
    initialized_fonts: bool,
}

impl SelectionApp {
    pub fn new(image_data: Option<Vec<u8>>, width: u32, height: u32) -> Self {
        Self {
            image_data,
            width,
            height,
            texture: None,
            current_tool: Tool::Pen,
            shapes: Vec::new(),
            redo_stack: Vec::new(),
            current_shape: None,
            stroke_color: egui::Color32::from_rgb(0, 255, 127),
            initialized_fonts: false,
        }
    }

    fn setup_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);
        self.initialized_fonts = true;
    }

    fn get_canvas_rect(&self, ctx: &egui::Context) -> egui::Rect {
        let screen_rect = ctx.screen_rect();
        let max_w = screen_rect.width() * 0.9;
        let max_h = screen_rect.height() * 0.8;

        let img_aspect = self.width as f32 / self.height as f32;
        let (canvas_w, canvas_h) = if img_aspect > (max_w / max_h) {
            (max_w, max_w / img_aspect)
        } else {
            (max_h * img_aspect, max_h)
        };

        egui::Rect::from_center_size(
            screen_rect.center() - egui::vec2(0.0, 30.0),
            egui::vec2(canvas_w, canvas_h),
        )
    }

    fn point_to_pixel(&self, pos: egui::Pos2, canvas_rect: egui::Rect) -> (u32, u32) {
        let x_norm = (pos.x - canvas_rect.min.x) / canvas_rect.width();
        let y_norm = (pos.y - canvas_rect.min.y) / canvas_rect.height();
        let x = (x_norm * self.width as f32).clamp(0.0, self.width as f32 - 1.0) as u32;
        let y = (y_norm * self.height as f32).clamp(0.0, self.height as f32 - 1.0) as u32;
        (x, y)
    }

    fn draw_arrow_points(&self, start: egui::Pos2, end: egui::Pos2) -> Vec<egui::Pos2> {
        let head_length = 15.0;
        let head_angle = 0.4;
        let angle = (end.y - start.y).atan2(end.x - start.x);
        vec![
            start,
            end,
            end,
            egui::pos2(
                end.x - head_length * (angle - head_angle).cos(),
                end.y - head_length * (angle - head_angle).sin(),
            ),
            end,
            egui::pos2(
                end.x - head_length * (angle + head_angle).cos(),
                end.y - head_length * (angle + head_angle).sin(),
            ),
        ]
    }

    fn draw_shape_ui(
        &self,
        painter: &egui::Painter,
        shape: &Shape,
        is_preview: bool,
        _canvas_rect: egui::Rect,
        ctx: &egui::Context,
    ) {
        let alpha = if is_preview { 160 } else { 255 };
        let color = egui::Color32::from_rgba_unmultiplied(
            shape.color.r(),
            shape.color.g(),
            shape.color.b(),
            alpha,
        );
        let stroke = egui::Stroke::new(3.0, color);

        match shape.tool {
            Tool::Rectangle if shape.points.len() >= 2 => {
                painter.rect_stroke(
                    egui::Rect::from_two_pos(shape.points[0], shape.points[1]),
                    0.0,
                    stroke,
                );
            }
            Tool::Pen => {
                for i in 0..shape.points.len().saturating_sub(1) {
                    painter.line_segment([shape.points[i], shape.points[i + 1]], stroke);
                }
            }
            Tool::Arrow if shape.points.len() >= 2 => {
                let pts = self.draw_arrow_points(shape.points[0], shape.points[1]);
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
            _ => {}
        }
    }

    fn get_system_font(&self) -> Option<FontVec> {
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

    fn render_to_image(&self, canvas_rect: egui::Rect) -> RgbaImage {
        let mut img =
            RgbaImage::from_raw(self.width, self.height, self.image_data.clone().unwrap()).unwrap();
        let font = self.get_system_font();

        for shape in &self.shapes {
            let color = Rgba([
                shape.color.r(),
                shape.color.g(),
                shape.color.b(),
                shape.color.a(),
            ]);
            match shape.tool {
                Tool::Rectangle if shape.points.len() >= 2 => {
                    let (p1, p2) = (
                        self.point_to_pixel(shape.points[0], canvas_rect),
                        self.point_to_pixel(shape.points[1], canvas_rect),
                    );
                    draw_hollow_rect_mut(
                        &mut img,
                        ProcRect::at(p1.0.min(p2.0) as i32, p1.1.min(p2.1) as i32)
                            .of_size(p1.0.abs_diff(p2.0), p1.1.abs_diff(p2.1)),
                        color,
                    );
                }
                Tool::Pen => {
                    for i in 0..shape.points.len().saturating_sub(1) {
                        let (p1, p2) = (
                            self.point_to_pixel(shape.points[i], canvas_rect),
                            self.point_to_pixel(shape.points[i + 1], canvas_rect),
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
                    let pts = self.draw_arrow_points(shape.points[0], shape.points[1]);
                    for i in (0..pts.len()).step_by(2) {
                        let (p1, p2) = (
                            self.point_to_pixel(pts[i], canvas_rect),
                            self.point_to_pixel(pts[i + 1], canvas_rect),
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
                    let (p1, p2) = (
                        self.point_to_pixel(shape.points[0], canvas_rect),
                        self.point_to_pixel(shape.points[1], canvas_rect),
                    );
                    let (rx, ry, rw, rh) = (
                        p1.0.min(p2.0),
                        p1.1.min(p2.1),
                        p1.0.abs_diff(p2.0),
                        p1.1.abs_diff(p2.1),
                    );
                    if rw > 0 && rh > 0 {
                        let sub = imageops::crop_imm(&img, rx, ry, rw, rh).to_image();
                        // Intensidade de desfoque aumentada (divisor de 32 em vez de 16)
                        let block_size = 32;
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
                        let (x, y) = self.point_to_pixel(shape.points[0], canvas_rect);
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

    fn styled_button(
        &self,
        ui: &mut egui::Ui,
        text: impl Into<egui::WidgetText>,
        selected: bool,
    ) -> egui::Response {
        let text_widget: egui::WidgetText = text.into();
        let button = egui::Button::new(text_widget)
            .fill(if selected {
                self.stroke_color
            } else {
                egui::Color32::from_rgb(55, 55, 65)
            })
            .rounding(egui::Rounding::same(8.0))
            .min_size(egui::vec2(36.0, 36.0));

        ui.add(button)
    }

    fn action_button(
        &self,
        ui: &mut egui::Ui,
        text: impl Into<egui::WidgetText>,
        color: egui::Color32,
    ) -> egui::Response {
        let button = egui::Button::new(text)
            .fill(color)
            .rounding(egui::Rounding::same(8.0))
            .min_size(egui::vec2(36.0, 36.0));
        ui.add(button)
    }

    fn save_action(&self, canvas_rect: egui::Rect) {
        let img = self.render_to_image(canvas_rect);
        if let Some(path) = FileDialog::new().set_file_name("shot.png").save_file() {
            let _ = img.save(path);
        }
        std::process::exit(0);
    }

    fn copy_action(&self, canvas_rect: egui::Rect) {
        let img = self.render_to_image(canvas_rect);
        if let Ok(mut cb) = Clipboard::new() {
            let (w, h) = img.dimensions();
            let _ = cb.set_image(ImageData {
                width: w as usize,
                height: h as usize,
                bytes: Cow::from(img.into_raw()),
            });
        }
        std::process::exit(0);
    }
}

impl eframe::App for SelectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized_fonts {
            self.setup_fonts(ctx);
        }

        // Atalhos globais
        let input = ctx.input(|i| i.clone());
        if input.key_pressed(egui::Key::Escape) {
            std::process::exit(0);
        }

        let canvas_rect = self.get_canvas_rect(ctx);

        if input.modifiers.command && input.key_pressed(egui::Key::S) {
            self.save_action(canvas_rect);
        }
        if input.modifiers.command && input.key_pressed(egui::Key::C) {
            self.copy_action(canvas_rect);
        }

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

        // Transparência total do sistema
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.window_fill = egui::Color32::TRANSPARENT;
        ctx.set_visuals(visuals);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |_ui| {
                let window_frame = egui::Frame::default()
                    .fill(egui::Color32::from_rgb(30, 30, 35))
                    .rounding(egui::Rounding::ZERO)
                    .shadow(egui::Shadow {
                        blur: 40.0,
                        offset: egui::vec2(0.0, 20.0),
                        color: egui::Color32::from_black_alpha(200),
                        ..Default::default()
                    })
                    .stroke(egui::Stroke::new(1.5, egui::Color32::from_white_alpha(45)));

                // Janela e Toolbar em uma Area centralizada
                egui::Area::new(egui::Id::new("main_view"))
                    .fixed_pos(canvas_rect.min)
                    .show(ctx, |ui| {
                        // Frame da Imagem
                        window_frame.show(ui, |ui| {
                            ui.set_width(canvas_rect.width());
                            let (res, painter) =
                                ui.allocate_painter(canvas_rect.size(), egui::Sense::drag());
                            if let Some(tex) = &self.texture {
                                painter.image(
                                    tex.id(),
                                    canvas_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            }

                            for shape in &self.shapes {
                                self.draw_shape_ui(&painter, shape, false, canvas_rect, ctx);
                            }
                            if let Some(shape) = &self.current_shape {
                                self.draw_shape_ui(&painter, shape, true, canvas_rect, ctx);
                            }

                            let pos = res.interact_pointer_pos();
                            if res.drag_started() {
                                if let Some(p) = pos {
                                    self.current_shape = Some(Shape {
                                        tool: self.current_tool,
                                        points: vec![p, p],
                                        color: self.stroke_color,
                                        _thickness: 3.0,
                                        text: String::new(),
                                    });
                                }
                            }
                            if res.dragged() {
                                if let (Some(p), Some(shape)) = (pos, &mut self.current_shape) {
                                    if shape.tool == Tool::Pen {
                                        shape.points.push(p);
                                    } else {
                                        shape.points[1] = p;
                                    }
                                }
                            }
                            if res.drag_stopped() {
                                if let Some(shape) = self.current_shape.take() {
                                    if shape.tool != Tool::Text {
                                        self.shapes.push(shape);
                                        self.redo_stack.clear();
                                    } else {
                                        self.current_shape = Some(shape);
                                    }
                                }
                            }

                            if self.current_tool == Tool::Text {
                                if let Some(shape) = &mut self.current_shape {
                                    let mut finished = false;
                                    let events = ctx.input(|i| i.events.clone());
                                    for event in events {
                                        match event {
                                            egui::Event::Text(t) => shape.text.push_str(&t),
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
                                    if finished {
                                        if let Some(s) = self.current_shape.take() {
                                            self.shapes.push(s);
                                        }
                                    }
                                }
                            }
                        });

                        // Toolbar centralizada e compacta colada abaixo
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let toolbar_width = 390.0;
                            ui.add_space((canvas_rect.width() - toolbar_width).max(0.0) / 2.0);

                            egui::Frame::default()
                                .fill(egui::Color32::from_rgb(40, 40, 45))
                                .rounding(egui::Rounding::same(14.0))
                                .inner_margin(10.0)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(35)))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);

                                        // Seletor de cor circular e centralizado
                                        ui.scope(|ui| {
                                            ui.visuals_mut().widgets.inactive.rounding =
                                                egui::Rounding::same(18.0);
                                            ui.visuals_mut().widgets.hovered.rounding =
                                                egui::Rounding::same(18.0);
                                            ui.visuals_mut().widgets.active.rounding =
                                                egui::Rounding::same(18.0);
                                            ui.spacing_mut().interact_size = egui::vec2(30.0, 30.0);
                                            ui.color_edit_button_srgba(&mut self.stroke_color);
                                        });

                                        ui.separator();

                                        let tools = [
                                            (Tool::Rectangle, icons::RECTANGLE, "Retângulo"),
                                            (Tool::Pen, icons::PENCIL_SIMPLE, "Caneta"),
                                            (Tool::Arrow, icons::ARROW_UP_RIGHT, "Seta"),
                                            (Tool::Blur, icons::DROP, "Desfoque"),
                                            (Tool::Text, icons::TEXT_T, "Texto"),
                                        ];
                                        for (t, icon, name) in tools {
                                            let is_active = self.current_tool == t;
                                            let icon_color = if is_active {
                                                if self.stroke_color.r() as u32
                                                    + self.stroke_color.g() as u32
                                                    + self.stroke_color.b() as u32
                                                    > 382
                                                {
                                                    egui::Color32::BLACK
                                                } else {
                                                    egui::Color32::WHITE
                                                }
                                            } else {
                                                egui::Color32::WHITE
                                            };

                                            if self
                                                .styled_button(
                                                    ui,
                                                    egui::RichText::new(icon)
                                                        .size(20.0)
                                                        .color(icon_color),
                                                    is_active,
                                                )
                                                .on_hover_text(name)
                                                .clicked()
                                            {
                                                self.current_tool = t;
                                            }
                                        }

                                        ui.separator();
                                        if self
                                            .styled_button(
                                                ui,
                                                egui::RichText::new(icons::ARROW_U_UP_LEFT)
                                                    .size(18.0),
                                                false,
                                            )
                                            .on_hover_text("Desfazer")
                                            .clicked()
                                        {
                                            if let Some(s) = self.shapes.pop() {
                                                self.redo_stack.push(s);
                                            }
                                        }
                                        if self
                                            .styled_button(
                                                ui,
                                                egui::RichText::new(icons::ARROW_U_UP_RIGHT)
                                                    .size(18.0),
                                                false,
                                            )
                                            .on_hover_text("Refazer")
                                            .clicked()
                                        {
                                            if let Some(s) = self.redo_stack.pop() {
                                                self.shapes.push(s);
                                            }
                                        }

                                        ui.separator();
                                        if self
                                            .styled_button(
                                                ui,
                                                egui::RichText::new(icons::COPY_SIMPLE).size(20.0),
                                                false,
                                            )
                                            .on_hover_text("Copiar (Ctrl+C)")
                                            .clicked()
                                        {
                                            self.copy_action(canvas_rect);
                                        }

                                        if self
                                            .action_button(
                                                ui,
                                                egui::RichText::new(icons::FLOPPY_DISK)
                                                    .size(20.0)
                                                    .color(egui::Color32::WHITE),
                                                egui::Color32::from_rgb(60, 130, 255),
                                            )
                                            .on_hover_text("Salvar (Ctrl+S)")
                                            .clicked()
                                        {
                                            self.save_action(canvas_rect);
                                        }
                                    });
                                });
                        });
                    });
            });
        ctx.request_repaint();
    }
}

pub fn run_ui(image_data: Option<Vec<u8>>, width: u32, height: u32) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };
    eframe::run_native(
        "theoshot",
        options,
        Box::new(|_cc| Ok(Box::new(SelectionApp::new(image_data, width, height)))),
    )
    .unwrap();
}
