use std::{
    path::{self, PathBuf},
    sync::{Arc, Mutex},
};

use db::init_db;
use sqlx::{Pool, Sqlite};
use student::ParsedStudentData;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

mod csv_processor;
mod db;
mod student;

pub struct AppState {
    pub db: Pool<Sqlite>,
}

pub async fn setup_db(app: &AppHandle) {
    // init db
    let mut path = app
        .path()
        .data_dir()
        .expect("Failed to get the data directory");
    let db = init_db(&mut path).await.expect("Failed to init db");
    app.manage(AppState { db });
}

async fn pick_folder_dialog(app: tauri::AppHandle) -> Option<path::PathBuf> {
    let directory_path = Arc::new(Mutex::new(None));

    // clone the directory_path which will be used in closure
    let directory_path_clone = Arc::clone(&directory_path);

    // spawn a new async thread
    let handle = tokio::spawn(async move {
        let selected_path = app.dialog().file().blocking_pick_folder();
        let mut dir = directory_path_clone
            .lock()
            .expect("Failed to lock directory path when selecting directory");
        *dir = selected_path;
    });

    // await the task
    handle
        .await
        .expect("Failed to await the selecting directory task");

    // get the value
    let dir_path = directory_path
        .lock()
        .expect("Failed to lock directory path when getting the value");
    match &*dir_path {
        Some(path) => Some(PathBuf::from(path.to_string())),
        None => None,
    }
}

#[tauri::command]
pub async fn initialize_searcher(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let db = &state.db;

    let path: PathBuf = match pick_folder_dialog(app).await {
        Some(val) => val,
        None => return Ok(()),
    };
    // parse csv
    let data = csv_processor::extract_data_from_files(&path).map_err(|err| err.to_string())?;
    // set db
    // TODO: use multiple threads to insert data
    for (sid, ParsedStudentData { student, records }) in data {
        db::insert_or_update_student(db, &student)
            .await
            .map_err(|err| err.to_string())?;
        for record in records {
            db::insert_academic_record(db, &sid, &record)
                .await
                .map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

pub fn get_gpa() {}

pub fn get_order() {}
