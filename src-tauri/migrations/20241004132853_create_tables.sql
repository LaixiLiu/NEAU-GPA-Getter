-- Add migration script here
-- 启用外键约束
PRAGMA foreign_keys = ON;

CREATE TABLE
    IF NOT EXISTS students (id TEXT PRIMARY KEY, name TEXT NOT NULL);

CREATE TABLE
    IF NOT EXISTS records (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        student_id TEXT NOT NULL,
        term TEXT NOT NULL,
        college TEXT NOT NULL,
        class TEXT NOT NULL,
        major TEXT NOT NULL,
        gpa REAL NOT NULL,
        FOREIGN KEY (student_id) REFERENCES students (id)
    );