mod cli;
pub mod db;
mod grpc;
pub mod schema;
mod proto {
    pub mod common {
        tonic::include_proto!("datasink.common");
    }
    pub mod admin {
        tonic::include_proto!("datasink.admin");
    }
    pub mod crud {
        tonic::include_proto!("datasink.crud");
    }
    tonic::include_proto!("datasink");
}

use clap::Parser;
use tracing::Level;
use tracing_subscriber;

use crate::cli::{commands, Cli, Commands, ServerCommands, SchemaCommands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    // Get database URL from CLI or environment with consistency checking
    let database_url = match cli.resolve_database_url() {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Server { command } => match command {
            ServerCommands::Start { bind_address } => {
                commands::start_server(database_url, bind_address).await?;
            }
            ServerCommands::Stop => {
                commands::stop_server(cli.server_address).await?;
            }
            ServerCommands::CreateTable { name, columns } => {
                commands::create_table(cli.server_address, name, columns).await?;
            }
            ServerCommands::CreateDatabase { name } => {
                commands::create_database(name).await?;
            }
            ServerCommands::CreateFromSchema { schema_file, database_name } => {
                commands::create_from_schema(schema_file, database_name).await?;
            }
        },
        Commands::Query { sql, format, database: _ } => {
            // TODO: Pass database parameter when multi-database support is implemented
            commands::query(cli.server_address, sql, format).await?;
        }
        Commands::Insert { table, data, database: _ } => {
            // TODO: Pass database parameter when multi-database support is implemented
            commands::insert(cli.server_address, table, data).await?;
        }
        Commands::Update {
            table,
            data,
            where_clause,
            database: _,
        } => {
            // TODO: Pass database parameter when multi-database support is implemented
            commands::update(cli.server_address, table, data, where_clause).await?;
        }
        Commands::Delete {
            table,
            where_clause,
            database: _,
        } => {
            // TODO: Pass database parameter when multi-database support is implemented
            commands::delete(cli.server_address, table, where_clause).await?;
        }
        Commands::Schema { command } => match command {
            SchemaCommands::ListTables { database: _ } => {
                commands::list_tables(cli.server_address).await?;
            }
            SchemaCommands::Describe { table, database: _ } => {
                commands::describe_table(cli.server_address, table).await?;
            }
            SchemaCommands::Stats { database: _, detailed } => {
                commands::show_stats(cli.server_address, detailed).await?;
            }
            SchemaCommands::Show { database: _, format } => {
                commands::show_schema(cli.server_address, format).await?;
            }
        }
    }

    Ok(())
}
