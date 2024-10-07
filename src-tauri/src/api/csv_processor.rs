use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{self, PathBuf},
};

use csv;
use regex::Regex;
use serde::Deserialize;

use super::student;

/// 序列化后的表格记录
#[derive(Debug, Deserialize)]
pub struct StudentRecord {
    /// 学号
    #[serde(rename = "xh")]
    pub sid: String,
    /// 姓名
    #[serde(rename = "xm")]
    pub name: String,
    /// 该学期的绩点
    #[serde(rename = "k101")]
    #[serde(deserialize_with = "csv::invalid_option")]
    pub gpa: Option<f64>,
}

#[derive(Debug)]
pub struct GradeTable {
    // 专业名称
    pub major: String,
    // 班级
    pub class: String,
    // 对应的表格记录
    pub data: Vec<StudentRecord>,
}

// TODO: 添加多线程解析数据的功能
/// 单线程获取指定路径下的所有csv文件数据
pub fn extract_data_from_files(
    path: &str,
) -> Result<HashMap<String, student::ParsedStudentData>, Box<dyn Error>> {
    let mut result = HashMap::new();
    let data_path = get_valid_data_paths(path)?;
    for path in data_path {
        parse_term_data(&path, &mut result)?;
    }
    Ok(result)
}

pub fn parse_term_data(
    path: &PathBuf,
    data: &mut HashMap<String, student::ParsedStudentData>,
) -> Result<(), Box<dyn Error>> {
    let term: String = {
        let dir_name = path.file_name().unwrap().to_str().unwrap();
        dir_name.chars().take(11).collect()
    };
    let college_dirs = fs::read_dir(path).expect("读取目录失败");

    let re = Regex::new(r"^[a-z]\d{2}([^\d]*)(\d{4})hz.csv$").unwrap();

    for entry in college_dirs {
        let college_path = entry.unwrap().path();
        if let Ok(()) = verify_file_name(&college_path, r"^\d{2}.*$") {
            // 获取学院信息
            let college_name: String = {
                let dir_name = college_path.file_name().unwrap().to_str().unwrap();
                dir_name.chars().skip(2).collect()
            };

            // 获取学院目录下的所有csv文件
            let tables = fs::read_dir(college_path).expect("读取目录失败");
            for wrapped_table in tables {
                let table = wrapped_table.expect("读取文件失败");
                // 解析表格信息
                let table_name = table.file_name().to_str().unwrap().to_string();
                let (major, class) = {
                    let captures = re
                        .captures(&table_name)
                        .expect(format!("{} 文件名不符合要求", table_name).as_str());
                    let major_name = captures.get(1).map_or("", |m| m.as_str()).to_string();
                    let class_name = captures.get(2).map_or("", |m| m.as_str()).to_string();

                    (major_name, class_name)
                };

                let row_records = extract_csv_data(&table.path())?;
                // 解析并保存表格记录
                for record in row_records {
                    data.entry(record.sid.clone())
                        .and_modify(|student_record| {
                            student_record.records.push(student::AcademicRecord::new(
                                term.clone(),
                                record.gpa,
                                college_name.clone(),
                                class.clone(),
                                major.clone(),
                            ));
                        })
                        .or_insert(student::ParsedStudentData::new(
                            student::Student::new(record.sid, record.name),
                            vec![student::AcademicRecord::new(
                                term.clone(),
                                record.gpa,
                                college_name.clone(),
                                class.clone(),
                                major.clone(),
                            )],
                        ));
                }
            }
        }
    }

    Ok(())
}

fn verify_file_name(file_path: &path::PathBuf, re_str: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(re_str).unwrap();

    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    if !re.is_match(file_name) {
        return Err(format!("{} 文件名不符合正则表达式: {}", file_name, re_str).into());
    }

    Ok(())
}

/// 解析指定路径下的csv文件
///
/// # Arguments
///
/// * `file_path` - csv文件的路径
///
/// # Returns
///
/// 返回解析后的表格记录，如果解析失败则返回错误信息
fn extract_csv_data(file_path: &path::PathBuf) -> Result<Vec<StudentRecord>, Box<dyn Error>> {
    verify_file_name(file_path, r"^[a-z]\d{2}([^\d]*)(\d{4})hz.csv$")
        .expect(format!("{} 文件名不符合要求", file_path.display()).as_str());
    let file = std::fs::File::open(file_path)?;
    let mut result = vec![];
    let mut rdr = csv::Reader::from_reader(file);
    let mut record_iter = rdr.deserialize();
    // skip the first record
    record_iter.next();
    for rd in record_iter {
        let record: StudentRecord = rd?;
        result.push(record);
    }
    Ok(result)
}

/// 测试数据路径是否符合格式要求
fn get_valid_data_paths(dir_path: &str) -> Result<Vec<path::PathBuf>, Box<dyn Error>> {
    let path = path::Path::new(dir_path);
    if !path.is_dir() {
        return Err(format!("请提供合法的文件所在目录路径: {}", dir_path).into());
    }

    let mut data_path = vec![];

    // 获取dir_path下的所有目录
    // 保存符合格式要求的目录
    let dirs = fs::read_dir(dir_path)?;
    for entry in dirs {
        match entry {
            Ok(dir) if dir.path().is_dir() => {
                let path = dir.path();
                if let Ok(()) = verify_file_name(&path, r"^\d{4}-\d{4}-\d学期智育学分绩$") {
                    data_path.push(path);
                }
            }
            Ok(_) => {}
            Err(e) => {
                panic!("读取目录失败: {}", e);
            }
        }
    }

    if data_path.len() < 1 {
        return Err(
            "请确保选择的路径下存在名称为如下格式的目录: 20xx-20xx-x学期智育学分绩"
                .to_string()
                .into(),
        );
    }

    for path_t in &data_path {
        // 判断dir_path下是否为学院目录
        // 学院目录名称为 xx yy
        // xx 为两位数字，yy 为学院简称
        let dirs = fs::read_dir(path_t)?;
        for entry in dirs {
            match entry {
                Ok(dir) if dir.path().is_dir() => {
                    let path = dir.path();
                    if let Err(_) = verify_file_name(&path, r"^\d{2}.*$") {
                        return Err(format!(
                            "{} 目录名称不符合如下格式: 19电信\n请检查是否选择了正确的目录或者是否对目录下的文件进行了修改",
                            path.display()
                        )
                        .into());
                    }
                }
                Ok(file) => {
                    let path = file.path();
                    return Err(format!("{} 不是目录", path.display()).into());
                }
                Err(e) => {
                    panic!("读取目录失败: {}", e);
                }
            }
        }
    }

    Ok(data_path)
}

#[cfg(test)]
mod tests {
    use std::panic;

    use super::*;

    #[test]
    fn test_parse_csv_correct_file() {
        // 获取cargo工作目录
        let mut file_path = std::env::current_dir().unwrap();
        file_path.push("test\\data\\2022-2023-1学期智育学分绩\\01农学\\b01农学1901hz.csv");
        let result = extract_csv_data(&file_path);
        assert!(result.is_ok());
        let records = result.unwrap();
        assert_eq!(records.len(), 29);
        assert_eq!(records[0].sid, "A01190013");
        assert_eq!(records[0].name, "赵瑞轩");
        assert_eq!(records[0].gpa, Some(4.1400));
    }

    #[test]
    #[should_panic]
    fn test_parse_csv_invalid_path() {
        let mut file_path = std::env::current_dir().unwrap();
        file_path.push("test\\data\\2022-2023-1学期智育学分绩\\01农学\\01农101hz.csv");
        extract_csv_data(&file_path).unwrap();
    }

    #[test]
    fn test_parse_data() {
        let mut file_path = std::env::current_dir().unwrap();
        // let file_path = path::PathBuf::from(".\\test\\data\\2022-2023-1学期智育学分绩");
        file_path.push("test\\data\\2022-2023-1学期智育学分绩");
        assert_eq!(
            file_path.display().to_string(),
            r"D:\project\web\neau-gpa-getter\src-tauri\test\data\2022-2023-1学期智育学分绩"
        );
    }

    #[test]
    fn test_handles_invalid_data_path() {
        let dir_path = "D:\\path\\to\\dir";
        match get_valid_data_paths(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(
                e.to_string(),
                "请提供合法的文件所在目录路径: D:\\path\\to\\dir"
            ),
        }
        let dir_path = r"D:\project\web\neau-gpa-getter\src-tauri\test";
        match get_valid_data_paths(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(
                e.to_string(),
                "请确保选择的路径下存在名称为如下格式的目录: 20xx-20xx-x学期智育学分绩"
            ),
        }
        let dir_path = r"D:\project\web\neau-gpa-getter\src-tauri\test\data\extra";
        match get_valid_data_paths(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(
                e.to_string(),
                format!( "{}\\2030-2031-1学期智育学分绩\\test 目录名称不符合如下格式: 19电信\n请检查是否选择了正确的目录或者是否对目录下的文件进行了修改", dir_path )
            ),
        }
    }

    #[test]
    fn test_validate_data_path() {
        let dir_path = r"D:\project\web\neau-gpa-getter\src-tauri\test\data";
        let mut data_path = vec![];

        let dirs = std::fs::read_dir(dir_path).unwrap();
        for entry in dirs {
            match entry {
                Ok(dir) => {
                    let path = dir.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let dir_name = path.file_name().unwrap().to_str().unwrap();
                    let re = Regex::new(r"^\d{4}-\d{4}-\d学期智育学分绩$").unwrap();
                    if re.is_match(dir_name) {
                        data_path.push(path);
                    }
                }
                Err(e) => {
                    panic!("读取目录失败: {}", e);
                }
            }
        }
        assert_eq!(data_path.len(), 4);
        let result = get_valid_data_paths(dir_path);
        assert!(result.is_ok());
        let result_path = result.unwrap();
        assert_eq!(result_path.len(), 4);
        // data_path and result_path should equal
        assert_eq!(data_path, result_path);
    }
}
