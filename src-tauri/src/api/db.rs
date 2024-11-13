pub mod table;

use super::{
    csv_processor::{CsvRecords, CsvTable},
    student::AcademicInfo,
};
use crate::api::data_parser::CollegeData;
use log::info;
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use table::{AcademicInfoId, ClassInfo, CollegeInfo, MajorInfo, ResultRow, TermInfo};
use tauri::{AppHandle, Manager};

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
    pub async fn set(&self, data: Vec<CollegeData>) -> Result<String, Box<dyn Error>> {
        // sort the data by term name
        let mut data = data;
        data.sort_by(|a, b| a.term_name.cmp(&b.term_name));

        // create the task handles
        let mut task_handles = Vec::new();

        info!("Inserting academic info");

        // insert the term and colleges info at first

        let (terms_map, classes_map) = {
            let mut tx = self.db.begin().await.unwrap();
            let (terms, classes) = insert_academic_info(&mut tx, &data).await.unwrap();
            tx.commit().await?;
            (terms, Arc::new(classes))
        };

        // create the tasks
        for college_data in data {
            // extract the college data
            let CollegeData {
                term_name,
                college_name: _,
                college_number: _,
                data,
            } = college_data;
            let term_id = *terms_map.get(term_name.as_str()).unwrap();

            // create a db connection clone
            let db = self.db.clone();

            let classes_map = classes_map.clone();
            // create the task
            let task = tokio::spawn(async move {
                // begin the transaction
                let mut tx = db.begin().await.unwrap();
                for table in data {
                    // extract the academic info
                    let CsvTable {
                        records,
                        major_name: _,
                        class_name,
                    } = table;
                    // begin the transaction
                    // get the classes id
                    let class_id = *classes_map.get(class_name.as_str()).unwrap();

                    // insert the academic records
                    insert_csv_row_record(&mut tx, &records, term_id, class_id)
                        .await
                        .unwrap();
                }
                // commit the transaction
                tx.commit().await.unwrap();

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

        let result_str = format!(
            "Success file count: {}, Failed file count: {}",
            success_cnt, failed_cnt
        );

        Ok(result_str)
    }

    /// get the loaded term info
    pub async fn get_terms(&self) -> Result<Vec<TermInfo>, Box<dyn Error>> {
        let terms: Vec<TermInfo> = sqlx::query_as(r"SELECT term_id, term_name FROM terms;")
            .fetch_all(&self.db)
            .await
            .unwrap(); // todo: handle the error
        Ok(terms)
    }

    /// get the all colleges
    pub async fn get_colleges(&self) -> Result<Vec<CollegeInfo>, Box<dyn Error>> {
        let colleges: Vec<CollegeInfo> =
            sqlx::query_as(r"SELECT college_id, college_name FROM colleges;")
                .fetch_all(&self.db)
                .await
                .unwrap(); // todo: handle the error
        Ok(colleges)
    }

    /// get the majors under the college
    pub async fn get_majors(&self, college_id: i64) -> Result<Vec<MajorInfo>, Box<dyn Error>> {
        let majors: Vec<MajorInfo> =
            sqlx::query_as(r"SELECT major_id, major_name FROM majors WHERE college_id = ?1;")
                .bind(college_id)
                .fetch_all(&self.db)
                .await
                .unwrap(); // todo: handle the error
        Ok(majors)
    }

    /// get the classes under the major
    pub async fn get_classes(
        &self,
        major_id: i64,
        grade: i32,
    ) -> Result<Vec<ClassInfo>, Box<dyn Error>> {
        let sql_statement = format!(
            r"SELECT class_id, class_name FROM classes WHERE major_id = {} AND classes.class_name LIKE '%{}__';",
            major_id, grade
        );
        let classes: Vec<ClassInfo> = sqlx::query_as(&sql_statement)
            .fetch_all(&self.db)
            .await
            .unwrap(); // todo: handle the error
        Ok(classes)
    }

    /// get the gpa by the given arguments
    ///
    /// # Arguments
    ///
    /// * `terms` - the terms id slice
    /// * `college_id` - the college id
    /// * `major_id` - the major id
    /// * `grade` - the grade, such as 19 20 21 22
    /// * `class_id` - the class id, optional
    pub async fn get_gpa(
        &self,
        terms: &[i64],
        major_id: i64,
        grade: &str,
        class_id: Option<i64>,
    ) -> Result<Vec<ResultRow>, Box<dyn Error>> {
        let mut sql_str = match terms.len() {
            1 => {
                format!(
                    r"SELECT    class_name AS class,
                                student_number AS sno, 
                                students.name AS name,
                                gpa
                    FROM academic_records
                    JOIN students ON academic_records.student_id = students.student_id
                    JOIN terms ON academic_records.term_id = terms.term_id
                    JOIN classes ON academic_records.class_id = classes.class_id 
                    JOIN majors ON majors.major_id = classes.major_id
                    JOIN colleges ON colleges.college_id = majors.college_id
                    WHERE terms.term_id = {} 
                    AND majors.major_id = {}
                    AND classes.class_name LIKE '%{}__' ",
                    terms[0], major_id, grade
                )
            }
            _ => {
                let placeholders = terms
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                format!(
                    r"SELECT s.cname AS class, s.sno AS sno, s.sname AS name, SUM( academic_records.gpa ) AS gpa 
                    FROM academic_records
                    JOIN (
                        SELECT
                        -- 学生基础信息字段
                        students.student_id AS sid,
                        students.student_number AS sno,
                        students.name AS sname,
                        -- 班级相关字段
                        classes.class_id AS cid,
                        classes.class_name AS cname 
                        FROM
                          students
                          JOIN academic_records ON academic_records.student_id = students.student_id 
                          AND academic_records.term_id = {}
                          JOIN classes ON classes.class_id = academic_records.class_id 
                          AND classes.major_id = {} 
                          AND classes.class_name LIKE '%{}__' 
                        GROUP BY
                          students.student_id 
                        ) AS s
                        ON academic_records.student_id = s.sid 
                            AND academic_records.term_id IN ( {} )
                    WHERE 1 = 1
                    ",
                    terms[terms.len() - 1],
                    major_id,
                    grade,
                    placeholders
                )
            }
        };
        if let Some(class_id) = class_id {
            let class_str = format!("AND academic_records.class_id = {}\n", class_id);
            sql_str.push_str(&class_str);
        }
        if terms.len() > 1 {
            sql_str.push_str("GROUP BY s.sid");
        }
        sql_str.push_str(";");

        let result: Vec<ResultRow> = sqlx::query_as(sql_str.as_str())
            .fetch_all(&self.db)
            .await
            .unwrap(); // todo: handle the error
        Ok(result)
    }
}

/// insert the academic info and return the id map
pub async fn insert_academic_info<'db_connect>(
    tx: &mut sqlx::Transaction<'db_connect, Sqlite>,
    data: &Vec<CollegeData>,
) -> Result<(HashMap<String, i64>, HashMap<String, i64>), Box<dyn Error + Send + Sync>> {
    // create the terms, colleges and majors map
    let mut terms = HashMap::new();
    let mut colleges = HashMap::new();
    let mut majors = HashMap::new();
    let mut classes = HashMap::new();

    // for each college data
    for college_data in data {
        // extract the college data
        let CollegeData {
            term_name,
            college_name,
            college_number,
            data: _,
        } = college_data;
        // insert the term and college info
        if let None = terms.get(term_name.as_str()) {
            let term_id = insert_terms(tx, term_name.as_str()).await?;
            terms.insert(term_name.as_str().to_string(), term_id);
        }
        let college_id = {
            match colleges.get(college_name.as_str()) {
                Some(college_id) => *college_id,
                None => {
                    let college_id =
                        insert_college(tx, college_name.as_str(), college_number.as_str()).await?;
                    colleges.insert(college_number.as_str().to_string(), college_id);
                    college_id
                }
            }
        };
        // insert info from each table
        for table in &college_data.data {
            let CsvTable {
                major_name,
                class_name,
                ..
            } = table;
            // insert the major info
            let major_id = if let Some(id) = majors.get(major_name) {
                *id
            } else {
                let id = insert_major(tx, major_name.as_str(), college_id.clone()).await?;
                majors.insert(major_name.as_str().to_string(), id);
                id
            };
            // insert the class info
            if let None = classes.get(class_name) {
                let class_id = insert_class(tx, class_name, major_id).await?;
                classes.insert(class_name.to_string(), class_id);
            }
        }
    }
    Ok((terms, classes))
}

/// insert the csv row record into the database
/// should be called after the academic info is inserted
async fn insert_csv_row_record<'db_connect>(
    tx: &mut sqlx::Transaction<'db_connect, Sqlite>,
    records: &CsvRecords,
    term_id: i64,
    class_id: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for record in records {
        // insert the student info
        let student_id = insert_or_ignore_student(tx, &record.sid, &record.name)
            .await
            .unwrap();
        // create the sql statement
        let sql_statement = format!(
            r"INSERT OR IGNORE INTO academic_records ( gpa, term_id, class_id, student_id ) VALUES ({}, {}, {}, {});",
            record.gpa.unwrap_or(0.0),
            term_id,
            class_id,
            student_id
        );
        insert_with_retry(tx, &sql_statement).await.unwrap();
    }

    Ok(())
}

async fn insert_terms(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    term_name: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    insert(
        tx,
        r"INSERT OR IGNORE INTO terms (term_name) VALUES (?1);",
        vec![term_name],
        None,
    )
    .await?;

    let term_id = get_record_id(
        tx,
        r"SELECT term_id FROM terms WHERE term_name = ?1;",
        vec![term_name],
    )
    .await?;

    Ok(term_id)
}

async fn insert_college(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    college_name: &str,
    college_number: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    insert(
        tx,
        r"INSERT INTO colleges (college_name, college_number) VALUES (?1, ?2) ON CONFLICT(college_number) DO UPDATE SET college_name = excluded.college_name;",
        vec![college_name, college_number],
        None,
    )
    .await?;

    let college_id = get_record_id(
        tx,
        r"SELECT college_id FROM colleges WHERE college_number = ?1;",
        vec![college_number],
    )
    .await?;

    Ok(college_id)
}

async fn insert_major(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    major_name: &str,
    college_id: i64,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    insert(
        tx,
        r"INSERT OR IGNORE INTO majors (major_name, college_id) VALUES (?1, ?2);",
        vec![major_name, &college_id.to_string()],
        Some(college_id),
    )
    .await?;

    let major_id = get_record_id(
        tx,
        r"SELECT major_id FROM majors WHERE major_name = ?1;",
        vec![major_name],
    )
    .await?;

    Ok(major_id)
}

async fn insert_class(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    class_name: &str,
    major_id: i64,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    insert(
        tx,
        r"INSERT OR IGNORE INTO classes (class_name, major_id) VALUES (?1, ?2);",
        vec![class_name, &major_id.to_string()],
        Some(major_id),
    )
    .await?;

    let class_id = get_record_id(
        tx,
        r"SELECT class_id FROM classes WHERE class_name = ?1;",
        vec![class_name],
    )
    .await?;

    Ok(class_id)
}

async fn insert_or_ignore_student(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    student_number: &str,
    name: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let sql_statement = format!(
        r"INSERT OR IGNORE INTO students (student_number, name) VALUES ('{}', '{}');",
        student_number, name
    );
    insert_with_retry(tx, &sql_statement).await?;

    let student_id = get_record_id(
        tx,
        r"SELECT student_id FROM students WHERE student_number = ?1;",
        vec![student_number],
    )
    .await?;

    Ok(student_id)
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
async fn insert_with_retry<'db_connection>(
    tx: &mut sqlx::Transaction<'db_connection, Sqlite>,
    sql_statement: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut retries = 20; // todo: move to the configuration
    let retry_delay = tokio::time::Duration::from_millis(100); // todo: move to the configuration

    // loop until the operation is successful
    loop {
        let result = sqlx::query(sql_statement).execute(&mut **tx).await;
        match result {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("database is locked") => {
                retries -= 1;
                tokio::time::sleep(retry_delay).await; // todo: move to the configuration
                if retries == 0 {
                    return Err(Box::new(e));
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
}

async fn insert<'db_connection>(
    tx: &mut sqlx::Transaction<'db_connection, Sqlite>,
    sql_statement: &str,
    values: Vec<&str>,
    id: Option<i64>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // create the sql statement
    let mut sql = sqlx::query(sql_statement);
    for value in &values {
        sql = sql.bind(value);
    }

    if let Some(id) = id {
        sql = sql.bind(id);
    }
    sql.execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use crate::api::csv_processor::RowRecord;

    use super::*;

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
    async fn set_csv_data_handles_empty_data() {
        let app_state = build_app_state().await.unwrap();
        let csv_data = vec![];

        let result = app_state.set(csv_data).await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn get_terms_returns_terms() {
        let app_state = build_app_state().await.unwrap();
        let terms = app_state.get_terms().await;
        assert!(terms.is_ok());
        assert!(!terms.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_colleges_returns_colleges() {
        let app_state = build_app_state().await.unwrap();
        let colleges = app_state.get_colleges().await;
        assert!(colleges.is_ok());
        assert!(!colleges.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_majors_returns_majors() {
        let app_state = build_app_state().await.unwrap();
        let college_id = 1; // Assuming a valid college_id
        let majors = app_state.get_majors(college_id).await;
        assert!(majors.is_ok());
        assert!(!majors.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_classes_returns_classes() {
        let app_state = build_app_state().await.unwrap();
        let major_id = 1; // Assuming a valid major_id
        let classes = app_state.get_classes(major_id).await;
        assert!(classes.is_ok());
        assert!(!classes.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_gpa_returns_gpa() {
        let app_state = build_app_state().await.unwrap();
        let terms = vec![1, 2]; // Assuming valid term_ids
        let major_id = 1; // Assuming a valid major_id
        let grade = "1";
        let class_id = Some(1); // Assuming a valid class_id

        let gpa = app_state.get_gpa(&terms, major_id, &grade, class_id).await;
        assert!(gpa.is_ok());
        assert!(!gpa.unwrap().is_empty());
    }
}
