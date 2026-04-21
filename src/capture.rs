use ashpd::desktop::screenshot::{ScreenshotOptions, ScreenshotProxy};
use image::GenericImageView;
use std::fs;
use std::io::Write;
use url::Url;

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

    let _ = writeln!(
        log,
        "--- Nova tentativa: {:?} ---",
        std::time::SystemTime::now()
    );

    let proxy = ScreenshotProxy::new().await.map_err(|e| {
        let _ = writeln!(log, "Proxy error: {}", e);
        format!("Proxy error: {}", e)
    })?;

    let _ = writeln!(log, "Proxy OK. Solicitando screenshot...");

    let response = proxy
        .screenshot(None, ScreenshotOptions::default().set_interactive(false))
        .await
        .map_err(|e| {
            let _ = writeln!(log, "Screenshot request error: {}", e);
            format!("Screenshot request error: {}", e)
        })?
        .response()
        .map_err(|e| {
            let _ = writeln!(log, "Portal response error: {}", e);
            format!("Portal response error: {}", e)
        })?;

    let uri_str = response.uri().as_str();
    let url = Url::parse(uri_str).map_err(|e| {
        let _ = writeln!(log, "Url parse error: {}", e);
        format!("Url parse error: {}", e)
    })?;

    let path = url.to_file_path().map_err(|_| {
        let _ = writeln!(log, "Invalid URI: {}", uri_str);
        format!("Invalid URI: {}", uri_str)
    })?;

    let _ = writeln!(log, "Arquivo recebido: {:?}", path);

    let img = image::open(&path).map_err(|e| {
        let _ = writeln!(log, "Image open error: {}", e);
        format!("Image open error: {}", e)
    })?;

    let (width, height) = img.dimensions();
    let data = img.to_rgba8().into_raw();

    let _ = fs::remove_file(&path);
    let _ = writeln!(log, "Captura concluída com sucesso: {}x{}", width, height);

    Ok(CapturedFrame {
        width,
        height,
        data,
    })
}
