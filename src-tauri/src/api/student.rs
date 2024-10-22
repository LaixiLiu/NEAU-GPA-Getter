use std::sync::Arc;

pub struct Student {
    // 学号
    pub id: String,
    // 姓名
    pub name: String,
}

#[derive(Clone)]
pub struct AcademicInfo {
    // 学期
    pub term: Arc<String>,
    // 学院
    pub college: Arc<String>,
    // 班级
    pub class: Arc<String>,
    // 专业
    pub major: Arc<String>,
}

pub struct AcademicRecordId {
    pub term_id: i32,
    pub college_id: i32,
    pub major_id: i32,
    pub class_id: i32,
}

impl Student {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}

impl AcademicInfo {
    pub fn new(
        term: Arc<String>,
        college: Arc<String>,
        class: Arc<String>,
        major: Arc<String>,
    ) -> Self {
        Self {
            term,
            college,
            class,
            major,
        }
    }
}
