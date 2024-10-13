use super::csv_processor;
use futures::future::join_all;
use regex::Regex;
use std::{error::Error, fs, path::PathBuf, sync::Arc};

pub struct DataProducer {
    tx: tokio::sync::mpsc::Sender<Vec<csv_processor::CsvTable>>,
}

pub struct DataConsumer {
    rx: tokio::sync::mpsc::Receiver<Vec<csv_processor::CsvTable>>,
}

impl DataConsumer {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Vec<csv_processor::CsvTable>>) -> Self {
        Self { rx }
    }
    pub async fn consume(&mut self) -> Result<Vec<csv_processor::CsvTable>, Box<dyn Error>> {
        let mut result = Vec::with_capacity(1000);
        while let Some(csv_table) = self.rx.recv().await {
            result.extend(csv_table);
        }
        Ok(result)
    }
}

impl DataProducer {
    pub fn new(tx: tokio::sync::mpsc::Sender<Vec<csv_processor::CsvTable>>) -> Self {
        Self { tx }
    }
    pub async fn produce(&self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let re = Regex::new(r"^\d{2}.{2,10}$").unwrap();
        let mut college_dirs = Vec::new();
        collect_college_dirs(&path, &re, &mut college_dirs)?;

        let mut tasks = Vec::with_capacity(500);

        for college_path in college_dirs {
            let (term, college_name) = parse_term_and_college_info(&college_path)?;
            let tx_clone = self.tx.clone();

            let task = tokio::task::spawn(async move {
                if let Ok(csv_files) = collect_csv_files(&college_path) {
                    let mut data = Vec::new();
                    for csv_file in csv_files {
                        let csv_table = csv_processor::CsvTableBuilder::new(
                            Arc::clone(&term),
                            Arc::clone(&college_name),
                            &csv_file,
                        )
                        .build()
                        .expect("Failed to build csv table");
                        data.push(csv_table);
                    }
                    tx_clone.send(data).await.expect("Failed to send csv table");
                } else {
                    // TODO: handle the error
                    eprintln!("Failed to collect csv files under {:?}", college_path);
                }
            });
            tasks.push(task);
        }

        join_all(tasks).await;
        Ok(())
    }
}

fn collect_college_dirs(
    path: &PathBuf,
    re: &Regex,
    buf: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
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

fn collect_csv_files(dir_path: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn Error + Send + Sync>> {
    let mut csv_files = Vec::new();
    let re = Regex::new(r"^[a-z]\d{2}([^\d]*)(\d{4})hz.csv$").expect("build regex failed");
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
) -> Result<(Arc<String>, Arc<String>), Box<dyn Error>> {
    let college_name: String = {
        let college_name = college_path.file_name().unwrap().to_str().unwrap();
        college_name.chars().skip(2).collect()
    };
    let term_path = college_path.parent().unwrap().to_path_buf();
    let term = match verify_file_name(&term_path, r"^\d{4}-\d{4}-\d学期智育学分绩$") {
        Ok(()) => term_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .chars()
            .take(11)
            .collect(),
        Err(e) => {
            return Err(e);
        }
    };

    Ok((Arc::new(term), Arc::new(college_name)))
}

/// 校验文件名称
fn verify_file_name(file_path: &PathBuf, re_str: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(re_str).unwrap();

    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    if !re.is_match(file_name) {
        return Err(format!("{} 文件名不符合正则表达式: {}", file_name, re_str).into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let consumer_task = tokio::spawn(async move { consumer.consume().await.unwrap() });

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
