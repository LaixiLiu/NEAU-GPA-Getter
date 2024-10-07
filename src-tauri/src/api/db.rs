use std::{
    error::Error,
    path::{Path, PathBuf},
};

use sqlx::{Pool, Sqlite, SqlitePool};
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

pub async fn insert_or_update_student(
    db: &Pool<Sqlite>,
    student: &student::Student,
) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        r#"
        INSERT INTO students (id, name)
        VALUES ($1, $2)
        ON CONFLICT(id) DO UPDATE SET name = $2
        "#,
    )
    .bind(&student.id)
    .bind(&student.name)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn insert_academic_record(
    db: &Pool<Sqlite>,
    sid: &str,
    record: &student::AcademicRecord,
) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        r#"
        INSERT INTO records (term, student_id, term, college, class, major, gpa)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&record.term)
    .bind(&sid)
    .bind(&record.college)
    .bind(&record.class)
    .bind(&record.major)
    .bind(&record.gpa)
    .execute(db)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tauri::{generate_context, Manager, State};

    use super::*;

    #[tokio::test]
    async fn test_database_initialization() {
        let mut work_dir = std::env::current_dir().unwrap();

        init_db(&mut work_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_student_insertion() {
        let mut work_dir = std::env::current_dir().unwrap();
        let pool = init_db(&mut work_dir).await.unwrap();

        let s1 = student::Student::new("A19220121".to_string(), "张三".to_string());
        let s2 = student::Student::new("A19220122".to_string(), "李四".to_string());

        insert_or_update_student(&pool, &s1).await.unwrap();
        insert_or_update_student(&pool, &s2).await.unwrap();
    }
}
