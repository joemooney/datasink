pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "datasink")]
#[command(about = "A gRPC-based database service", long_about = None)]
#[command(version)]
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
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    /// Query data from tables
    Query {
        /// SQL query to execute
        sql: String,
        /// Output format (json, table, csv)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Insert data into a table
    Insert {
        /// Table name
        table: String,
        /// JSON data to insert (e.g., '{"id": 1, "name": "Alice"}')
        data: String,
    },
    /// Update data in a table
    Update {
        /// Table name
        table: String,
        /// JSON data to update (e.g., '{"name": "Alice Smith"}')
        data: String,
        /// WHERE clause (e.g., "id = 1")
        #[arg(short, long)]
        where_clause: String,
    },
    /// Delete data from a table
    Delete {
        /// Table name
        table: String,
        /// WHERE clause (e.g., "id = 1")
        #[arg(short, long)]
        where_clause: String,
    },
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Start the gRPC server
    Start {
        /// Server bind address
        #[arg(short, long, default_value = "127.0.0.1:50051")]
        bind_address: String,
    },
    /// Stop the gRPC server (requires server to implement shutdown endpoint)
    Stop,
    /// Create a new table
    CreateTable {
        /// Table name
        name: String,
        /// Column definitions as JSON array
        /// Example: '[{"name":"id","type":"INTEGER","primary_key":true},{"name":"name","type":"TEXT"}]'
        columns: String,
    },
    /// Create a new database (SQLite: creates new file)
    CreateDatabase {
        /// Database name/path
        name: String,
    },
}
