mod table;
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::error::Error;
use std::sync::Arc;
use table::AcademicInfoId;
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
    /// build the application state
    ///
    /// # Arguments
    ///
    /// * `app` - the application handle
    ///
    /// # Returns
    ///
    /// the application state
    ///
    /// # Errors
    ///
    /// return the error if the operation failed
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

    /// set the csv data
    ///
    /// # Arguments
    ///
    /// * `data` - the csv data
    ///
    /// # Returns
    ///
    /// the result
    ///
    /// # Errors
    ///
    /// return the error if the operation failed
    pub async fn set(&self, data: Vec<CsvTable>) -> Result<(), Box<dyn Error>> {
        let group_size = 400; // todo: move to the configuration
        let data = Arc::new(data);

        // create the task handles
        let mut task_handles = Vec::new();

        // calculate the number of tasks
        let data_cnt = data.len();
        let task_cnt = {
            if data_cnt % group_size == 0 {
                data_cnt / group_size
            } else {
                data_cnt / group_size + 1
            }
        };

        // create the tasks
        for i in 1..=task_cnt {
            // calculate the start and end index
            let start = (i - 1) * group_size;
            let end = if i < task_cnt {
                i * group_size
            } else {
                data_cnt
            };
            println!("start: {}, end: {}", start, end);
            // clone the data and the database
            let data_clone = Arc::clone(&data);
            let db = self.db.clone();
            // create the task
            let task = tokio::spawn(async move {
                let data_slice = &data_clone[start..end];
                for csv_table in data_slice {
                    let mut tx_1 = db.begin().await.unwrap();
                    let CsvTable { records, info } = csv_table;
                    let academic_info_id = insert_academic_info(&mut tx_1, &info).await.unwrap();
                    tx_1.commit().await.unwrap();
                    let mut tx_2 = db.begin().await.unwrap();
                    insert_csv_row_record(&mut tx_2, records, academic_info_id)
                        .await
                        .unwrap();
                    tx_2.commit().await.unwrap();
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
}

/// insert the csv row record into the database
/// should be called after the academic info is inserted
async fn insert_csv_row_record<'db_connect>(
    tx: &mut sqlx::Transaction<'db_connect, Sqlite>,
    records: &CsvRecords,
    academic_info_id: AcademicInfoId,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for record in records {
        // insert the student info
        insert_or_ignore(
            tx,
            r"INSERT OR IGNORE INTO students (student_number, name) VALUES (?1, ?2);",
            vec![record.sid.as_str(), record.name.as_str()],
            None,
        )
        .await
        .unwrap();
        // get the student id
        let student_id = get_record_id(
            tx,
            r"SELECT student_id FROM students WHERE student_number = ?1;",
            vec![record.sid.as_str()],
        )
        .await
        .unwrap();
        // insert the academic record
        sqlx::query(
            r"INSERT INTO academic_records 
                ( gpa, term_id, class_id, student_id )
                VALUES (?1, ?2, ?3, ?4);",
        )
        .bind(record.gpa)
        .bind(academic_info_id.term_id)
        .bind(academic_info_id.class_id)
        .bind(student_id)
        .execute(&mut **tx)
        .await
        .unwrap();
    }

    Ok(())
}

/// insert the academic info into the database
///
/// # Arguments
///
/// * `tx` - the database transaction
/// * `record` - the academic info
///
/// # Returns
///
/// the academic info id
///
/// # Errors
///
/// return the error if the operation failed
async fn insert_academic_info<'db_connect>(
    tx: &mut sqlx::Transaction<'db_connect, Sqlite>,
    record: &AcademicInfo,
) -> Result<AcademicInfoId, Box<dyn Error + Send + Sync>> {
    // insert the academic info into the database

    insert_or_ignore(
        tx,
        r"INSERT OR IGNORE INTO terms (term_name) VALUES (?1);",
        vec![record.term.as_str()],
        None,
    )
    .await
    .unwrap();
    let term_id = get_record_id(
        tx,
        r"SELECT term_id FROM terms WHERE term_name = ?1;",
        vec![record.term.as_str()],
    )
    .await
    .unwrap();

    insert_or_ignore(
        tx,
        r"INSERT OR IGNORE INTO colleges (college_name) VALUES (?1);",
        vec![record.college.as_str()],
        None,
    )
    .await
    .unwrap();
    let college_id = get_record_id(
        tx,
        r"SELECT college_id FROM colleges WHERE college_name = ?1;",
        vec![record.college.as_str()],
    )
    .await
    .unwrap();

    insert_or_ignore(
        tx,
        r"INSERT OR IGNORE INTO majors (major_name, college_id) VALUES (?1, ?2);",
        vec![record.major.as_str()],
        Some(college_id),
    )
    .await
    .unwrap();
    let major_id = get_record_id(
        tx,
        r"SELECT major_id FROM majors WHERE major_name = ?1;",
        vec![record.major.as_str()],
    )
    .await
    .unwrap();

    insert_or_ignore(
        tx,
        r"INSERT OR IGNORE INTO classes (class_name, major_id) VALUES (?1, ?2);",
        vec![record.class.as_str()],
        Some(major_id),
    )
    .await
    .unwrap();
    let class_id = get_record_id(
        tx,
        r"SELECT class_id FROM classes WHERE class_name = ?1;",
        vec![record.class.as_str()],
    )
    .await
    .unwrap();

    Ok(AcademicInfoId {
        term_id,
        college_id,
        major_id,
        class_id,
    })
}

/// get the record id
///
/// # Arguments
///
/// * `tx` - the database transaction
/// * `sql_statement` - the sql statement
/// * `values` - the values
///
/// # Returns
///
/// the record id
///
/// # Errors
///
/// return the error if the operation failed
async fn get_record_id<'db_connection>(
    tx: &mut sqlx::Transaction<'db_connection, Sqlite>,
    sql_statement: &str,
    values: Vec<&str>,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    // create the sql statement
    let mut sql = sqlx::query(sql_statement);
    for value in &values {
        sql = sql.bind(value);
    }
    // get the record id
    let row = sql.fetch_one(&mut **tx).await?;
    let record_id = row.try_get(0)?;
    Ok(record_id)
}

/// insert or ignore the record
///
/// # Arguments
///
/// * `tx` - the database transaction
/// * `sql_statement` - the sql statement
/// * `values` - the values
///
/// # Returns
///
/// return the result
///
/// # Errors
///
/// return the error if the operation failed
async fn insert_or_ignore<'db_connection>(
    tx: &mut sqlx::Transaction<'db_connection, Sqlite>,
    sql_statement: &str,
    values: Vec<&str>,
    id: Option<i64>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut retries = 10; // todo: move to the configuration

    // loop until the operation is successful
    loop {
        // create the sql statement
        let mut sql = sqlx::query(sql_statement);
        for value in &values {
            sql = sql.bind(value);
        }

        if let Some(id) = id {
            sql = sql.bind(id);
        }

        let result = sql.execute(&mut **tx).await;
        match result {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("database is locked") => {
                retries -= 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // todo: move to the configuration
                if retries == 0 {
                    return Err(Box::new(e));
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
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
