use ashpd::desktop::screenshot::{ScreenshotOptions, ScreenshotProxy};
use image::GenericImageView;
use std::fs;

pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub async fn capture_frame() -> Result<CapturedFrame, String> {
    println!("[capture] Solicitando screenshot via Portal...");

    let proxy = ScreenshotProxy::new().await.map_err(|e| e.to_string())?;

    let response = proxy
        .screenshot(None, ScreenshotOptions::default().set_interactive(false))
        .await
        .map_err(|e| e.to_string())?
        .response()
        .map_err(|e| e.to_string())?;

    // O Uri contém "file:///caminho/do/arquivo"
    let uri_str = response.uri().as_str();
    let path_str = uri_str.strip_prefix("file://").unwrap_or(uri_str);

    // Decodifica %20 e outros caracteres se necessário (simplificado aqui)
    let path = std::path::PathBuf::from(path_str);

    println!("[capture] Lendo arquivo: {:?}", path);

    let img = image::open(&path).map_err(|e| e.to_string())?;
    let (width, height) = img.dimensions();
    let data = img.to_rgba8().into_raw();

    let _ = fs::remove_file(&path);

    Ok(CapturedFrame {
        width,
        height,
        data,
    })
}
