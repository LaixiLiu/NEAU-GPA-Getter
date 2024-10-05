use std::{
    error::Error,
    path::{Path, PathBuf},
};

use sqlx::SqlitePool;
use tauri::App;

use super::{student, AppState};

pub async fn init_db(path: &mut PathBuf) -> Result<SqlitePool, Box<dyn Error>> {
    // create the data directory if it doesn't exist
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    // append the database file name to the path
    path.push("data.db");

    // test if the database file exists
    if !path.exists() {
        // create the database file if it doesn't exist
        std::fs::File::create(&path)
            .map_err(|e| format!("Failed to create database file: {}", e))?;
    }

    // open the database
    let pool = SqlitePool::connect(&format!("sqlite://{}", path.to_str().unwrap()))
        .await
        .map_err(|e| format!("Failed to connect to the database: {}", e))?;

    // use the migration feature of sqlx to create the table
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub async fn add_or_update_student(
    state: tauri::State<'_, AppState>,
    student: &student::Student,
) -> Result<(), Box<dyn Error>> {
    let db = &state.db;

    let result = sqlx::query(
        r#"
        INSERT INTO student (id, name, college, major)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT(id) DO UPDATE SET college = $3, major = $4
        "#,
    )
    .bind(&student.id)
    .bind(&student.name)
    .bind(&student.college)
    .bind(&student.major)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn add_record(
    state: tauri::State<'_, AppState>,
    record: &student::Record,
) -> Result<(), Box<dyn Error>> {
    let db = &state.db;

    let result = sqlx::query(
        r#"
        INSERT INTO records (term, gpa, sid)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(&record.term)
    .bind(&record.gpa)
    .bind(&record.sid)
    .execute(db)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tauri::{generate_context, Manager, State};

    use super::*;

    #[tokio::test]
    async fn test_init_db() {
        let mut work_dir = std::env::current_dir().unwrap();

        init_db(&mut work_dir).await.unwrap();
    }

    async fn add_student(db: &SqlitePool, student: &student::Student) {
        sqlx::query(
            r#"
        INSERT INTO students (id, name, college, major)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT(id) DO UPDATE SET college = $3, major = $4
        "#,
        )
        .bind(&student.id)
        .bind(&student.name)
        .bind(&student.college)
        .bind(&student.major)
        .execute(db)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_add_student() {
        let mut work_dir = std::env::current_dir().unwrap();
        let pool = init_db(&mut work_dir).await.unwrap();

        let s1 = student::Student {
            id: "123456".to_string(),
            name: "张三".to_string(),
            college: "计算机学院".to_string(),
            major: "软件工程".to_string(),
        };
        let s2 = student::Student {
            id: "123456".to_string(),
            name: "李四".to_string(),
            college: "水利学院".to_string(),
            major: "土木工程".to_string(),
        };

        add_student(&pool, &s1).await;
        add_student(&pool, &s2).await;
    }
}
