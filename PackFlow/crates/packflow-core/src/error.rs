use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DomainError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}
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
        match e {
            rusqlite::Error::SqliteFailure(ref code, _)
                if matches!(
                    code.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                ) =>
            {
                Self::new("BUSY", "Database is busy. Retry with the same request.")
            }
            rusqlite::Error::SqliteFailure(ref code, _)
                if code.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Self::new(
                    "INVARIANT",
                    "This change conflicts with an existing record or data rule.",
                )
            }
            rusqlite::Error::QueryReturnedNoRows => {
                Self::new("NOT_FOUND", "The record no longer exists.")
            }
            _ => {
                eprintln!("SQLite operation failed: {e}");
                Self::storage("Database operation failed. Existing records were preserved.")
            }
        }
    }
}
impl From<std::io::Error> for DomainError {
    fn from(_: std::io::Error) -> Self {
        Self::new(
            "FILE_ACCESS",
            "Unable to read or write the selected data location.",
        )
    }
}
impl From<serde_json::Error> for DomainError {
    fn from(_: serde_json::Error) -> Self {
        Self::new(
            "VALIDATION",
            "Request does not match the expected contract.",
        )
    }
}
pub type Result<T> = std::result::Result<T, DomainError>;
