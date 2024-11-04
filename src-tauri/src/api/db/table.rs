use serde::Serialize;

#[derive(sqlx::FromRow, Serialize)]
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

#[derive(sqlx::FromRow)]
pub struct Student {
    pub student_id: i64,
    pub name: String,
    pub student_number: String,
}

/// 学生相关信息的id
#[derive(sqlx::FromRow)]
pub struct AcademicInfoId {
    /// 学期ID
    pub term_id: i64,
    /// 学院ID
    pub college_id: i64,
    /// 专业ID
    pub major_id: i64,
    /// 班级ID
    pub class_id: i64,
}

impl AcademicInfoId {
    pub fn new(term_id: i64, college_id: i64, major_id: i64, class_id: i64) -> Self {
        Self {
            term_id,
            college_id,
            major_id,
            class_id,
        }
    }
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultRow {
    class: String,
    sno: String,
    name: String,
    gpa: Option<f64>,
}
