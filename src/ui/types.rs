use eframe::egui;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Rectangle,
    Circle,
    Step,
    Pen,
    Arrow,
    Blur,
    Text,
    Move,
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub tool: Tool,
    pub points: Vec<egui::Pos2>,
    pub color: egui::Color32,
    pub _thickness: f32,
    pub text: String,
    pub step_number: Option<usize>,
}

impl Shape {
    pub fn bounding_box(&self) -> egui::Rect {
        if self.points.is_empty() {
            return egui::Rect::NOTHING;
        }

        let mut rect = egui::Rect::from_two_pos(self.points[0], self.points[0]);
        for &p in &self.points[1..] {
            rect.extend_with(p);
        }

        // Expandir um pouco para ferramentas como Círculo ou Texto que podem
        // ter extensões visuais além dos pontos de controle principais.
        match self.tool {
            Tool::Circle if self.points.len() >= 2 => {
                let center = self.points[0];
                let radius = center.distance(self.points[1]);
                rect = egui::Rect::from_center_size(center, egui::vec2(radius * 2.0, radius * 2.0));
            }
            Tool::Text => {
                rect = rect.expand(10.0);
            }
            _ => {
                rect = rect.expand(2.0);
            }
        }
        rect
    }
}
