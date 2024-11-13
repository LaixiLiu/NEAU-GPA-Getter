// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use api::*;
use simplelog::{
    format_description, CombinedLogger, Config, ConfigBuilder, LevelFilter, TermLogger, WriteLogger,
};
use std::fs::{File, OpenOptions};
use tauri::Manager;

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
            // init db
            let handle = app.handle();

            let mut path = app.path().data_dir().expect("Failed to get data directory");

            // append the data dir name to the path
            path.push("com.neau.gpa.getter");
            if !path.exists() {
                std::fs::create_dir(&path).expect("Failed to create data directory");
            }
            path.push("tauri.log");

            // init logger
            CombinedLogger::init(vec![
                TermLogger::new(
                    LevelFilter::Warn,
                    Config::default(),
                    simplelog::TerminalMode::Mixed,
                    simplelog::ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_time_offset_to_local()
                        .expect("Failed to set time offset")
                        .set_time_format_custom(format_description!(
                            "[month]-[day] [hour]:[minute]:[second]"
                        ))
                        .build(),
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .append(true)
                        .open(path)
                        .expect("Failed to open log file"),
                ),
            ])
            .expect("Failed to initialize logger");

            log::info!("Logger initialized");

            tauri::async_runtime::block_on(setup_db(handle));

            log::info!("DB initialized");

            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap();

    app.run(|_, _| {});
}
