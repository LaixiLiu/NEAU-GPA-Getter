use db::init_db;
use sqlx::{Pool, Sqlite};
use tauri::App;

mod csv_parser;
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

pub fn init_searcher() {
    // parse csv

    // set db
}

pub fn get_gpa() {}

pub fn get_order() {}
