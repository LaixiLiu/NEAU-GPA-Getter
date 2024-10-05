// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod api;

#[tokio::main]
async fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let db = api::setup_db(&app).await;
    app.manage(api::AppState { db });
    app.run(|_, _| {});
}
