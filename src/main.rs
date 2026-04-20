mod capture;
mod ui;

#[tokio::main]
async fn main() {
    println!("[theoshot] Iniciando captura...");

    match capture::capture_frame().await {
        Ok(frame) => {
            println!(
                "[theoshot] Abrindo interface de seleção ({}x{})...",
                frame.width, frame.height
            );
            ui::run_ui(Some(frame.data), frame.width, frame.height);
        }
        Err(e) => {
            eprintln!("[theoshot] Erro fatal na captura: {}", e);
            // Fallback: abre UI vazia para teste de interface se necessário
            // ui::run_ui(None, 1920, 1080);
        }
    }
}
