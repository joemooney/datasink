/// Database URL validation utilities

/// Validate and normalize a database URL
pub fn validate_database_url(url: &str) -> Result<String, String> {
    // Check if it's already a proper URL with scheme
    if url.starts_with("sqlite://") || url.starts_with("postgres://") || url.starts_with("mysql://") {
        // Ensure SQLite URLs have proper format
        if url.starts_with("sqlite://") {
            let path = &url[9..]; // Skip "sqlite://"
            if path.is_empty() {
                return Err("SQLite URL must specify a database file path".to_string());
            }
            // If path doesn't have an extension and doesn't exist, add .db
            if !path.contains('.') && !std::path::Path::new(path).exists() {
                return Ok(format!("sqlite://{}.db", path));
            }
        }
        Ok(url.to_string())
    } else if url.contains("://") {
        // Has a scheme but not a recognized one
        Err(format!("Unsupported database URL scheme: {}", url))
    } else {
        // No scheme - assume it's a SQLite file path
        // If it doesn't have an extension and doesn't exist, add .db
        if !url.contains('.') && !std::path::Path::new(url).exists() {
            Ok(format!("sqlite://{}.db", url))
        } else {
            Ok(format!("sqlite://{}", url))
        }
    }
}

/// Check if a database URL is valid
pub fn is_valid_database_url(url: &str) -> bool {
    validate_database_url(url).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sqlite_urls() {
        assert_eq!(validate_database_url("sqlite://test.db").unwrap(), "sqlite://test.db");
        assert_eq!(validate_database_url("sqlite://data/test.db").unwrap(), "sqlite://data/test.db");
        assert_eq!(validate_database_url("test.db").unwrap(), "sqlite://test.db");
        assert_eq!(validate_database_url("test").unwrap(), "sqlite://test.db");
        assert_eq!(validate_database_url("data/test").unwrap(), "sqlite://data/test.db");
    }

    #[test]
    fn test_validate_other_urls() {
        assert_eq!(validate_database_url("postgres://localhost/mydb").unwrap(), "postgres://localhost/mydb");
        assert_eq!(validate_database_url("mysql://localhost/mydb").unwrap(), "mysql://localhost/mydb");
    }

    #[test]
    fn test_invalid_urls() {
        assert!(validate_database_url("invalid://test").is_err());
        assert!(validate_database_url("sqlite://").is_err());
    }
}