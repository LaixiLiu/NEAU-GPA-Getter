use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use db::AppState;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

mod csv_processor;
mod data_parser;
mod db;
mod err;
mod student;

pub async fn setup_db(app: &AppHandle) {
    let db = db::AppState::build(app)
        .await
        .expect("Failed to build the database");
    app.manage(db);
}

async fn pick_folder_dialog(app: AppHandle) -> Option<PathBuf> {
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
    db: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let path: PathBuf = match pick_folder_dialog(app).await {
        Some(val) => val,
        None => return Ok(()),
    };

    // 计时
    let start = std::time::Instant::now();

    // parse csv
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let producer = data_parser::DataProducer::new(tx);
    let mut consumer = data_parser::DataConsumer::new(rx);

    // todo: handle the error
    let producer_task = tokio::spawn(async move {
        producer.produce(path).await.unwrap();
    });
    let consumer_task = tokio::spawn(async move { consumer.consume().await });

    let (producer_result, consumer_result) = tokio::join!(producer_task, consumer_task);

    // 结束计时
    let elapsed = start.elapsed();
    println!("Time elapsed for parsing csv files: {:?}", elapsed);
    println!(
        "The number of records: {}",
        consumer_result.as_ref().unwrap().len()
    );

    // handle the error
    if let Err(e) = producer_result {
        return Err(format!("Failed to produce data: {:?}", e));
    }
    let data = match consumer_result {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to consume data: {:?}", e)),
    };

    // set db
    // TODO: use multiple threads to insert data
    db.set(data)
        .await
        .map_err(|e| format!("Failed to set data: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_terms(
    app: tauri::State<'_, AppState>,
) -> Result<Vec<db::table::TermInfo>, String> {
    match app.get_terms().await {
        Ok(terms) => Ok(terms),
        Err(e) => Err(format!("Failed to get terms: {:?}", e)),
    }
}

#[tauri::command]
pub async fn get_colleges(
    app: tauri::State<'_, AppState>,
) -> Result<Vec<db::table::CollegeInfo>, String> {
    match app.get_colleges().await {
        Ok(colleges) => Ok(colleges),
        Err(e) => Err(format!("Failed to get colleges: {:?}", e)),
    }
}

#[tauri::command]
pub async fn get_majors(
    app: tauri::State<'_, AppState>,
    college_id: i64,
) -> Result<Vec<db::table::MajorInfo>, String> {
    match app.get_majors(college_id).await {
        Ok(majors) => Ok(majors),
        Err(e) => Err(format!("Failed to get majors: {:?}", e)),
    }
}

#[tauri::command]
pub async fn get_classes(
    app: tauri::State<'_, AppState>,
    major_id: i64,
) -> Result<Vec<db::table::ClassInfo>, String> {
    match app.get_classes(major_id).await {
        Ok(classes) => Ok(classes),
        Err(e) => Err(format!("Failed to get classes: {:?}", e)),
    }
}

#[tauri::command]
pub async fn get_gpa(
    app: tauri::State<'_, AppState>,
    terms: Vec<i64>,
    major_id: i64,
    grade: String,
    class_id: Option<i64>,
) -> Result<Vec<db::table::ResultRow>, String> {
    match app.get_gpa(&terms, major_id, &grade, class_id).await {
        Ok(gpa) => Ok(gpa),
        Err(e) => Err(format!("Failed to get gpa: {:?}", e)),
    }
}
