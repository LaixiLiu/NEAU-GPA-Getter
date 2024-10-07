pub struct Student {
    // 学号
    pub id: String,
    // 姓名
    pub name: String,
}

pub struct AcademicRecord {
    // term
    pub term: String,
    // gpa
    pub gpa: Option<f64>,
    // 学院
    pub college: String,
    // 班级
    pub class: String,
    // 专业
    pub major: String,
}

pub struct ParsedStudentData {
    pub student: Student,
    pub records: Vec<AcademicRecord>,
}

impl Student {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}

impl AcademicRecord {
    pub fn new(
        term: String,
        gpa: Option<f64>,
        college: String,
        class: String,
        major: String,
    ) -> Self {
        Self {
            term,
            gpa,
            college,
            class,
            major,
        }
    }
}

impl ParsedStudentData {
    pub fn new(student: Student, records: Vec<AcademicRecord>) -> Self {
        Self { student, records }
    }
}
