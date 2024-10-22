-- 启用外键约束
PRAGMA foreign_keys = ON;

-- 创建 students 表
CREATE TABLE
    IF NOT EXISTS students (
        student_id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        student_number TEXT NOT NULL UNIQUE
    );

-- 创建 terms 表
CREATE TABLE
    IF NOT EXISTS terms (
        term_id INTEGER PRIMARY KEY AUTOINCREMENT,
        term_name TEXT NOT NULL UNIQUE
    );

-- 创建 colleges 表
CREATE TABLE
    IF NOT EXISTS colleges (
        college_id INTEGER PRIMARY KEY AUTOINCREMENT,
        college_name TEXT NOT NULL UNIQUE
    );

-- 创建 majors 表
CREATE TABLE
    IF NOT EXISTS majors (
        major_id INTEGER PRIMARY KEY AUTOINCREMENT,
        major_name TEXT NOT NULL UNIQUE
    );

-- 创建 classes 表
CREATE TABLE
    IF NOT EXISTS classes (
        class_id INTEGER PRIMARY KEY AUTOINCREMENT,
        class_name TEXT NOT NULL UNIQUE
    );

-- 创建 academic_records 表
CREATE TABLE
    IF NOT EXISTS academic_records (
        record_id INTEGER PRIMARY KEY AUTOINCREMENT,
        student_id INTEGER NOT NULL,
        term_id INTEGER NOT NULL,
        college_id INTEGER NOT NULL,
        major_id INTEGER NOT NULL,
        class_id INTEGER NOT NULL,
        gpa REAL,
        FOREIGN KEY (student_id) REFERENCES students (student_id),
        FOREIGN KEY (term_id) REFERENCES terms (term_id),
        FOREIGN KEY (college_id) REFERENCES colleges (college_id),
        FOREIGN KEY (major_id) REFERENCES majors (major_id),
        FOREIGN KEY (class_id) REFERENCES classes (class_id)
    );