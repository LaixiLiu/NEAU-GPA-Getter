// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use api::*;

mod api;

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            initialize_searcher,
            get_terms,
            get_colleges,
            get_majors,
            get_classes,
            get_gpa,
        ])
        .setup(|app| {
            let handle = app.handle();
            tauri::async_runtime::block_on(api::setup_db(handle));
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap();

    app.run(|_, _| {});
}
