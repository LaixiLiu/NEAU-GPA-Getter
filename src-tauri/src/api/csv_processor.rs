use csv;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;

use super::err::CustomError;

// 行记录
#[derive(Deserialize)]
pub struct RowRecord {
    // 学号
    #[serde(rename = "xh")]
    pub sid: String,
    // 姓名
    #[serde(rename = "xm")]
    pub name: String,
    // 绩点
    #[serde(rename = "k101")]
    #[serde(deserialize_with = "csv::invalid_option")]
    pub gpa: Option<f64>,
}

pub type CsvRecords = Vec<RowRecord>;

// csv表
pub struct CsvTable {
    pub records: CsvRecords,
    pub major_name: String,
    pub class_name: String,
}

// csv表构建器
pub struct CsvTableBuilder<'builder> {
    csv_path: &'builder PathBuf,
}

impl<'builder> CsvTableBuilder<'builder> {
    pub fn new(csv_path: &'builder PathBuf) -> Self {
        Self { csv_path }
    }

    pub fn build(&self) -> Result<CsvTable, CustomError> {
        let (major_name, class_name) = self.extract_major_and_class_info()?;
        let records = self.build_csv_records()?;

        Ok(CsvTable {
            records,
            major_name,
            class_name,
        })
    }

    /// 从csv文件中构建记录
    /// 如果csv文件中没有绩点列，则绩点为None
    ///
    /// # Errors
    ///
    /// 如果文件读取失败，返回`CustomError::FileReadError`
    /// 如果文件中的数据不符合预期，返回`CustomError::CsvDataError`
    /// 如果文件名不符合规范，返回`CustomError::IllegalFileError`
    /// 如果csv解析失败，返回`CustomError::CsvParseError`
    fn build_csv_records(&self) -> Result<CsvRecords, CustomError> {
        let file = std::fs::File::open(self.csv_path)?;
        let mut records: CsvRecords = vec![];
        let mut rdr = csv::Reader::from_reader(file);

        // 特判空文件
        let header_record = rdr.records().next();
        let has_valid_gpa = {
            match header_record {
                Some(record) => check_gpa_column(&record?)?,
                None => {
                    return Err(CustomError::CsvDataError(
                        self.csv_path.to_string_lossy().to_string(),
                    ))
                }
            }
        };

        // 判读是否有绩点列
        match has_valid_gpa {
            true => {
                let record_iter = rdr.deserialize();

                for rd in record_iter {
                    let record = rd?;
                    records.push(record);
                }
            }
            false => {
                let record_iter = rdr.records();

                for record in record_iter {
                    let record = record?;
                    let sid = record.get(0).ok_or(CustomError::CsvDataError(format!(
                        "缺失学号信息: {:?}",
                        self.csv_path
                    )))?;
                    let name = record.get(1).ok_or(CustomError::CsvDataError(format!(
                        "缺失姓名信息: {:?}",
                        self.csv_path
                    )))?;
                    let gpa = None;
                    let student = RowRecord {
                        sid: sid.to_string(),
                        name: name.to_string(),
                        gpa,
                    };
                    records.push(student);
                }
            }
        }
        Ok(records)
    }

    /// 从文件名中提取专业和班级信息
    ///
    /// # Errors
    ///
    /// 如果文件名不符合规范，返回`CustomError::IllegalFileError`
    /// regex构建失败或解析失败，返回`CustomError::RegexError`
    /// 如果出现了预期外的文件，返回`CustomError::UnexpectedFileError`
    fn extract_major_and_class_info(&self) -> Result<(String, String), CustomError> {
        let re = Regex::new(r"^[a-z](\d{2})((\D*)\d{4})hz.csv$")?;
        let file_name = get_file_name(self.csv_path)?;
        if re.is_match(file_name) {
            let captures = re.captures(file_name).expect("Regex match failed"); // safe to unwrap
            let class_name = captures.get(2).map_or("", |m| m.as_str()).to_string();
            let major_name = captures.get(3).map_or("", |m| m.as_str()).to_string();

            Ok((major_name, class_name))
        } else {
            Err(CustomError::UnexpectedFileError(file_name.to_string()))
        }
    }
}

/// 判断csv是否有绩点列
fn check_gpa_column(record: &csv::StringRecord) -> Result<bool, CustomError> {
    let column_three = record.iter().skip(2).take(1).next();
    match column_three {
        Some(name) => {
            let re = Regex::new(r"^\d{5}\|\d\.\d\|\d{4}-\d{4}-\d智育学分绩\|\|$")?;
            Ok(re.is_match(name))
        }
        None => Ok(false),
    }
}

/// 解析文件名
pub fn get_file_name(file: &PathBuf) -> Result<&str, CustomError> {
    let file_name_os = file.file_name().ok_or(CustomError::IllegalFileError(
        file.to_string_lossy().to_string(),
    ))?;
    let file_name = file_name_os.to_str().ok_or(CustomError::IllegalFileError(
        file.to_string_lossy().to_string(),
    ))?;
    Ok(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    #[test]
    fn test_extract_major_and_class_info_valid() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("a22major2012hz.csv");
        File::create(&file_path).unwrap();

        let builder = CsvTableBuilder::new(
            Arc::new("2022-2023-1".to_string()),
            Arc::new("Engineering".to_string()),
            &file_path,
        );

        let (major, class) = builder.extract_major_and_class_info().unwrap();
        assert_eq!(major, "major");
        assert_eq!(class, "major2012");
    }

    #[test]
    fn test_extract_major_and_class_info_invalid() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("invalid_filename.csv");
        File::create(&file_path).unwrap();

        let builder = CsvTableBuilder::new(
            Arc::new("2022".to_string()),
            Arc::new("Engineering".to_string()),
            &file_path,
        );

        let result = builder.extract_major_and_class_info();
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_major_and_class_info_regex_error() {
        // Inject an invalid regex pattern
        let result = Regex::new(r"[").map_err(|e| CustomError::RegexError(e));
        assert!(result.is_err());
    }

    #[test]
    fn test_build_csv_records_with_gpa() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_with_gpa.csv");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "xh,xm,k101").unwrap();
        writeln!(file, ",,00231|0.0|2022-2023-1智育学分绩||").unwrap();
        writeln!(file, "12345,John Doe,3.5").unwrap();

        let builder = CsvTableBuilder::new(
            Arc::new("2022".to_string()),
            Arc::new("Engineering".to_string()),
            &file_path,
        );

        let records = builder.build_csv_records().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].sid, "12345");
        assert_eq!(records[0].name, "John Doe");
        assert_eq!(records[0].gpa, Some(3.5));
    }

    #[test]
    fn test_build_csv_records_without_gpa() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_without_gpa.csv");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "xh,xm,k101").unwrap();
        writeln!(file, ",,00231|0.0|2022-2023-1智育学分绩||").unwrap();
        writeln!(file, "12345,John Doe,").unwrap();

        let builder = CsvTableBuilder::new(
            Arc::new("2022".to_string()),
            Arc::new("Engineering".to_string()),
            &file_path,
        );

        let records = builder.build_csv_records().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].sid, "12345");
        assert_eq!(records[0].name, "John Doe");
        assert_eq!(records[0].gpa, None);
    }

    #[test]
    fn test_check_gpa_column_valid() {
        let record = csv::StringRecord::from(vec!["xh", "xm", "00101|3.5|2022-2023-1智育学分绩||"]);
        assert!(check_gpa_column(&record).unwrap());
    }

    #[test]
    fn test_check_gpa_column_invalid() {
        let record = csv::StringRecord::from(vec!["xh", "xm", "invalid_column"]);
        assert!(!check_gpa_column(&record).unwrap());
    }

    #[test]
    fn test_check_gpa_column_missing() {
        let record = csv::StringRecord::from(vec!["xh", "xm"]);
        assert!(!check_gpa_column(&record).unwrap());
    }

    #[test]
    fn test_build_csv_records_file_read_error() {
        let csv_path = PathBuf::from("non_existent_file.csv");
        let builder = CsvTableBuilder::new(
            Arc::new("2022".to_string()),
            Arc::new("Engineering".to_string()),
            &csv_path,
        );

        let result = builder.build_csv_records();
        assert!(matches!(result, Err(CustomError::FileReadError(_))));
    }

    #[test]
    fn test_build_csv_records_csv_parse_error() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_invalid_csv.csv");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "invalid_csv_content").unwrap();

        let builder = CsvTableBuilder::new(
            Arc::new("2022".to_string()),
            Arc::new("Engineering".to_string()),
            &file_path,
        );

        let result = builder.build_csv_records();
        assert!(matches!(result, Err(CustomError::CsvDataError(_))));
    }
}
