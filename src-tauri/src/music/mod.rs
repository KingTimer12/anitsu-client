//! Player de músicas: lê faixas da nuvem por Range, decodifica e toca pelo Rust.

pub mod commands;
pub mod decode;
pub mod engine;
pub mod gain;
pub mod image;
pub mod media_keys;
pub mod output;
pub mod queue;
pub mod source;

use tauri::Manager;

/// Motor, comandos e teclas de mídia.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let engine = engine::Engine::spawn(app.handle().clone());
    let meta = app.path().app_data_dir().ok().map(|d| d.join("music-meta.json"));
    app.manage(commands::MusicState::new(engine, meta));
    media_keys::start(app);
    Ok(())
}
