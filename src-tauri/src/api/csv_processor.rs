use std::{error::Error, path::PathBuf, sync::Arc};

use csv;
use regex::Regex;
use serde::Deserialize;

use super::student;

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

type CsvRecords = Vec<RowRecord>;

// csv表
pub struct CsvTable {
    pub records: CsvRecords,
    pub info: student::AcademicInfo,
}

// csv表构建器
pub struct CsvTableBuilder<'builder> {
    term: Arc<String>,
    college: Arc<String>,
    csv_path: &'builder PathBuf,
}

impl<'builder> CsvTableBuilder<'builder> {
    pub fn new(term: Arc<String>, college: Arc<String>, csv_path: &'builder PathBuf) -> Self {
        Self {
            term,
            college,
            csv_path,
        }
    }

    pub fn build(&self) -> Result<CsvTable, Box<dyn Error>> {
        let (major, class) = self.extract_major_and_class_info()?;
        let records = self.build_csv_records()?;
        let info = student::AcademicInfo::new(
            self.term.clone(),
            self.college.clone(),
            Arc::new(major),
            Arc::new(class),
        );

        Ok(CsvTable {
            records: records,
            info,
        })
    }

    fn build_csv_records(&self) -> Result<CsvRecords, Box<dyn Error>> {
        let file = std::fs::File::open(self.csv_path)?;
        let mut records: CsvRecords = vec![];
        let mut rdr = csv::Reader::from_reader(file);

        // special judge for the first line
        let header_record = rdr.records().next();
        let has_valid_gpa = {
            match header_record {
                Some(record) => check_gpa_column(&record?),
                None => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Empty file",
                    )))
                }
            }
        };

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
                    let sid = record.get(0).unwrap(); // todo: handle error
                    let name = record.get(1).unwrap();
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

    fn extract_major_and_class_info(&self) -> Result<(String, String), Box<dyn Error>> {
        let re = Regex::new(r"^[a-z]\d{2}([^\d]*)(\d{4})hz.csv$").unwrap();
        if re.is_match(self.csv_path.file_name().unwrap().to_str().unwrap()) {
            let captures = re
                .captures(self.csv_path.file_name().unwrap().to_str().unwrap())
                .expect("Invalid file name");
            let major_name = captures.get(1).map_or("", |m| m.as_str()).to_string();
            let class_name = captures.get(2).map_or("", |m| m.as_str()).to_string();

            Ok((major_name, class_name))
        } else {
            Err("Invalid file name".into())
        }
    }
}

/// 判断csv是否有绩点列
fn check_gpa_column(record: &csv::StringRecord) -> bool {
    let column_three = record.iter().skip(2).take(1).next();
    match column_three {
        Some(name) => {
            let re = Regex::new(r"^\d{5}\|\d\.\d\|\d{4}-\d{4}-\d智育学分绩\|\|$").unwrap();
            re.is_match(name)
        }
        None => false,
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    #[test]
    fn test_extract_major_and_class_info_valid() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("a22major2022hz.csv");
        File::create(&file_path).unwrap();

        let builder = CsvTableBuilder::new(
            Arc::new("2022".to_string()),
            Arc::new("Engineering".to_string()),
            &file_path,
        );

        let (major, class) = builder.extract_major_and_class_info().unwrap();
        assert_eq!(major, "major");
        assert_eq!(class, "2022");
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
        assert!(check_gpa_column(&record));
    }

    #[test]
    fn test_check_gpa_column_invalid() {
        let record = csv::StringRecord::from(vec!["xh", "xm", "invalid_column"]);
        assert!(!check_gpa_column(&record));
    }
}
