use std::{error::Error, path};

use db::init_db;
use sqlx::{Pool, Sqlite};
use student::ParsedStudentData;
use tauri::{App, Manager};

mod csv_processor;
mod db;
mod student;

pub struct AppState {
    pub db: Pool<Sqlite>,
}

pub async fn setup_db(app: &App) -> Pool<Sqlite> {
    // init db
    let mut path = app
        .path()
        .data_dir()
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
    // TODO: use multiple threads to insert data
    for (sid, ParsedStudentData { student, records }) in data {
        db::insert_or_update_student(db, &student).await?;
        for record in records {
            db::insert_academic_record(db, &sid, &record).await?;
        }
    }

    Ok(())
}

pub fn get_gpa() {}

pub fn get_order() {}
