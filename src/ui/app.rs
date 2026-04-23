use crate::ui::components::{action_button, styled_button};
use crate::ui::painter::draw_shape;
use crate::ui::renderer::render_to_image;
use crate::ui::types::{Shape, Tool};
use crate::ui::utils::get_canvas_rect;
use arboard::{Clipboard, ImageData};
use eframe::egui::{self};
use egui_phosphor::regular as icons;
use rfd::FileDialog;
use std::borrow::Cow;

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

    selected_shape_index: Option<usize>,
    hover_shape_index: Option<usize>,
    current_step: usize,
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
            stroke_color: egui::Color32::from_rgb(255, 0, 0),
            initialized_fonts: false,
            selected_shape_index: None,
            hover_shape_index: None,
            current_step: 1,
        }
    }

    fn setup_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);
        self.initialized_fonts = true;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context, canvas_rect: egui::Rect) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            std::process::exit(0);
        }

        // 1. Coleta as ações dentro do closure (sem tocar em self)
        let (save, copy, redo, undo) = ctx.input_mut(|input| {
            let save = input.consume_key(egui::Modifiers::COMMAND, egui::Key::S);

            // Copiar: Ctrl+C ou Evento nativo de Copy
            let copy = input.consume_key(egui::Modifiers::COMMAND, egui::Key::C)
                || input.events.iter().any(|e| matches!(e, egui::Event::Copy));

            // Redo: Ctrl+Shift+Z ou Ctrl+Y
            let redo = input.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            ) || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);

            // Undo: Ctrl+Z
            let undo = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);

            (save, copy, redo, undo)
        });

        // 2. Executa as ações fora do closure (self disponível)
        if save {
            self.save_action(canvas_rect);
        }

        if copy {
            self.copy_action(canvas_rect);
        }

        // Redo antes do Undo para evitar conflito no consumo de teclas
        if redo {
            if let Some(s) = self.redo_stack.pop() {
                self.shapes.push(s);
            }
        } else if undo {
            if let Some(s) = self.shapes.pop() {
                self.redo_stack.push(s);
            }
        }
    }

    fn save_action(&self, canvas_rect: egui::Rect) {
        if let Some(data) = &self.image_data {
            let img = render_to_image(&self.shapes, self.width, self.height, data, canvas_rect);
            if let Some(path) = FileDialog::new().set_file_name("shot.png").save_file() {
                if let Err(e) = img.save(path) {
                    eprintln!("[theoshot] Erro ao salvar imagem: {}", e);
                } else {
                    std::process::exit(0);
                }
            }
        }
    }

    fn copy_action(&self, canvas_rect: egui::Rect) {
        if let Some(data) = &self.image_data {
            let img = render_to_image(&self.shapes, self.width, self.height, data, canvas_rect);
            match Clipboard::new() {
                Ok(mut cb) => {
                    let (w, h) = img.dimensions();
                    let image_data = ImageData {
                        width: w as usize,
                        height: h as usize,
                        bytes: Cow::from(img.into_raw()),
                    };
                    if let Err(e) = cb.set_image(image_data) {
                        eprintln!("[theoshot] Erro ao copiar imagem: {}", e);
                    } else {
                        // Delay para garantir persistência no Linux antes de fechar o app
                        std::thread::sleep(std::time::Duration::from_millis(350));
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    eprintln!("[theoshot] Erro ao acessar clipboard: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    fn load_background_texture(&mut self, ctx: &egui::Context) {
        if self.texture.is_none() && self.image_data.is_some() {
            let data = self.image_data.as_ref().unwrap();
            let size = [self.width as usize, self.height as usize];

            self.texture = Some(ctx.load_texture(
                "bg",
                egui::ColorImage::from_rgba_unmultiplied(size, data),
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Linear,
                    minification: egui::TextureFilter::Linear,
                    ..Default::default()
                },
            ));
        }
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui, canvas_rect: egui::Rect) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let toolbar_width = 560.0;
            ui.add_space((canvas_rect.width() - toolbar_width).max(0.0) / 2.0);

            egui::Frame::default()
                .fill(egui::Color32::from_rgb(40, 40, 45))
                .rounding(egui::Rounding::same(14.0))
                .inner_margin(10.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(35)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);

                        // Seletor de cor
                        ui.vertical(|ui| {
                            ui.add_space(7.0); // Centralização vertical manual
                            ui.scope(|ui| {
                                ui.visuals_mut().widgets.inactive.rounding =
                                    egui::Rounding::same(11.0);
                                ui.visuals_mut().widgets.hovered.rounding =
                                    egui::Rounding::same(11.0);
                                ui.visuals_mut().widgets.active.rounding =
                                    egui::Rounding::same(11.0);
                                ui.spacing_mut().interact_size = egui::vec2(22.0, 22.0);
                                ui.color_edit_button_srgba(&mut self.stroke_color);
                            });
                        });

                        ui.separator();

                        let tools = [
                            (Tool::Rectangle, icons::RECTANGLE, "Retângulo"),
                            (Tool::Circle, icons::CIRCLE, "Círculo"),
                            (Tool::Step, icons::LIST_NUMBERS, "Etapas"),
                            (Tool::Pen, icons::PENCIL_SIMPLE, "Caneta"),
                            (Tool::Arrow, icons::ARROW_UP_RIGHT, "Seta"),
                            (Tool::Blur, icons::DROP, "Desfoque"),
                            (Tool::Text, icons::TEXT_T, "Texto"),
                            (Tool::Move, icons::CURSOR_CLICK, "Mover"),
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

                            if styled_button(
                                ui,
                                egui::RichText::new(icon).size(20.0).color(icon_color),
                                is_active,
                                self.stroke_color,
                            )
                            .on_hover_text(name)
                            .clicked()
                            {
                                self.current_tool = t;
                                if t != Tool::Move {
                                    self.selected_shape_index = None;
                                }
                            }
                        }

                        ui.separator();
                        if styled_button(
                            ui,
                            egui::RichText::new(icons::ARROW_U_UP_LEFT).size(18.0),
                            false,
                            self.stroke_color,
                        )
                        .on_hover_text("Desfazer (Ctrl+Z)")
                        .clicked()
                        {
                            if let Some(s) = self.shapes.pop() {
                                self.redo_stack.push(s);
                            }
                        }
                        if styled_button(
                            ui,
                            egui::RichText::new(icons::ARROW_U_UP_RIGHT).size(18.0),
                            false,
                            self.stroke_color,
                        )
                        .on_hover_text("Refazer (Ctrl+Shift+Z)")
                        .clicked()
                        {
                            if let Some(s) = self.redo_stack.pop() {
                                self.shapes.push(s);
                            }
                        }

                        if styled_button(
                            ui,
                            egui::RichText::new(icons::TRASH).size(18.0),
                            false,
                            self.stroke_color,
                        )
                        .on_hover_text("Limpar Tudo")
                        .clicked()
                        {
                            self.shapes.clear();
                            self.redo_stack.clear();
                            self.current_step = 1;
                            self.selected_shape_index = None;
                        }

                        ui.separator();
                        if styled_button(
                            ui,
                            egui::RichText::new(icons::COPY_SIMPLE).size(20.0),
                            false,
                            self.stroke_color,
                        )
                        .on_hover_text("Copiar (Ctrl+C)")
                        .clicked()
                        {
                            self.copy_action(canvas_rect);
                        }

                        if action_button(
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
    }

    fn handle_canvas_interactions(&mut self, ctx: &egui::Context, res: egui::Response) {
        let pos = res.interact_pointer_pos();

        // Hover e Cursor
        if self.current_tool == Tool::Move {
            if let Some(p) = pos {
                let mut nearest = None;
                let mut min_dist = 20.0;
                for (i, shape) in self.shapes.iter().enumerate() {
                    let bbox = shape.bounding_box();

                    // Se for uma forma com "corpo" (Retângulo, Círculo, Blur, Texto, Step),
                    // verificamos se o mouse está dentro da área.
                    let is_inside = match shape.tool {
                        Tool::Rectangle | Tool::Blur | Tool::Text | Tool::Step => bbox.contains(p),
                        Tool::Circle if shape.points.len() >= 2 => {
                            let center = shape.points[0];
                            let radius = center.distance(shape.points[1]);
                            p.distance(center) <= radius
                        }
                        _ => false,
                    };

                    if is_inside {
                        nearest = Some(i);
                        break; // Prioridade para o que está em cima
                    }

                    // Se não estiver dentro, ou for uma ferramenta de linha (Pen, Arrow),
                    // verificamos a proximidade dos pontos.
                    if bbox.expand(min_dist).contains(p) {
                        for pt in &shape.points {
                            let dist = p.distance(*pt);
                            if dist < min_dist {
                                min_dist = dist;
                                nearest = Some(i);
                            }
                        }
                    }
                }
                self.hover_shape_index = nearest;
                if self.hover_shape_index.is_some() {
                    ctx.set_cursor_icon(egui::CursorIcon::Grab);
                }
            } else {
                self.hover_shape_index = None;
            }
            if res.dragged() && self.selected_shape_index.is_some() {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }

        // Drag Start
        if res.drag_started() {
            if let Some(p) = pos {
                if self.current_tool == Tool::Move {
                    self.selected_shape_index = self.hover_shape_index;
                } else {
                    self.current_shape = Some(Shape {
                        tool: self.current_tool,
                        points: vec![p, p],
                        color: self.stroke_color,
                        _thickness: 3.0,
                        text: String::new(),
                        step_number: if self.current_tool == Tool::Step {
                            Some(self.current_step)
                        } else {
                            None
                        },
                    });
                }
            }
        }

        // Dragging
        if res.dragged() {
            if let Some(p) = pos {
                if self.current_tool == Tool::Move {
                    if let Some(idx) = self.selected_shape_index {
                        let delta = res.drag_delta();
                        if let Some(shape) = self.shapes.get_mut(idx) {
                            for pt in &mut shape.points {
                                *pt += delta;
                            }
                        }
                    }
                } else if let Some(shape) = &mut self.current_shape {
                    if shape.tool == Tool::Pen {
                        shape.points.push(p);
                    } else {
                        shape.points[1] = p;
                    }
                }
            }
        }

        // Drag Stop
        if res.drag_stopped() {
            if let Some(shape) = self.current_shape.take() {
                if shape.tool != Tool::Text {
                    if shape.tool == Tool::Step {
                        self.current_step += 1;
                    }
                    self.shapes.push(shape);
                    self.redo_stack.clear();
                } else {
                    self.current_shape = Some(shape);
                }
            }
        }

        // Text input handling
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
    }

    fn setup_visuals(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.window_fill = egui::Color32::TRANSPARENT;
        ctx.set_visuals(visuals);
    }
}

impl eframe::App for SelectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized_fonts {
            self.setup_fonts(ctx);
            self.setup_visuals(ctx); // 👈 mover pra cá
        }

        let canvas_rect = get_canvas_rect(ctx, self.width, self.height);
        self.load_background_texture(ctx);
        self.handle_shortcuts(ctx, canvas_rect);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |_ui| {
                egui::Area::new(egui::Id::new("main_view"))
                    .fixed_pos(canvas_rect.min)
                    .show(ctx, |ui| {
                        egui::Frame::default()
                            .fill(egui::Color32::from_rgb(30, 30, 35))
                            .rounding(egui::Rounding::ZERO)
                            .shadow(egui::Shadow {
                                blur: 40.0,
                                offset: egui::vec2(0.0, 20.0),
                                color: egui::Color32::from_black_alpha(200),
                                ..Default::default()
                            })
                            .stroke(egui::Stroke::new(1.5, egui::Color32::from_white_alpha(45)))
                            .show(ui, |ui| {
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

                                for (i, shape) in self.shapes.iter().enumerate() {
                                    draw_shape(
                                        &painter,
                                        shape,
                                        false,
                                        ctx,
                                        Some(i) == self.selected_shape_index,
                                        Some(i) == self.hover_shape_index,
                                    );
                                }
                                if let Some(shape) = &self.current_shape {
                                    draw_shape(&painter, shape, true, ctx, false, false);
                                }

                                self.handle_canvas_interactions(ctx, res);
                            });

                        self.render_toolbar(ui, canvas_rect);
                    });
            });
        ctx.request_repaint();
    }
}
