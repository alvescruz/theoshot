use eframe::egui;
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;

pub struct SelectionApp {
    pub image_data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,

    texture: Option<egui::TextureHandle>,
    selection_start: Option<egui::Pos2>,
    selection_end: Option<egui::Pos2>,
    is_dragging: bool,
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
            is_dragging: false,
        }
    }

    fn get_selection_rect(&self) -> Option<egui::Rect> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            Some(egui::Rect::from_two_pos(start, end))
        } else {
            None
        }
    }

    fn create_color_image(&self) -> Option<egui::ColorImage> {
        let data = self.image_data.as_ref()?;
        let mut pixels = Vec::with_capacity((self.width * self.height) as usize);

        for chunk in data.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, 255));
        }

        if pixels.len() == (self.width * self.height) as usize {
            Some(egui::ColorImage {
                size: [self.width as usize, self.height as usize],
                pixels,
            })
        } else {
            None
        }
    }

    fn copy_selection_to_clipboard(&self, rect: egui::Rect) {
        let Some(data) = &self.image_data else { return };

        let x_start = rect.min.x.max(0.0) as u32;
        let y_start = rect.min.y.max(0.0) as u32;
        let w = rect.width() as u32;
        let h = rect.height() as u32;

        let mut cropped_pixels = Vec::with_capacity((w * h * 4) as usize);
        for y in y_start..(y_start + h) {
            for x in x_start..(x_start + w) {
                if x < self.width && y < self.height {
                    let idx = ((y * self.width + x) * 4) as usize;
                    if idx + 2 < data.len() {
                        let b = data[idx];
                        let g = data[idx + 1];
                        let r = data[idx + 2];
                        cropped_pixels.push(r);
                        cropped_pixels.push(g);
                        cropped_pixels.push(b);
                        cropped_pixels.push(255);
                    }
                }
            }
        }

        let mut clipboard = Clipboard::new().unwrap();
        let image = ImageData {
            width: w as usize,
            height: h as usize,
            bytes: Cow::from(cropped_pixels),
        };
        let _ = clipboard.set_image(image);
    }
}

impl eframe::App for SelectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.texture.is_none() && self.image_data.is_some() {
            if let Some(color_image) = self.create_color_image() {
                self.texture = Some(ctx.load_texture("screen-capture", color_image, Default::default()));
            }
        }

        let panel_frame = egui::Frame::NONE.fill(egui::Color32::TRANSPARENT);

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            let sel_rect = self.get_selection_rect();

            // 1. Imagem de fundo
            if let Some(texture) = &self.texture {
                painter.image(texture.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }

            // 2. Overlay (Opacidade reduzida para 80)
            let overlay_color = egui::Color32::from_black_alpha(80);
            if let Some(sel) = sel_rect {
                painter.rect_filled(egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, sel.min.y)), 0.0, overlay_color);
                painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x, sel.max.y), rect.max), 0.0, overlay_color);
                painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x, sel.min.y), egui::pos2(sel.min.x, sel.max.y)), 0.0, overlay_color);
                painter.rect_filled(egui::Rect::from_min_max(egui::pos2(sel.max.x, sel.min.y), egui::pos2(rect.max.x, sel.max.y)), 0.0, overlay_color);
                
                // Borda nítida
                painter.rect_stroke(sel, 0.0, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
                
                // Botão
                let btn_rect = egui::Rect::from_min_size(sel.max - egui::vec2(80.0, 40.0), egui::vec2(70.0, 30.0));
                if ui.put(btn_rect, egui::Button::new("Copiar")).clicked() {
                    self.copy_selection_to_clipboard(sel);
                    std::process::exit(0);
                }
            } else {
                painter.rect_filled(rect, 0.0, overlay_color);
            }

            // 3. Mouse
            let response = ui.interact(rect, ui.id(), egui::Sense::drag());
            if response.drag_started() {
                self.selection_start = response.interact_pointer_pos();
                self.is_dragging = true;
            }
            if self.is_dragging {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.selection_end = Some(pos);
                }
            }
            if response.drag_stopped() {
                self.is_dragging = false;
            }
            
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                std::process::exit(0);
            }
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
    let _ = eframe::run_native("theoshot", options, Box::new(|_cc| Ok(Box::new(SelectionApp::new(image_data, width, height)))));
}
