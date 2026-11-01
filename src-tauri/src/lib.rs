mod anilist;
mod auth;
mod http;
mod nuvem;
mod player;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_libmpv::init())
        .setup(|app| {
            auth::setup(app)?;
            http::setup(app)?;
            anilist::setup(app)?;
            player::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::commands::auth_login,
            auth::commands::auth_logout,
            auth::commands::auth_session,
            auth::commands::auth_access_token,
            auth::commands::auth_refresh,
            auth::commands::auth_reopen_login,
            auth::commands::auth_cancel_login,
            http::commands::anitsu_request,
            nuvem::commands::nuvem_list,
            nuvem::commands::nuvem_search,
            nuvem::commands::nuvem_download,
            anilist::commands::anilist_lookup,
            anilist::commands::anilist_media,
            anilist::commands::anilist_season,
            player::commands::player_stream_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
