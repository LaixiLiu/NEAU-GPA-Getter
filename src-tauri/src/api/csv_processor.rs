use std::{error::Error, path::PathBuf, sync::Arc};

use csv;
use regex::Regex;
use serde::Deserialize;

use super::student;

// 行记录
#[derive(Debug, Deserialize)]
struct RowRecord {
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
    records: CsvRecords,
    info: student::AcademicInfo,
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
        let info =
            student::AcademicInfo::new(self.term.clone(), self.college.clone(), major, class);

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
mod tests {}
