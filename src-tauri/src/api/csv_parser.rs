use std::{error::Error, fs, path};

use csv;
use regex::Regex;
use serde::Deserialize;

/// 序列化后的表格记录
#[derive(Debug, Deserialize)]
struct Record {
    /// 学号
    #[serde(rename = "xh")]
    sid: String,
    /// 姓名
    #[serde(rename = "xm")]
    name: String,
    /// 该学期的绩点
    #[serde(rename = "k101")]
    #[serde(deserialize_with = "csv::invalid_option")]
    gpa: Option<f64>,
}

#[derive(Debug)]
struct Table {
    // 专业名称
    major: String,
    // 班级名称
    class: String,
    // 对应的表格记录
    data: Vec<Record>,
}

/// 用于描述一个学院及其下的所有表格的信息
#[derive(Debug)]
pub struct College {
    // 学院id
    id: u8,
    // 学院名称
    name: String,
    // 该学院下所有表格的名称
    data: Vec<Table>,
}

impl College {
    fn new(id: u8, name: String, data: Vec<Table>) -> Self {
        College { id, name, data }
    }
}

// TODO: 添加多线程解析数据的功能
/// 单线程获取指定路径下的所有csv文件数据
pub fn get_data(path: &str) -> Result<Vec<College>, Box<dyn Error>> {
    let mut result = vec![];
    let data_path = judge_data_path(path)?;
    for path in data_path {
        let data = parse_data(&path)?;
        result.extend(data);
    }
    Ok(result)
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
fn parse_csv(file_path: &path::PathBuf) -> Result<Vec<Record>, Box<dyn Error>> {
    let file = std::fs::File::open(file_path)?;
    let mut result = vec![];
    let mut rdr = csv::Reader::from_reader(file);
    let mut record_iter = rdr.deserialize();
    // skip the first record
    record_iter.next();
    for rd in record_iter {
        let record: Record = rd?;
        result.push(record);
    }
    Ok(result)
}

/// 解析指定路径下的所有csv文件数据
///
/// # Arguments
///
/// * `dir_path` - csv文件所在的目录路径
/// 格式为: "D:\\path\\to\\dir"
/// 其中，`dir`名称格式为: "年份-学期-学期类型智育学分绩", 如"2022-2023-1学期智育学分绩"
///
/// # Returns
///
/// 返回解析后的表格记录，如果解析失败则返回错误信息
fn parse_data(dir_path: &path::PathBuf) -> Result<Vec<College>, Box<dyn Error>> {
    let mut result = vec![];
    let dirs = fs::read_dir(dir_path).expect("读取目录失败");

    let re = Regex::new(r"^[a-z]\d{2}([^\d]*)(\d{4})hz.csv$").unwrap();

    // 按学院遍历某一学期的智育学分绩目录
    for entry in dirs {
        match entry {
            Ok(dir) => {
                // 解析学院信息
                let temp = dir.file_name();
                let college_dir_name = temp.to_str().expect(
                    format!("{} 不是有效的目录名称", dir.file_name().to_str().unwrap()).as_str(),
                );
                let college_id: u8 = college_dir_name[0..2]
                    .parse()
                    .expect(format!("{} 不包含有效的学院编号", college_dir_name).as_str());
                let college_name = college_dir_name[2..].to_string();

                // 解析学院目录下的所有csv文件
                let college = {
                    let tables = fs::read_dir(dir.path()).expect("读取目录失败");
                    let mut data = vec![];
                    for entry in tables {
                        match entry {
                            Ok(table) => {
                                let result = parse_csv(&table.path());
                                match result {
                                    Ok(records) => {
                                        // 解析专业，班级信息
                                        let table_name = table.file_name();
                                        let table_name =
                                            table_name.to_str().expect("文件名不是有效的字符串");

                                        let captures = re
                                            .captures(table_name)
                                            .expect("文件名不符合正则表达式");
                                        let major_name =
                                            captures.get(1).map_or("", |m| m.as_str()).to_string();
                                        let class_name =
                                            captures.get(2).map_or("", |m| m.as_str()).to_string();

                                        let table = Table {
                                            major: major_name,
                                            class: class_name,
                                            data: records,
                                        };
                                        data.push(table);
                                    }
                                    Err(e) => {
                                        panic!("解析表格失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                panic!("读取目录失败: {}", e);
                            }
                        }
                    }
                    College::new(college_id, college_name, data)
                };
                result.push(college);
            }
            Err(e) => {
                panic!("读取目录失败: {}", e);
            }
        }
    }

    Ok(result)
}

/// 测试数据路径是否符合格式要求
fn judge_data_path(dir_path: &str) -> Result<Vec<path::PathBuf>, Box<dyn Error>> {
    let path = path::Path::new(dir_path);
    if !path.is_dir() {
        return Err(format!("无法访问: {}", dir_path).into());
    }

    let mut data_path = vec![];

    // 获取dir_path下的所有目录
    // 保存符合格式要求的目录
    let dirs = fs::read_dir(dir_path)?;
    let re = Regex::new(r"^\d{4}-\d{4}-\d学期智育学分绩$").unwrap();
    for entry in dirs {
        match entry {
            Ok(dir) => {
                let path = dir.path();
                if !path.is_dir() {
                    continue;
                }
                let dir_name = path.file_name().unwrap().to_str().unwrap();
                if re.is_match(dir_name) {
                    data_path.push(path);
                }
            }
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

    let re = Regex::new(r"^\d{2}.*$").unwrap();
    for path_t in &data_path {
        // 判断dir_path下是否为学院目录
        // 学院目录名称为 xx yy
        // xx 为两位数字，yy 为学院简称
        let dirs = fs::read_dir(path_t)?;
        for entry in dirs {
            match entry {
                Ok(dir) => {
                    let path = dir.path();
                    if !path.is_dir() {
                        return Err(format!("{} 不是目录", path.display()).into());
                    }
                    let dir_name = path.file_name().unwrap().to_str().unwrap();
                    if !re.is_match(dir_name) {
                        return Err(format!(
                            "{} 目录名称不符合如下格式: 19电信\n请检查是否选择了正确的目录或者是否对目录下的文件进行了修改",
                            path.display()
                        )
                        .into());
                    }
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
    fn test_parse_csv() {
        // 获取cargo工作目录
        let mut file_path = std::env::current_dir().unwrap();
        file_path.push("test\\data\\2022-2023-1学期智育学分绩\\01农学\\b01农学1901hz.csv");
        let result = parse_csv(&file_path);
        assert!(result.is_ok());
        let records = result.unwrap();
        assert_eq!(records.len(), 29);
        assert_eq!(records[0].sid, "A01190013");
        assert_eq!(records[0].name, "赵瑞轩");
        assert_eq!(records[0].gpa, Some(4.1400));
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
        let result = parse_data(&file_path).unwrap();
        for college in &result {
            println!("{:?}", college.name);
            for table in &college.data {
                println!("{:?}", table.major);
                println!("{:?}", table.class);
                // for record in &table.data {
                //     println!("{:?}", record.sid);
                //     println!("{:?}", record.name);
                //     println!("{:?}", record.gpa);
                // }
            }
        }
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_illegal_data_path() {
        let dir_path = r"path\to\dir";
        match judge_data_path(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(e.to_string(), "无法访问: path\\to\\dir"),
        }
        let dir_path = "D:\\path\\to\\dir";
        match judge_data_path(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(e.to_string(), "无法访问: D:\\path\\to\\dir"),
        }
        let dir_path = r"D:\project\web\neau-gpa-getter\src-tauri\test";
        match judge_data_path(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(
                e.to_string(),
                "请确保选择的路径下存在名称为如下格式的目录: 20xx-20xx-x学期智育学分绩"
            ),
        }
        let dir_path = r"D:\project\web\neau-gpa-getter\src-tauri\test\data\extra";
        match judge_data_path(dir_path) {
            Ok(_) => panic!("非法文件路径但未检测出"),
            Err(e) => assert_eq!(
                e.to_string(),
                format!( "{}\\2030-2031-1学期智育学分绩\\test 目录名称不符合如下格式: 19电信\n请检查是否选择了正确的目录或者是否对目录下的文件进行了修改", dir_path )
            ),
        }
    }

    #[test]
    fn test_legal_data_path() {
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
        let result = judge_data_path(dir_path);
        assert!(result.is_ok());
        let result_path = result.unwrap();
        assert_eq!(result_path.len(), 4);
        // data_path and result_path should equal
        assert_eq!(data_path, result_path);
    }
}
