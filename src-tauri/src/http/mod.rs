//! Cliente HTTP dos serviços do Anitsu, sobre o reqwest do `tauri-plugin-http`.
//!
//! Tokens nunca saem do Rust: o frontend chama comandos e o [`AnitsuHttp`]
//! injeta a sessão (cookie ou bearer) conforme o serviço.

mod client;
pub mod commands;
mod error;

use tauri::Manager;

pub use client::{AnitsuApis, AnitsuHttp};
pub use error::HttpResult;

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(AnitsuApis::new());
    Ok(())
}
