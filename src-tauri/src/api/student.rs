use csv;
use regex::Regex;
use serde::Deserialize;
use std::{error::Error, path::PathBuf, sync::Arc};

struct Student {
    // 学号
    pub id: String,
    // 姓名
    pub name: String,
}

pub struct AcademicInfo {
    // 学期
    pub term: Arc<String>,
    // 学院
    pub college: Arc<String>,
    // 班级
    pub class: String,
    // 专业
    pub major: String,
}

struct AcademicRecord {
    pub gpa: Option<f64>,
    pub info: AcademicInfo,
}

pub struct ParsedRecord {
    student: Student,
    records: Vec<AcademicRecord>,
}

impl AcademicInfo {
    pub fn new(term: Arc<String>, college: Arc<String>, class: String, major: String) -> Self {
        Self {
            term,
            college,
            class,
            major,
        }
    }
}
