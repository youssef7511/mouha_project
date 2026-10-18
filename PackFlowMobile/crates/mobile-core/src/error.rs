use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct DomainError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}
pub type Result<T> = std::result::Result<T, DomainError>;
impl DomainError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }
    pub fn validation(field: &str, message: &str) -> Self {
        Self {
            code: "VALIDATION".into(),
            message: message.into(),
            field: Some(field.into()),
        }
    }
    pub fn storage(message: &str) -> Self {
        Self::new("STORAGE", message)
    }
}
impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for DomainError {}
impl From<rusqlite::Error> for DomainError {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("Mobile SQLite: {e}");
        Self::storage("The local operation failed. Existing records were preserved.")
    }
}
impl From<std::io::Error> for DomainError {
    fn from(_: std::io::Error) -> Self {
        Self::new(
            "FILE_ACCESS",
            "Could not read or write the selected local file.",
        )
    }
}
impl From<serde_json::Error> for DomainError {
    fn from(_: serde_json::Error) -> Self {
        Self::validation("request", "The request does not match the expected format.")
    }
}
