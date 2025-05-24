use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Table already exists: {0}")]
    TableAlreadyExists(String),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Invalid column type: {0}")]
    InvalidColumnType(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;
