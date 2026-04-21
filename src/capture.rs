use ashpd::desktop::screenshot::{ScreenshotOptions, ScreenshotProxy};
use image::{GenericImageView, imageops};
use std::fs;
use std::io::Write;
use url::Url;
use device_query::{DeviceQuery, DeviceState};
use display_info::DisplayInfo;

pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub async fn capture_frame() -> Result<CapturedFrame, String> {
    let log_file = "/tmp/theoshot.log";
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .map_err(|e| e.to_string())?;

    let _ = writeln!(log, "--- Nova tentativa de captura inteligente: {:?} ---", std::time::SystemTime::now());

    // 1. Detecta posição do mouse imediatamente
    let device_state = DeviceState::new();
    let mouse_pos = device_state.get_mouse().coords;
    let _ = writeln!(log, "Posição do mouse detectada: {:?}", mouse_pos);

    // 2. Obtém informações de todos os monitores
    let displays = DisplayInfo::all().map_err(|e| e.to_string())?;
    
    // 3. Solicita captura da tela inteira (todos os monitores) sem interação
    let proxy = ScreenshotProxy::new().await.map_err(|e| format!("Proxy error: {}", e))?;
    let response = proxy
        .screenshot(None, ScreenshotOptions::default().set_interactive(false))
        .await
        .map_err(|e| format!("Screenshot request error: {}", e))?
        .response()
        .map_err(|e| format!("Portal response error: {}", e))?;

    let uri_str = response.uri().as_str();
    let url = Url::parse(uri_str).map_err(|e| format!("Url parse error: {}", e))?;
    let path = url.to_file_path().map_err(|_| format!("Invalid URI: {}", uri_str))?;

    let mut full_img = image::open(&path).map_err(|e| format!("Image open error: {}", e))?;
    let (full_w, full_h) = full_img.dimensions();
    let _ = writeln!(log, "Captura total recebida: {}x{}", full_w, full_h);

    // 4. Localiza o monitor que contém o mouse
    // Em Wayland/Portal, a captura total costuma ser um canvas gigante onde (0,0) é o topo-esquerda do monitor principal
    let target_display = displays.iter().find(|d| {
        let x = mouse_pos.0 as i32;
        let y = mouse_pos.1 as i32;
        x >= d.x && x < d.x + d.width as i32 && y >= d.y && y < d.y + d.height as i32
    }).or_else(|| displays.first()); // Fallback para o primeiro monitor se não achar o mouse

    let final_img = if let Some(d) = target_display {
        let _ = writeln!(log, "Monitor alvo identificado: {}x{} em ({}, {})", d.width, d.height, d.x, d.y);
        
        // Ajusta as coordenadas se o monitor estiver fora dos limites da imagem (por segurança)
        let cx = (d.x.max(0) as u32).min(full_w - 1);
        let cy = (d.y.max(0) as u32).min(full_h - 1);
        let cw = (d.width as u32).min(full_w - cx);
        let ch = (d.height as u32).min(full_h - cy);
        
        imageops::crop(&mut full_img, cx, cy, cw, ch).to_image()
    } else {
        full_img.to_rgba8()
    };

    let (width, height) = final_img.dimensions();
    let data = final_img.into_raw();

    let _ = fs::remove_file(&path);
    let _ = writeln!(log, "Recorte final concluído: {}x{}", width, height);

    Ok(CapturedFrame {
        width,
        height,
        data,
    })
}
