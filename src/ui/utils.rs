use eframe::egui;

pub fn get_arrow_points(start: egui::Pos2, end: egui::Pos2) -> Vec<egui::Pos2> {
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

pub fn point_to_pixel(
    pos: egui::Pos2,
    canvas_rect: egui::Rect,
    img_w: u32,
    img_h: u32,
) -> (u32, u32) {
    let x_norm = (pos.x - canvas_rect.min.x) / canvas_rect.width();
    let y_norm = (pos.y - canvas_rect.min.y) / canvas_rect.height();
    let x = (x_norm * img_w as f32)
        .round()
        .clamp(0.0, img_w as f32 - 1.0) as u32;
    let y = (y_norm * img_h as f32)
        .round()
        .clamp(0.0, img_h as f32 - 1.0) as u32;
    (x, y)
}

pub fn get_canvas_rect(ctx: &egui::Context, img_w: u32, img_h: u32) -> egui::Rect {
    let screen_rect = ctx.screen_rect();
    let ppp = ctx.pixels_per_point();

    // Deixamos um pouco mais de espaço para a barra de ferramentas
    let max_w = screen_rect.width() * 0.95;
    let max_h = screen_rect.height() * 0.85;

    let img_w_pts = img_w as f32 / ppp;
    let img_h_pts = img_h as f32 / ppp;

    let (canvas_w, canvas_h) = if img_w_pts <= max_w && img_h_pts <= max_h {
        // Se couber no tamanho nativo, usamos o tamanho nativo para evitar interpolação
        (img_w_pts, img_h_pts)
    } else {
        let img_aspect = img_w as f32 / img_h as f32;
        if img_aspect > (max_w / max_h) {
            (max_w, max_w / img_aspect)
        } else {
            (max_h * img_aspect, max_h)
        }
    };

    let rect = egui::Rect::from_center_size(
        screen_rect.center() - egui::vec2(0.0, 40.0),
        egui::vec2(canvas_w, canvas_h),
    );

    // Arredonda para pixels físicos manualmente para garantir nitidez
    let min = egui::pos2(
        (rect.min.x * ppp).round() / ppp,
        (rect.min.y * ppp).round() / ppp,
    );
    let max = egui::pos2(
        (rect.max.x * ppp).round() / ppp,
        (rect.max.y * ppp).round() / ppp,
    );

    egui::Rect::from_min_max(min, max)
}
