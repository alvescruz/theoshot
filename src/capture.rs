use ashpd::desktop::screenshot::Screenshot;
use image::GenericImageView;
use url::Url;

pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub async fn capture_frame(interactive: bool) -> Result<CapturedFrame, String> {
    // Usando a API de alto nível recomendada pela ashpd
    let response = Screenshot::request()
        .interactive(interactive)
        .modal(true)
        .send()
        .await
        .map_err(|e| format!("Portal error: {}", e))?
        .response()
        .map_err(|e| format!("Response error: {}", e))?;

    let uri_str = response.uri().as_str();
    let url = Url::parse(uri_str).map_err(|e| format!("Url parse error: {}", e))?;
    let path = url
        .to_file_path()
        .map_err(|_| format!("Invalid URI: {}", uri_str))?;

    let img = image::open(&path).map_err(|e| format!("Image open error: {}", e))?;
    let (width, height) = img.dimensions();
    let data = img.to_rgba8().into_raw();

    let _ = std::fs::remove_file(&path);

    Ok(CapturedFrame {
        width,
        height,
        data,
    })
}
