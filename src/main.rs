mod capture;
mod ui;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("interactive");

    // Se o comando for "screen", usamos interactive(false)
    // Isso captura a tela inteira sem abrir o diálogo de seleção
    let interactive = !matches!(mode, "screen");

    println!("[theoshot] Modo: {} (Interativo: {})", mode, interactive);

    match capture::capture_frame(interactive).await {
        Ok(frame) => {
            ui::run_ui(Some(frame.data), frame.width, frame.height);
        }
        Err(e) => {
            eprintln!("[theoshot] Erro fatal na captura: {}", e);
        }
    }
}
