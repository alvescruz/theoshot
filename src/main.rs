mod capture;
mod ui;

use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
};
use std::path::PathBuf;

// ─── Token storage ───────────────────────────────────────────────────────────

fn token_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("theoshot")
        .join("screencast_restore_token")
}

fn load_token() -> Option<String> {
    std::fs::read_to_string(token_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn save_token(token: &str) {
    let path = token_path();
    let Some(parent) = path.parent() else { return };

    if let Err(e) = std::fs::create_dir_all(parent) {
        eprintln!("[screencast] Falha ao criar diretório do token: {e}");
        return;
    }

    if let Err(e) = std::fs::write(&path, token) {
        eprintln!("[screencast] Falha ao salvar token: {e}");
    } else {
        println!("[screencast] Token salvo em: {}", path.display());
    }
}

fn clear_token() {
    let path = token_path();
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            eprintln!("[screencast] Falha ao remover token: {e}");
        } else {
            println!("[screencast] Token removido.");
        }
    }
}

// ─── Screencast session ───────────────────────────────────────────────────────

pub async fn start_screencast() -> ashpd::Result<()> {
    let restore_token = load_token();

    let proxy = Screencast::new().await?;
    let session = proxy.create_session(Default::default()).await?;

    let mut options = SelectSourcesOptions::default()
        .set_cursor_mode(CursorMode::Metadata)
        .set_sources(SourceType::Monitor | SourceType::Window)
        .set_multiple(false)
        .set_persist_mode(PersistMode::ExplicitlyRevoked);

    if let Some(ref token) = restore_token {
        options = options.set_restore_token(token.as_str());
    }

    proxy.select_sources(&session, options).await?;

    let response = proxy
        .start(&session, None, Default::default())
        .await?
        .response()?;

    if let Some(new_token) = response.restore_token() {
        save_token(new_token);
    }

    for stream in response.streams() {
        let node_id = stream.pipe_wire_node_id();
        let (w, h) = stream.size().unwrap_or((1920, 1080));
        
        println!("[screencast] Node: {}, Resolução: {}x{}", node_id, w, h);
        
        match capture::capture_frame(node_id, w as u32, h as u32).await {
            Ok(frame) => {
                println!("[ui] Abrindo interface de seleção...");
                ui::run_ui(Some(frame.data), frame.width, frame.height);
            }
            Err(e) => {
                eprintln!("[capture] Erro: {}", e);
                // Mesmo com erro de captura, abre UI vazia para teste
                ui::run_ui(None, w as u32, h as u32);
            }
        }
    }

    Ok(())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    if let Err(e) = start_screencast().await {
        eprintln!("[screencast] Erro fatal: {e}");
        clear_token();
    }
}
