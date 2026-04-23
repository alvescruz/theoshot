use eframe::egui;

pub fn styled_button(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    selected: bool,
    stroke_color: egui::Color32,
) -> egui::Response {
    let text_widget: egui::WidgetText = text.into();
    let button = egui::Button::new(text_widget)
        .fill(if selected {
            stroke_color
        } else {
            egui::Color32::from_rgb(55, 55, 65)
        })
        .rounding(egui::Rounding::same(8.0))
        .min_size(egui::vec2(36.0, 36.0));

    ui.add(button)
}

pub fn action_button(
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
