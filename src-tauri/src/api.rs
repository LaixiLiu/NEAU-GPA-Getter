use std::error::Error;

use db::init_db;
use sqlx::{Pool, Sqlite};
use tauri::App;

mod csv_processor;
mod db;
mod student;

pub struct AppState {
    pub db: Pool<Sqlite>,
}

pub async fn setup_db(app: &App) -> Pool<Sqlite> {
    // init db
    let mut path = app
        .path_resolver()
        .app_data_dir()
        .expect("Failed to get the data directory");
    init_db(&mut path).await.unwrap()
}

pub async fn init_searcher(
    state: tauri::State<'_, AppState>,
    path: &str,
) -> Result<(), Box<dyn Error>> {
    let db = &state.db;
    // parse csv
    let data = csv_processor::extract_data_from_files(path)?;
    // set db

    Ok(())
}

pub fn get_gpa() {}

pub fn get_order() {}
