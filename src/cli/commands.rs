use crate::db::{Database, SqliteDatabase, DatabaseManager};
use crate::grpc::DataSinkService;
use crate::proto::data_sink_server::DataSinkServer;
use crate::proto::data_sink_client::DataSinkClient;
use crate::proto::admin::{CreateTableRequest, ServerStatusRequest, AddDatabaseRequest};
use crate::proto::crud::{
    DeleteRequest, InsertRequest, QueryRequest, UpdateRequest, 
    query_response, QueryResponse,
};
use crate::proto::common::{ColumnDefinition, DataType, Value, value};
use crate::schema::parser;
use std::collections::HashMap;
use std::path::Path;
use tokio_stream::StreamExt;
use tonic::transport::Server;
use tracing::info;

pub async fn start_server(
    database_url: String,
    bind_address: String,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::cli::validation::validate_database_url;
    
    // Validate and normalize the database URL
    let validated_url = validate_database_url(&database_url)?;
    
    // Add create mode if not already present in the URL
    let db_url = if validated_url.contains("?") {
        validated_url
    } else {
        format!("{}?mode=rwc", validated_url)
    };
    
    info!("Connecting to database: {}", db_url);
    
    // Create database manager and add the primary database
    let db_manager = std::sync::Arc::new(DatabaseManager::new());
    db_manager.add_database("default".to_string(), db_url.clone()).await?;
    
    info!("Starting DataSink gRPC server on {}", bind_address);
    let addr = bind_address.parse()?;

    let service = DataSinkService::new_with_manager(db_manager);

    Server::builder()
        .add_service(DataSinkServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

pub async fn stop_server(_server_address: String) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement graceful shutdown
    // This would require the server to expose a shutdown endpoint
    // or use a signal handler
    eprintln!("Server stop not yet implemented. Use Ctrl+C to stop the server.");
    Ok(())
}

pub async fn server_status(server_address: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;
    
    let request = ServerStatusRequest {};
    let response = client.get_server_status(request).await?;
    let status = response.into_inner();
    
    println!("🚀 DataSink Server Status");
    println!("========================");
    println!("Status: {}", if status.server_running { "🟢 Running" } else { "🔴 Stopped" });
    println!("Uptime: {} seconds", status.uptime_seconds);
    println!();
    
    if status.databases.is_empty() {
        println!("📋 No databases connected");
    } else {
        println!("📋 Connected Databases ({}):", status.databases.len());
        println!();
        for db in status.databases {
            let connection_time = if db.connection_time > 0 {
                let dt = chrono::DateTime::from_timestamp(db.connection_time, 0)
                    .unwrap_or_else(|| chrono::Utc::now());
                dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            } else {
                "Unknown".to_string()
            };
            
            println!("  📊 Database: {}", db.name);
            println!("     URL: {}", db.url);
            println!("     Status: {}", if db.connected { "🟢 Connected" } else { "🔴 Disconnected" });
            println!("     Connected: {}", connection_time);
            println!("     Active Connections: {}", db.active_connections);
            println!();
        }
    }
    
    Ok(())
}

pub async fn add_database(
    server_address: String,
    name: String,
    url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;
    
    let request = AddDatabaseRequest { name: name.clone(), url };
    let response = client.add_database(request).await?;
    let result = response.into_inner();
    
    if result.success {
        println!("✅ {}", result.message);
    } else {
        eprintln!("❌ {}", result.message);
        std::process::exit(1);
    }
    
    Ok(())
}

pub async fn create_table(
    server_address: String,
    table_name: String,
    columns_json: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    // Parse column definitions from JSON
    let column_defs: Vec<serde_json::Value> = serde_json::from_str(&columns_json)?;

    let mut columns = Vec::new();
    for col in column_defs {
        let name = col["name"].as_str().ok_or("Missing column name")?;
        let type_str = col["type"].as_str().ok_or("Missing column type")?;

        let data_type = match type_str.to_uppercase().as_str() {
            "INTEGER" => DataType::Integer,
            "REAL" => DataType::Real,
            "TEXT" => DataType::Text,
            "BLOB" => DataType::Blob,
            "BOOLEAN" => DataType::Boolean,
            "TIMESTAMP" => DataType::Timestamp,
            _ => return Err(format!("Unknown data type: {}", type_str).into()),
        };

        columns.push(ColumnDefinition {
            name: name.to_string(),
            r#type: data_type as i32,
            nullable: col["nullable"].as_bool().unwrap_or(true),
            primary_key: col["primary_key"].as_bool().unwrap_or(false),
            unique: col["unique"].as_bool().unwrap_or(false),
            default_value: col["default_value"].as_str().unwrap_or("").to_string(),
        });
    }

    let request = CreateTableRequest {
        table_name,
        columns,
        database: database.unwrap_or_default(),
    };

    let response = client.create_table(request).await?;
    let inner = response.into_inner();

    if inner.success {
        println!("{}", inner.message);
    } else {
        eprintln!("Failed to create table: {}", inner.message);
    }

    Ok(())
}

pub async fn create_database(name: String) -> Result<(), Box<dyn std::error::Error>> {
    // Check if a database with the same name (case-insensitive) already exists
    let db_file = if name.ends_with(".db") {
        name.clone()
    } else {
        format!("{}.db", name)
    };
    
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                if file_name.to_lowercase() == db_file.to_lowercase() {
                    return Err(format!(
                        "Database already exists: {} (case-insensitive match)",
                        file_name
                    ).into());
                }
            }
        }
    }
    
    // For SQLite, we just need to ensure the directory exists
    if let Some(parent) = Path::new(&db_file).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Connect to create the database file with create mode
    let db_url = if name.starts_with("sqlite://") {
        format!("{}?mode=rwc", name)
    } else {
        format!("sqlite://{}?mode=rwc", db_file)
    };

    let _db = SqliteDatabase::connect(&db_url).await?;
    println!("Database created: {}", db_file);

    Ok(())
}

pub async fn create_from_schema(
    schema_file: String,
    database_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the schema file
    let schema_path = Path::new(&schema_file);
    if !schema_path.exists() {
        return Err(format!("Schema file not found: {}", schema_file).into());
    }
    
    println!("Loading schema from: {}", schema_file);
    let schema = parser::load_schema(schema_path).await?;
    
    // Determine database name
    let db_name = database_name.unwrap_or_else(|| schema.database.name.clone());
    let db_file = format!("{}.db", db_name);
    let db_url = format!("sqlite://{}?mode=rwc", db_file);
    
    println!("Creating database: {}", db_file);
    
    // Create database directly without server
    let db = SqliteDatabase::connect(&db_url).await?;
    
    // Create tables
    for table in &schema.tables {
        println!("Creating table: {}", table.name);
        
        let mut db_columns = Vec::new();
        for col in &table.columns {
            let db_col = parser::column_def_to_db(col)?;
            db_columns.push(db_col);
        }
        
        if let Err(e) = db.create_table(&table.name, db_columns).await {
            eprintln!("Warning: Failed to create table {}: {}", table.name, e);
        }
    }
    
    // Insert initial data
    for (table_name, rows) in &schema.data {
        if rows.is_empty() {
            continue;
        }
        
        println!("Inserting data into table: {}", table_name);
        
        // Find table definition
        let table_def = schema.tables.iter()
            .find(|t| t.name == *table_name)
            .ok_or_else(|| format!("Table {} not found in schema", table_name))?;
        
        // Insert rows using batch insert
        let mut db_rows = Vec::new();
        for row_data in rows {
            let values = parser::prepare_insert_data_db(table_def, row_data)?;
            db_rows.push(values);
        }
        
        match db.batch_insert(table_name, db_rows).await {
            Ok(count) => println!("  Inserted {} rows", count),
            Err(e) => eprintln!("  Warning: Failed to insert data: {}", e),
        }
    }
    
    // Create indexes (if supported in future)
    if !schema.indexes.is_empty() {
        println!("Note: Index creation from schema files will be supported in a future version");
    }
    
    println!("\nDatabase '{}' created successfully from schema!", db_name);
    println!("Database file: {}", db_file);
    println!("Schema version: {}", schema.database.version);
    
    Ok(())
}

pub async fn query(
    server_address: String,
    sql: String,
    format: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = QueryRequest {
        sql,
        parameters: HashMap::new(),
        database: database.unwrap_or_default(),
    };

    let mut stream = client.query(request).await?.into_inner();
    let mut columns = Vec::new();
    let mut rows = Vec::new();

    while let Some(response) = stream.next().await {
        match response? {
            QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                if !result_set.columns.is_empty() {
                    columns = result_set.columns;
                }
                for row in result_set.rows {
                    rows.push(row.values);
                }
            }
            QueryResponse {
                response: Some(query_response::Response::Error(error)),
            } => {
                eprintln!("Query error: {} - {}", error.code, error.message);
                return Ok(());
            }
            _ => {}
        }
    }

    // Format output
    match format.as_str() {
        "json" => {
            let mut json_rows = Vec::new();
            for row in rows {
                let mut json_row = serde_json::Map::new();
                for (i, value) in row.into_iter().enumerate() {
                    if let Some(col) = columns.get(i) {
                        json_row.insert(col.name.clone(), proto_value_to_json(value));
                    }
                }
                json_rows.push(serde_json::Value::Object(json_row));
            }
            println!("{}", serde_json::to_string_pretty(&json_rows)?);
        }
        "csv" => {
            // Print header
            println!(
                "{}",
                columns
                    .iter()
                    .map(|c| &c.name)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",")
            );
            // Print rows
            for row in rows {
                let values: Vec<String> =
                    row.into_iter().map(|v| proto_value_to_string(v)).collect();
                println!("{}", values.join(","));
            }
        }
        _ => {
            // Table format (default)
            if !columns.is_empty() {
                // Print header
                println!(
                    "{}",
                    columns
                        .iter()
                        .map(|c| &c.name)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                println!("{}", "-".repeat(80));

                // Print rows
                for row in rows {
                    let values: Vec<String> =
                        row.into_iter().map(|v| proto_value_to_string(v)).collect();
                    println!("{}", values.join(" | "));
                }
            }
        }
    }

    Ok(())
}

pub async fn insert(
    server_address: String,
    table_name: String,
    data_json: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let data: serde_json::Value = serde_json::from_str(&data_json)?;
    let values = json_to_proto_values(data)?;

    let request = InsertRequest { 
        table_name, 
        values,
        database: database.unwrap_or_default(),
    };

    let response = client.insert(request).await?;
    let inner = response.into_inner();

    if inner.success {
        println!("Insert successful. ID: {}", inner.inserted_id);
    } else {
        eprintln!("Insert failed: {}", inner.message);
    }

    Ok(())
}

pub async fn update(
    server_address: String,
    table_name: String,
    data_json: String,
    where_clause: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let data: serde_json::Value = serde_json::from_str(&data_json)?;
    let values = json_to_proto_values(data)?;

    let request = UpdateRequest {
        table_name,
        values,
        where_clause,
        database: database.unwrap_or_default(),
    };

    let response = client.update(request).await?;
    let inner = response.into_inner();

    if inner.success {
        println!("{}", inner.message);
    } else {
        eprintln!("Update failed: {}", inner.message);
    }

    Ok(())
}

pub async fn delete(
    server_address: String,
    table_name: String,
    where_clause: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = DeleteRequest {
        table_name,
        where_clause,
        database: database.unwrap_or_default(),
    };

    let response = client.delete(request).await?;
    let inner = response.into_inner();

    if inner.success {
        println!("{}", inner.message);
    } else {
        eprintln!("Delete failed: {}", inner.message);
    }

    Ok(())
}

// Helper functions
fn json_to_proto_values(
    json: serde_json::Value,
) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    let obj = json.as_object().ok_or("Expected JSON object")?;
    let mut values = HashMap::new();

    for (key, val) in obj {
        let proto_value = match val {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value {
                        value: Some(value::Value::IntValue(i)),
                    }
                } else if let Some(f) = n.as_f64() {
                    Value {
                        value: Some(value::Value::RealValue(f)),
                    }
                } else {
                    return Err("Invalid number".into());
                }
            }
            serde_json::Value::String(s) => Value {
                value: Some(value::Value::TextValue(s.clone())),
            },
            serde_json::Value::Bool(b) => Value {
                value: Some(value::Value::BoolValue(*b)),
            },
            serde_json::Value::Null => Value {
                value: Some(value::Value::NullValue(true)),
            },
            _ => return Err("Unsupported JSON value type".into()),
        };

        values.insert(key.clone(), proto_value);
    }

    Ok(values)
}

fn proto_value_to_string(value: Value) -> String {
    match value.value {
        Some(value::Value::IntValue(i)) => i.to_string(),
        Some(value::Value::RealValue(f)) => f.to_string(),
        Some(value::Value::TextValue(s)) => s,
        Some(value::Value::BoolValue(b)) => b.to_string(),
        Some(value::Value::TimestampValue(t)) => t.to_string(),
        Some(value::Value::BlobValue(b)) => format!("<blob:{} bytes>", b.len()),
        _ => "NULL".to_string(),
    }
}

fn proto_value_to_json(value: Value) -> serde_json::Value {
    match value.value {
        Some(value::Value::IntValue(i)) => serde_json::Value::Number(i.into()),
        Some(value::Value::RealValue(f)) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(value::Value::TextValue(s)) => serde_json::Value::String(s),
        Some(value::Value::BoolValue(b)) => serde_json::Value::Bool(b),
        Some(value::Value::TimestampValue(t)) => serde_json::Value::Number(t.into()),
        Some(value::Value::BlobValue(b)) => {
            use base64::Engine;
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
        _ => serde_json::Value::Null,
    }
}

pub async fn list_tables(
    server_address: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = QueryRequest {
        sql: "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name".to_string(),
        parameters: HashMap::new(),
        database: database.clone().unwrap_or_default(),
    };

    let mut stream = client.query(request).await?.into_inner();
    let mut tables = Vec::new();

    while let Some(response) = stream.next().await {
        match response? {
            QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                for row in result_set.rows {
                    if let Some(value) = row.values.first() {
                        if let Some(value::Value::TextValue(table_name)) = &value.value {
                            tables.push(table_name.clone());
                        }
                    }
                }
            }
            QueryResponse {
                response: Some(query_response::Response::Error(error)),
            } => {
                eprintln!("Query error: {} - {}", error.code, error.message);
                return Ok(());
            }
            _ => {}
        }
    }

    if tables.is_empty() {
        let db_info = if database.is_some() {
            format!(" '{}'", database.as_ref().unwrap())
        } else {
            " (default)".to_string()
        };
        println!("No tables found in database{}", db_info);
        println!("Tip: Use 'datasink server status' to see available databases");
    } else {
        let db_info = if database.is_some() {
            format!(" '{}'", database.as_ref().unwrap())
        } else {
            " (default)".to_string()
        };
        println!("Tables in database{}:", db_info);
        for table in tables {
            println!("  {}", table);
        }
    }

    Ok(())
}

pub async fn describe_table(
    server_address: String,
    table_name: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = QueryRequest {
        sql: format!("PRAGMA table_info({})", table_name),
        parameters: HashMap::new(),
        database: database.unwrap_or_default(),
    };

    let mut stream = client.query(request).await?.into_inner();
    let mut rows = Vec::new();

    while let Some(response) = stream.next().await {
        match response? {
            QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                for row in result_set.rows {
                    rows.push(row.values);
                }
            }
            QueryResponse {
                response: Some(query_response::Response::Error(error)),
            } => {
                eprintln!("Query error: {} - {}", error.code, error.message);
                return Ok(());
            }
            _ => {}
        }
    }

    if rows.is_empty() {
        println!("Table '{}' not found", table_name);
        return Ok(());
    }

    println!("Table: {}", table_name);
    println!("Columns:");
    println!("  Name          Type      Nullable  Primary Key  Default");
    println!("  {}","-".repeat(60));

    for row in rows {
        if row.len() >= 6 {
            let name = proto_value_to_string(row[1].clone());
            let type_name = proto_value_to_string(row[2].clone());
            let nullable = if proto_value_to_string(row[3].clone()) == "0" { "YES" } else { "NO" };
            let pk = if proto_value_to_string(row[5].clone()) == "0" { "NO" } else { "YES" };
            let default = proto_value_to_string(row[4].clone());
            let default_display = if default == "NULL" { "-" } else { &default };

            println!("  {:<12}  {:<8}  {:<8}  {:<11}  {}", 
                name, type_name, nullable, pk, default_display);
        }
    }

    Ok(())
}

pub async fn show_stats(
    server_address: String,
    detailed: bool,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address.clone()).await?;

    // First get all tables
    let tables_request = QueryRequest {
        sql: "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name".to_string(),
        parameters: HashMap::new(),
        database: database.clone().unwrap_or_default(),
    };

    let mut stream = client.query(tables_request).await?.into_inner();
    let mut tables = Vec::new();

    while let Some(response) = stream.next().await {
        match response? {
            QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                for row in result_set.rows {
                    if let Some(value) = row.values.first() {
                        if let Some(value::Value::TextValue(table_name)) = &value.value {
                            tables.push(table_name.clone());
                        }
                    }
                }
            }
            QueryResponse {
                response: Some(query_response::Response::Error(error)),
            } => {
                eprintln!("Query error: {} - {}", error.code, error.message);
                return Ok(());
            }
            _ => {}
        }
    }

    if tables.is_empty() {
        println!("No tables found in database");
        return Ok(());
    }

    println!("Database Statistics:");
    println!("  Total tables: {}", tables.len());
    println!();
    println!("Row counts by table:");
    println!("  {:<20}  {}", "Table", "Rows");
    println!("  {}", "-".repeat(30));

    let mut total_rows = 0;
    for table in &tables {
        // Reconnect for each query to avoid stream issues
        let mut client = DataSinkClient::connect(server_address.clone()).await?;
        
        let count_request = QueryRequest {
            sql: format!("SELECT COUNT(*) FROM {}", table),
            parameters: HashMap::new(),
            database: database.clone().unwrap_or_default(),
        };

        let mut stream = client.query(count_request).await?.into_inner();
        let mut count = 0;

        while let Some(response) = stream.next().await {
            match response? {
                QueryResponse {
                    response: Some(query_response::Response::ResultSet(result_set)),
                } => {
                    for row in result_set.rows {
                        if let Some(value) = row.values.first() {
                            if let Some(value::Value::IntValue(row_count)) = &value.value {
                                count = *row_count;
                            }
                        }
                    }
                }
                QueryResponse {
                    response: Some(query_response::Response::Error(error)),
                } => {
                    eprintln!("Error counting {}: {} - {}", table, error.code, error.message);
                    continue;
                }
                _ => {}
            }
        }

        println!("  {:<20}  {}", table, count);
        total_rows += count;
    }

    println!("  {}", "-".repeat(30));
    println!("  {:<20}  {}", "Total", total_rows);

    if detailed {
        println!();
        println!("Detailed table information:");
        for table in &tables {
            describe_table(server_address.clone(), table.clone(), database.clone()).await?;
            println!();
        }
    }

    Ok(())
}

pub async fn show_schema(
    server_address: String,
    format: String,
    database: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = QueryRequest {
        sql: "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name".to_string(),
        parameters: HashMap::new(),
        database: database.unwrap_or_default(),
    };

    let mut stream = client.query(request).await?.into_inner();
    let mut schemas = Vec::new();

    while let Some(response) = stream.next().await {
        match response? {
            QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                for row in result_set.rows {
                    if let Some(value) = row.values.first() {
                        if let Some(value::Value::TextValue(sql)) = &value.value {
                            schemas.push(sql.clone());
                        }
                    }
                }
            }
            QueryResponse {
                response: Some(query_response::Response::Error(error)),
            } => {
                eprintln!("Query error: {} - {}", error.code, error.message);
                return Ok(());
            }
            _ => {}
        }
    }

    if schemas.is_empty() {
        println!("No tables found in database");
        return Ok(());
    }

    match format.as_str() {
        "sql" => {
            println!("-- Database Schema");
            for schema in schemas {
                println!("{};", schema);
                println!();
            }
        }
        "json" => {
            let json_schemas: Vec<serde_json::Value> = schemas
                .into_iter()
                .map(|s| serde_json::Value::String(s))
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_schemas)?);
        }
        _ => {
            println!("Database Schema:");
            println!("{}", "=".repeat(50));
            for schema in schemas {
                println!("{}", schema);
                println!("{}", "-".repeat(50));
            }
        }
    }

    Ok(())
}
