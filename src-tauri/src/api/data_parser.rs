use super::csv_processor::{self, CsvTable};
use crate::api::csv_processor::get_file_name;
use crate::api::err::CustomError;
use futures::future::join_all;
use log::info;
use regex::Regex;
use std::{fs, path::PathBuf, sync::Arc};


pub struct CollegeData {
    pub term_name: Arc<String>,
    pub college_name: Arc<String>,
    pub data: Vec<CsvTable>,
}

pub struct DataProducer {
    tx: tokio::sync::mpsc::Sender<CollegeData>,
}

pub struct DataConsumer {
    rx: tokio::sync::mpsc::Receiver<CollegeData>,
}

impl DataConsumer {
    pub fn new(rx: tokio::sync::mpsc::Receiver<CollegeData>) -> Self {
        Self { rx }
    }
    pub async fn consume(&mut self) -> Vec<CollegeData> {
        let mut result = Vec::with_capacity(150);
        while let Some(college_data) = self.rx.recv().await {
            result.push(college_data);
        }
        result
    }
}

impl DataProducer {
    pub fn new(tx: tokio::sync::mpsc::Sender<CollegeData>) -> Self {
        Self { tx }
    }
    pub async fn produce(&self, path: PathBuf) -> Result<(), CustomError> {
        let re = Regex::new(r"^\d{2}.{2,10}$")?;
        let mut college_dirs = Vec::new();
        collect_college_dirs(&path, &re, &mut college_dirs)?;

        let mut tasks = Vec::with_capacity(150);

        for college_path in college_dirs {
            let (term_name, college_name) = parse_term_and_college_info(&college_path)?;
            let tx_clone = self.tx.clone();

            let task = tokio::task::spawn(async move {
                let csv_files = collect_csv_files(&college_path)?;
                let mut data = Vec::new();
                for csv_file in csv_files {
                    let csv_table = csv_processor::CsvTableBuilder::new(
                        &csv_file,
                    )
                    .build()?;

                    data.push(csv_table);
                }
                tx_clone.send(CollegeData {
                    term_name: term_name.clone(),
                    college_name: college_name.clone(),
                    data,
                }).await.expect("Failed to send csv table");

                Ok::<String, CustomError>(format!("{}-{} done", term_name, college_name))
            });
            tasks.push(task);
        }

        let results = join_all(tasks).await;
        // log the results
        for result in results {
            match result {
                Ok(message) => {
                    info!("{:?}", message);
                }
                Err(e) => {
                    log::error!("{:?}", e);
                }
            }
        }
        Ok(())
    }
}

fn collect_college_dirs(
    path: &PathBuf,
    re: &Regex,
    buf: &mut Vec<PathBuf>,
) -> Result<(), CustomError> {
    // get the files under the path
    let dirs = fs::read_dir(path)?;

    for entry in dirs {
        if let Ok(file) = entry {
            if file.path().is_dir() {
                if re.is_match(file.file_name().to_str().expect("Invalid file name")) {
                    buf.push(file.path());
                } else {
                    collect_college_dirs(&file.path(), re, buf)?;
                }
            }
        }
    }

    Ok(())
}

fn collect_csv_files(dir_path: &PathBuf) -> Result<Vec<PathBuf>, CustomError> {
    let mut csv_files = Vec::new();
    let re = Regex::new(r"^[a-z]\d{2}(\D*)(\d{4})hz.csv$")?;
    let files = fs::read_dir(dir_path)?;

    for entry in files {
        if let Ok(file) = entry {
            if file.path().is_file() {
                if let Some(file_name) = file.file_name().to_str() {
                    if re.is_match(file_name) {
                        csv_files.push(file.path());
                    }
                }
            }
        }
    }

    Ok(csv_files)
}

fn parse_term_and_college_info(
    college_path: &PathBuf,
) -> Result<(Arc<String>, Arc<String>), CustomError> {
    let college_name: String = {
        let t = get_file_name(college_path)?;
        t.chars().skip(2).collect()
    };
    let term_path = college_path
        .parent()
        .ok_or(CustomError::UnexpectedFileError(
            college_path.to_string_lossy().to_string(),
        ))?
        .to_path_buf();
    verify_file_name(&term_path, r"^\d{4}-\d{4}-\d学期智育学分绩$")?;
    let term: String = {
        let term_str = get_file_name(&term_path)?;
        term_str.chars().take(11).collect()
    };

    Ok((Arc::new(term), Arc::new(college_name)))
}

/// 校验文件名称
fn verify_file_name(file_path: &PathBuf, re_str: &str) -> Result<(), CustomError> {
    let re = Regex::new(re_str)?;

    let file_name = get_file_name(file_path)?;
    if !re.is_match(file_name) {
        return Err(CustomError::UnexpectedFileError(format!(
            "{} 文件名不符合要求",
            file_name
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    fn data_dir() -> Result<PathBuf, Box<dyn Error>> {
        let mut current_dir = std::env::current_dir()?;
        current_dir.push("test\\data");
        Ok(current_dir)
    }

    #[tokio::test]
    async fn test_data_producer_consumer() -> Result<(), Box<dyn Error>> {
        let start = std::time::Instant::now();

        let (tx, rx) = mpsc::channel(32);
        let producer = DataProducer::new(tx);
        let mut consumer = DataConsumer::new(rx);

        let temp_path = data_dir()?;

        let producer_task = tokio::spawn(async move {
            producer.produce(temp_path).await.unwrap();
        });

        let consumer_task = tokio::spawn(async move { consumer.consume().await });

        let (producer_result, consumer_result) = tokio::join!(producer_task, consumer_task);

        let end = std::time::Instant::now();

        if let Err(e) = producer_result {
            eprintln!("Producer task failed: {:?}", e);
        }

        if let Err(e) = consumer_result {
            eprintln!("Consumer task failed: {:?}", e);
        } else if let Ok(records) = consumer_result {
            assert_eq!(records.len(), 3571);
            println!("Records count: {}", records.len());
        }

        println!("Time elapsed: {:?}", end - start);
        Ok(())
    }

    #[test]
    fn test_collect_college_dirs() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create dummy directories
        let college_dir = temp_path.join("21College");
        fs::create_dir_all(&college_dir).unwrap();

        let re = Regex::new(r"^\d{2}.{2,10}$").unwrap();
        let mut college_dirs = Vec::new();
        collect_college_dirs(&temp_path, &re, &mut college_dirs).unwrap();

        assert_eq!(college_dirs.len(), 1);
        assert_eq!(college_dirs[0], college_dir);
    }

    #[test]
    fn test_collect_csv_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create dummy CSV files
        let csv_file = temp_path.join("a21test2021hz.csv");
        fs::write(&csv_file, "dummy content").unwrap();

        let csv_files = collect_csv_files(&temp_path).unwrap();

        assert_eq!(csv_files.len(), 1);
        assert_eq!(csv_files[0], csv_file);
    }

    #[test]
    fn test_parse_term_and_college_info() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create dummy directories
        let term_dir = temp_path.join("2021-2022-1学期智育学分绩");
        let college_dir = term_dir.join("21College");
        fs::create_dir_all(&college_dir).unwrap();

        let (term, college_name) = parse_term_and_college_info(&college_dir).unwrap();

        assert_eq!(*term, "2021-2022-1");
        assert_eq!(*college_name, "College");
    }

    #[test]
    fn test_verify_file_name() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create dummy directory
        let term_dir = temp_path.join("2021-2022-1学期智育学分绩");
        fs::create_dir_all(&term_dir).unwrap();

        let result = verify_file_name(&term_dir, r"^\d{4}-\d{4}-\d学期智育学分绩$");

        assert!(result.is_ok());
    }
}
