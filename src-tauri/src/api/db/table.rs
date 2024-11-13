use serde::Serialize;

#[derive(sqlx::FromRow, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClassInfo {
    class_id: i64,
    class_name: String,
}

/// 学院相关信息
#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollegeInfo {
    college_id: i64,
    college_name: String,
}

/// 专业相关信息
#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorInfo {
    major_id: i64,
    major_name: String,
}

/// 学期相关信息
#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermInfo {
    term_id: i64,
    term_name: String,
}

/// 学生相关信息的id

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultRow {
    class: String,
    sno: String,
    name: String,
    gpa: Option<f64>,
}
