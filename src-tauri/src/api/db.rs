mod table;
use sqlx::{Pool, Sqlite, SqlitePool};
use std::error::Error;
use std::sync::Arc;
use table::{AcademicInfoId, Student};
use tauri::{AppHandle, Manager};

use super::{
    csv_processor::{CsvRecords, CsvTable},
    student::AcademicInfo,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Sqlite>,
}

impl AppState {
    pub async fn build(app: &AppHandle) -> Result<Self, Box<dyn Error>> {
        let mut path = app.path().data_dir()?;

        // create the data directory if it doesn't exist
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        // append the database file name to the path
        path.push("com.neau.gpa.getter");
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }
        path.push("data.db");

        // test if the database file exists
        if !path.exists() {
            // create the database file if it doesn't exist
            std::fs::File::create(&path)
                .map_err(|e| format!("Failed to create database file: {}", e))?;
        }

        // connect the database
        let pool = SqlitePool::connect(&format!("sqlite://{}", path.to_str().unwrap()))
            .await
            .map_err(|e| format!("Failed to connect to the database: {}", e))?;
        // use the migration feature of sqlx to create the table
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(AppState { db: pool })
    }

    pub async fn set(&self, data: Vec<CsvTable>) -> Result<(), Box<dyn Error>> {
        let data = Arc::new(data);

        // create the task handles
        let mut task_handles = Vec::new();

        // calculate the number of tasks
        let data_cnt = data.len();
        let task_cnt = data_cnt / 400;

        // create the tasks
        for i in 1..=task_cnt {
            // calculate the start and end index
            let start = (i - 1) * 400;
            let end = if i < task_cnt { i * 400 } else { data_cnt };
            // clone the data and the database
            let data_clone = Arc::clone(&data);
            let db_state = self.clone();
            // create the task
            let task = tokio::spawn(async move {
                let data_slice = &data_clone[start..end];
                for csv_table in data_slice {
                    let mut tx = db_state.db.begin().await?;
                    let CsvTable { records, info } = csv_table;
                    let academic_info_id = insert_academic_info(&mut tx, &info).await?;
                    insert_csv_row_record(&mut tx, records, academic_info_id).await?;
                    tx.commit().await?;
                }
                Ok::<(), Box<dyn Error + Send + Sync>>(())
            });
            // push the task handle to the vector
            task_handles.push(task);
        }

        // join the tasks
        let results = futures::future::join_all(task_handles).await;

        // todo: handle the error
        let mut success_cnt = 0;
        let mut failed_cnt = 0;
        for result in results {
            match result {
                Ok(_) => {
                    success_cnt += 1;
                }
                Err(_) => {
                    failed_cnt += 1;
                }
            }
        }

        println!("Success: {}, Failed: {}", success_cnt, failed_cnt);

        // todo: handle the results

        Ok(())
    }

    // todo: to be implement
    pub async fn get_single_student_info(
        &self,
        student_number: &str,
    ) -> Result<Student, Box<dyn Error>> {
        todo!();
    }

    // todo: to be implement
    pub async fn get_major_info(
        &self,
        major_id: i64,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}

/// insert the csv row record into the database
/// should be called after the academic info is inserted
async fn insert_csv_row_record<'db_connect>(
    tx: &mut sqlx::Transaction<'db_connect, Sqlite>,
    records: &CsvRecords,
    academic_info_id: AcademicInfoId,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for record in records {
        sqlx::query(r"  INSERT OR IGNORE INTO students (student_number, name) VALUES (?1, ?2);")
            .bind(record.sid.as_str())
            .bind(record.name.as_str())
            .execute(&mut **tx)
            .await?;
        let student: Student =
            sqlx::query_as(r"SELECT * FROM students WHERE student_number == (?1)")
                .bind(record.sid.as_str())
                .fetch_one(&mut **tx)
                .await?;
        sqlx::query(
            r"INSERT INTO academic_records 
                ( gpa, term_id, class_id, college_id, major_id, student_id )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
        )
        .bind(record.gpa)
        .bind(academic_info_id.term_id)
        .bind(academic_info_id.class_id)
        .bind(academic_info_id.college_id)
        .bind(academic_info_id.major_id)
        .bind(student.student_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

/// insert the academic info into the database
/// return the academic info id
async fn insert_academic_info<'db_connect>(
    tx: &mut sqlx::Transaction<'db_connect, Sqlite>,
    record: &AcademicInfo,
) -> Result<AcademicInfoId, Box<dyn Error + Send + Sync>> {
    // create the query statement
    sqlx::query(r"  INSERT OR IGNORE INTO classes (class_name) VALUES (?1)")
        .bind(record.class.as_str())
        .execute(&mut **tx)
        .await?;
    sqlx::query(r"  INSERT OR IGNORE INTO majors (major_name) VALUES (?1);")
        .bind(record.major.as_str())
        .execute(&mut **tx)
        .await?;
    sqlx::query(r"  INSERT OR IGNORE INTO colleges (college_name) VALUES (?1);")
        .bind(record.college.as_str())
        .execute(&mut **tx)
        .await?;
    sqlx::query(r"  INSERT OR IGNORE INTO terms (term_name) VALUES (?1);")
        .bind(record.term.as_str())
        .execute(&mut **tx)
        .await?;

    let academic_info_id = sqlx::query_as(
        r"SELECT
                    t.term_id,
                    c.college_id,
                    m.major_id,
                    cl.class_id 
                FROM terms AS t
                JOIN colleges c ON c.college_name = ?1
                JOIN majors m ON m.major_name = ?2
                JOIN classes cl ON cl.class_name = ?3
                WHERE t.term_name = ?4;",
    )
    .bind(record.college.as_str())
    .bind(record.major.as_str())
    .bind(record.class.as_str())
    .bind(record.term.as_str())
    .fetch_one(&mut **tx)
    .await
    .unwrap();

    Ok(academic_info_id)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use crate::api::csv_processor::RowRecord;

    use super::*;
    use tempfile::tempdir;

    async fn build_app_state() -> Result<AppState, Box<dyn Error>> {
        // get the cargo project root directory
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // create the data directory if it doesn't exist
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        // append the database file name to the path
        path.push("com.neau.gpa.getter");
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }
        path.push("data.db");

        // test if the database file exists
        if !path.exists() {
            // create the database file if it doesn't exist
            std::fs::File::create(&path)
                .map_err(|e| format!("Failed to create database file: {}", e))?;
        }

        // connect the database
        let pool = SqlitePool::connect(&format!("sqlite://{}", path.to_str().unwrap()))
            .await
            .map_err(|e| format!("Failed to connect to the database: {}", e))?;
        // use the migration feature of sqlx to create the table
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(AppState { db: pool })
    }

    #[tokio::test]
    async fn test_insert_academic_info() {
        let app_state = build_app_state().await.unwrap();
        let academic_info = AcademicInfo {
            term: Arc::new("Fall 2021".to_string()),
            college: Arc::new("Engineering".to_string()),
            major: Arc::new("Computer Science".to_string()),
            class: Arc::new("CS101".to_string()),
        };

        let mut tx = app_state.db.begin().await.unwrap();
        let result = insert_academic_info(&mut tx, &academic_info).await;
        tx.commit().await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_csv_data() {
        let app_state = build_app_state().await.unwrap();
        let csv_data = vec![CsvTable {
            info: AcademicInfo {
                term: Arc::new("Fall 2021".to_string()),
                college: Arc::new("Engineering".to_string()),
                major: Arc::new("Computer Science".to_string()),
                class: Arc::new("CS101".to_string()),
            },
            records: vec![RowRecord {
                sid: "12345".to_string(),
                name: "John Doe".to_string(),
                gpa: Some(4.0_f64),
            }],
        }];

        let result = app_state.set(csv_data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_single_student_info() {
        let app_state = build_app_state().await.unwrap();
        let csv_data = vec![CsvTable {
            info: AcademicInfo {
                term: Arc::new("Fall 2021".to_string()),
                college: Arc::new("Engineering".to_string()),
                major: Arc::new("Computer Science".to_string()),
                class: Arc::new("CS101".to_string()),
            },
            records: vec![RowRecord {
                sid: "12345".to_string(),
                name: "John Doe".to_string(),
                gpa: Some(4.0_f64),
            }],
        }];

        app_state.set(csv_data).await.unwrap();
        let result = app_state.get_single_student_info("12345").await;
        assert!(result.is_ok());
        let student = result.unwrap();
        assert_eq!(student.student_number, "12345");
        assert_eq!(student.name, "John Doe");
    }

    #[tokio::test]
    async fn test_get_major_info() {
        let app_state = build_app_state().await.unwrap();
        let academic_info = AcademicInfo {
            term: Arc::new("Fall 2021".to_string()),
            college: Arc::new("Engineering".to_string()),
            major: Arc::new("Computer Science".to_string()),
            class: Arc::new("CS101".to_string()),
        };

        let mut tx = app_state.db.begin().await.unwrap();

        let academic_info_id = insert_academic_info(&mut tx, &academic_info).await.unwrap();
        let result = app_state.get_major_info(academic_info_id.major_id).await;
        assert!(result.is_ok());
        let major_name = result.unwrap();
        assert_eq!(major_name, "Computer Science");
    }
}
