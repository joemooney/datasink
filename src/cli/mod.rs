pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "datasink")]
#[command(about = "A gRPC-based database service", long_about = None)]
#[command(version)]
#[command(after_help = "Use 'datasink <COMMAND> --help' for more information about a specific command.")]
pub struct Cli {
    /// Database URL (can also be set via DATABASE_URL env var)
    #[arg(short, long, global = true)]
    pub database_url: Option<String>,

    /// Database name (can also be set via DATABASE_NAME env var)
    /// This will be used to infer the database URL as sqlite://<name>.db
    #[arg(short = 'n', long, global = true)]
    pub database_name: Option<String>,

    /// Server address for client commands
    #[arg(short, long, global = true, default_value = "http://127.0.0.1:50051")]
    pub server_address: String,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Resolve the database URL from either database_url or database_name
    /// with consistency checking and environment variable support
    /// Database names are case-insensitive and will match existing files
    pub fn resolve_database_url(&self) -> Result<String, String> {
        // Get values from CLI args or environment variables
        let db_url = self.database_url.clone()
            .or_else(|| std::env::var("DATABASE_URL").ok());
        
        let db_name = self.database_name.clone()
            .or_else(|| std::env::var("DATABASE_NAME").ok());
        
        match (db_url, db_name) {
            // Both provided - check consistency (case-insensitive)
            (Some(url), Some(name)) => {
                let inferred_url = self.find_or_create_db_url(&name)?;
                let url_lower = url.to_lowercase();
                let inferred_lower = inferred_url.to_lowercase();
                
                if url_lower != inferred_lower {
                    // Also check if URL ends with the database file name
                    let db_file_lower = format!("/{}.db", name.to_lowercase());
                    if !url_lower.ends_with(&db_file_lower) {
                        return Err(format!(
                            "Inconsistent database configuration: URL '{}' doesn't match name '{}'",
                            url, name
                        ));
                    }
                }
                Ok(url)
            }
            // Only URL provided
            (Some(url), None) => Ok(url),
            // Only name provided - find existing or create new
            (None, Some(name)) => self.find_or_create_db_url(&name),
            // Neither provided - use default
            (None, None) => Ok("sqlite://datasink.db".to_string()),
        }
    }
    
    /// Find existing database file (case-insensitive) or return path for new one
    fn find_or_create_db_url(&self, name: &str) -> Result<String, String> {
        self.find_or_create_db_url_in_dir(name, ".")
    }
    
    /// Find existing database file (case-insensitive) in a specific directory or return path for new one
    fn find_or_create_db_url_in_dir(&self, name: &str, dir: &str) -> Result<String, String> {
        let name_lower = name.to_lowercase();
        
        // Try to find existing database file with case-insensitive match
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    let file_lower = file_name.to_lowercase();
                    
                    // Check if this is a database file matching our name
                    if file_lower == format!("{}.db", name_lower) {
                        return Ok(format!("sqlite://{}", file_name));
                    }
                }
            }
        }
        
        // No existing file found, use the provided name as-is
        Ok(format!("sqlite://{}.db", name))
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Server management commands
    #[command(after_help = "Examples:
  datasink server start
  datasink server create-database myapp.db
  datasink server create-from-schema schemas/blog.schema")]
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    /// Query data from tables
    #[command(after_help = "Examples:
  datasink query \"SELECT * FROM users\"
  datasink query \"SELECT * FROM users WHERE age > 18\" -f json
  datasink query \"SELECT name, email FROM users\" -f csv -D mydb")]
    Query {
        /// SQL query to execute
        sql: String,
        /// Output format (json, table, csv)
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
    },
    /// Insert data into a table
    #[command(after_help = "Examples:
  datasink insert users '{\"name\": \"Alice\", \"email\": \"alice@example.com\"}'
  datasink insert products '{\"name\": \"Laptop\", \"price\": 999.99, \"stock\": 10}'
  datasink insert notes '{\"title\": \"Meeting\", \"priority\": \"high\"}' -D postit")]
    Insert {
        /// Table name
        table: String,
        /// JSON data to insert (e.g., '{"id": 1, "name": "Alice"}')
        data: String,
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
    },
    /// Update data in a table
    #[command(after_help = "Examples:
  datasink update users '{\"email\": \"newemail@example.com\"}' -w \"id = 1\"
  datasink update products '{\"price\": 899.99}' -w \"name = 'Laptop'\"
  datasink update notes '{\"status\": \"closed\"}' -w \"id = 5\" -D postit")]
    Update {
        /// Table name
        table: String,
        /// JSON data to update (e.g., '{"name": "Alice Smith"}')
        data: String,
        /// WHERE clause (e.g., "id = 1")
        #[arg(short, long)]
        where_clause: String,
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
    },
    /// Delete data from a table
    #[command(after_help = "Examples:
  datasink delete users -w \"id = 1\"
  datasink delete products -w \"stock = 0\"
  datasink delete notes -w \"status = 'archived' AND created_at < '2023-01-01'\" -D postit")]
    Delete {
        /// Table name
        table: String,
        /// WHERE clause (e.g., "id = 1")
        #[arg(short, long)]
        where_clause: String,
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
    },
    /// Schema information and statistics
    #[command(after_help = "Examples:
  datasink schema list-tables
  datasink schema describe users
  datasink schema stats")]
    Schema {
        #[command(subcommand)]
        command: SchemaCommands,
    },
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Start the gRPC server
    #[command(after_help = "Examples:
  datasink server start
  datasink server start -b 0.0.0.0:8080
  datasink server start -b 127.0.0.1:9000 -d sqlite://myapp.db")]
    Start {
        /// Server bind address
        #[arg(short, long, default_value = "127.0.0.1:50051")]
        bind_address: String,
    },
    /// Stop the gRPC server (requires server to implement shutdown endpoint)
    #[command(after_help = "Examples:
  datasink server stop")]
    Stop,
    /// Create a new table
    #[command(after_help = "Examples:
  datasink server create-table users '[{\"name\":\"id\",\"type\":\"INTEGER\",\"primary_key\":true}]'
  datasink server create-table products '[{\"name\":\"id\",\"type\":\"INTEGER\",\"primary_key\":true},{\"name\":\"name\",\"type\":\"TEXT\",\"nullable\":false},{\"name\":\"price\",\"type\":\"REAL\"}]'")]
    CreateTable {
        /// Table name
        name: String,
        /// Column definitions as JSON array
        /// Example: '[{"name":"id","type":"INTEGER","primary_key":true},{"name":"name","type":"TEXT"}]'
        columns: String,
    },
    /// Create a new database (SQLite: creates new file)
    #[command(after_help = "Examples:
  datasink server create-database myapp.db
  datasink server create-database /path/to/database.db
  datasink server create-database sqlite://myapp.db")]
    CreateDatabase {
        /// Database name/path
        name: String,
    },
    /// Create a database from a schema file
    #[command(after_help = "Examples:
  datasink server create-from-schema schemas/example.schema
  datasink server create-from-schema schemas/blog.schema -n myblog
  datasink server create-from-schema /path/to/custom.schema --database-name myapp")]
    CreateFromSchema {
        /// Path to the .schema file
        schema_file: String,
        /// Optional database name (overrides schema file)
        #[arg(short = 'n', long)]
        database_name: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SchemaCommands {
    /// List all tables in the database
    #[command(name = "list-tables", after_help = "Examples:
  datasink schema list-tables
  datasink schema list-tables -D mydb")]
    ListTables {
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
    },
    /// Describe table structure
    #[command(name = "describe", after_help = "Examples:
  datasink schema describe users
  datasink schema describe products -D mydb")]
    Describe {
        /// Table name to describe
        table: String,
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
    },
    /// Show database statistics
    #[command(name = "stats", after_help = "Examples:
  datasink schema stats
  datasink schema stats -D mydb
  datasink schema stats --detailed")]
    Stats {
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
        /// Show detailed statistics
        #[arg(long)]
        detailed: bool,
    },
    /// Display full database schema
    #[command(name = "show", after_help = "Examples:
  datasink schema show
  datasink schema show -D mydb
  datasink schema show --format sql")]
    Show {
        /// Target database (defaults to "default")
        #[arg(short = 'D', long)]
        database: Option<String>,
        /// Output format (text, sql)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_resolve_database_url_default() {
        // Clear any existing env vars first
        env::remove_var("DATABASE_URL");
        env::remove_var("DATABASE_NAME");
        
        let cli = Cli {
            database_url: None,
            database_name: None,
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.resolve_database_url().unwrap();
        assert_eq!(result, "sqlite://datasink.db");
    }

    #[test]
    fn test_resolve_database_url_with_name() {
        // Clear any existing env vars first
        env::remove_var("DATABASE_URL");
        env::remove_var("DATABASE_NAME");
        
        // Don't change directories - just test the logic
        let cli = Cli {
            database_url: None,
            database_name: Some("testdb".to_string()),
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.resolve_database_url().unwrap();
        assert_eq!(result, "sqlite://testdb.db");
    }

    #[test]
    fn test_resolve_database_url_with_url() {
        let cli = Cli {
            database_url: Some("sqlite://custom.db".to_string()),
            database_name: None,
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.resolve_database_url().unwrap();
        assert_eq!(result, "sqlite://custom.db");
    }

    #[test]
    fn test_resolve_database_url_consistency_check_pass() {
        let cli = Cli {
            database_url: Some("sqlite://myapp.db".to_string()),
            database_name: Some("myapp".to_string()),
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.resolve_database_url().unwrap();
        assert_eq!(result, "sqlite://myapp.db");
    }

    #[test]
    fn test_resolve_database_url_consistency_check_fail() {
        let cli = Cli {
            database_url: Some("sqlite://other.db".to_string()),
            database_name: Some("myapp".to_string()),
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.resolve_database_url();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Inconsistent database configuration"));
    }

    #[test]
    fn test_case_insensitive_database_matching() {
        // Clear any existing env vars first
        env::remove_var("DATABASE_URL");
        env::remove_var("DATABASE_NAME");
        
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a database file with mixed case in temp dir
        let db_path = temp_path.join("TestDB.db");
        fs::write(&db_path, "").unwrap();

        // Test the find_or_create_db_url_in_dir method directly
        let cli = Cli {
            database_url: None,
            database_name: None,
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.find_or_create_db_url_in_dir("testdb", temp_path.to_str().unwrap()).unwrap();
        assert_eq!(result, "sqlite://TestDB.db"); // Should find the actual file with correct case
    }
    
    #[test]
    fn test_database_matching_no_file_exists() {
        // Clear any existing env vars first
        env::remove_var("DATABASE_URL");
        env::remove_var("DATABASE_NAME");
        
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Don't create any files - test the fallback behavior
        let cli = Cli {
            database_url: None,
            database_name: None,
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.find_or_create_db_url_in_dir("testdb", temp_path.to_str().unwrap()).unwrap();
        assert_eq!(result, "sqlite://testdb.db"); // Should return the name as-is when no file exists
    }

    #[test]
    #[serial]
    fn test_environment_variable_database_url() {
        // Clear any existing env vars first
        env::remove_var("DATABASE_URL");
        env::remove_var("DATABASE_NAME");
        
        env::set_var("DATABASE_URL", "sqlite://env.db");
        
        let cli = Cli {
            database_url: None,
            database_name: None,
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        let result = cli.resolve_database_url().unwrap();
        assert_eq!(result, "sqlite://env.db");
        
        env::remove_var("DATABASE_URL");
    }

    #[test]
    #[serial]
    fn test_environment_variable_database_name() {
        // Clear any existing env vars first
        env::remove_var("DATABASE_URL");
        env::remove_var("DATABASE_NAME");
        
        // Work in a temp directory to avoid any interference
        let temp_dir = TempDir::new().unwrap();
        let _original_dir = env::current_dir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();
        
        env::set_var("DATABASE_NAME", "envdb");
        
        // Verify the env var is set
        assert_eq!(env::var("DATABASE_NAME").unwrap(), "envdb");
        
        let cli = Cli {
            database_url: None,
            database_name: None,
            server_address: "http://127.0.0.1:50051".to_string(),
            verbose: false,
            command: Commands::Server {
                command: ServerCommands::Stop,
            },
        };

        // Test resolve_database_url which should pick up the DATABASE_NAME env var
        let result = cli.resolve_database_url();
        assert!(result.is_ok(), "resolve_database_url failed: {:?}", result);
        assert_eq!(result.unwrap(), "sqlite://envdb.db");
        
        env::remove_var("DATABASE_NAME");
        env::set_current_dir(&_original_dir).unwrap();
    }
}
