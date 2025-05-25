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

    /// Server address for client commands
    #[arg(short, long, global = true, default_value = "http://127.0.0.1:50051")]
    pub server_address: String,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
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
  datasink server create-from-schema schemas/blog.schema -d myblog
  datasink server create-from-schema /path/to/custom.schema")]
    CreateFromSchema {
        /// Path to the .schema file
        schema_file: String,
        /// Optional database name (overrides schema file)
        #[arg(short, long)]
        database_name: Option<String>,
    },
}
