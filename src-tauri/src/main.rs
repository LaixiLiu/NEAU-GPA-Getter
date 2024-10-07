// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tokio::runtime;

mod api;

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .build(tauri::generate_context!())
        .unwrap();

    let db = runtime::Runtime::new()
        .unwrap()
        .block_on(api::setup_db(&app));

    app.manage(api::AppState { db });

    app.run(|_, _| {});
}
