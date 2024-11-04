/// 自定义错误类型
#[derive(thiserror::Error, Debug)]
pub enum CustomError {
    /// 文件读取失败
    #[error("读取文件失败: {0}")]
    FileReadError(#[from] std::io::Error),
    /// 无法解析或非法的文件
    #[error("非法的文件: {0}")]
    IllegalFileError(String),
    /// 不符合预期的文件,如csv文件名称不符合预期
    #[error("不符合预期的文件: {0}")]
    UnexpectedFileError(String),
    /// csv处理错误
    #[error("解析csv失败: {0}")]
    CsvParseError(#[from] csv::Error),
    /// csv文件中的数据不符合预期
    #[error("csv数据错误: {0}")]
    CsvDataError(String),
    /// regex相关错误
    #[error("failed to parse or compile a regular expression: {0}")]
    RegexError(#[from] regex::Error),
    /// 未知错误
    #[error("未知错误: {0}")]
    UnknownError(String),
}
