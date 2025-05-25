use crate::db::{Database, SqliteDatabase};
use crate::grpc::DataSinkService;
use crate::proto::data_sink_server::DataSinkServer;
use crate::proto::{
    data_sink_client::DataSinkClient, query_response, value, ColumnDefinition, CreateTableRequest,
    DataType, DeleteRequest, InsertRequest, QueryRequest, UpdateRequest, Value,
};
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
    info!("Connecting to database: {}", database_url);
    let db = SqliteDatabase::connect(&database_url).await?;

    info!("Starting DataSink gRPC server on {}", bind_address);
    let addr = bind_address.parse()?;

    let service = DataSinkService::new(Box::new(db));

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

pub async fn create_table(
    server_address: String,
    table_name: String,
    columns_json: String,
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
    // For SQLite, we just need to ensure the directory exists
    if let Some(parent) = Path::new(&name).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Connect to create the database file
    let db_url = if name.starts_with("sqlite://") {
        name.clone()
    } else {
        format!("sqlite://{}", name)
    };

    let _db = SqliteDatabase::connect(&db_url).await?;
    println!("Database created: {}", name);

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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = QueryRequest {
        sql,
        parameters: HashMap::new(),
    };

    let mut stream = client.query(request).await?.into_inner();
    let mut columns = Vec::new();
    let mut rows = Vec::new();

    while let Some(response) = stream.next().await {
        match response? {
            crate::proto::QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                if !result_set.columns.is_empty() {
                    columns = result_set.columns;
                }
                for row in result_set.rows {
                    rows.push(row.values);
                }
            }
            crate::proto::QueryResponse {
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let data: serde_json::Value = serde_json::from_str(&data_json)?;
    let values = json_to_proto_values(data)?;

    let request = InsertRequest { table_name, values };

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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let data: serde_json::Value = serde_json::from_str(&data_json)?;
    let values = json_to_proto_values(data)?;

    let request = UpdateRequest {
        table_name,
        values,
        where_clause,
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataSinkClient::connect(server_address).await?;

    let request = DeleteRequest {
        table_name,
        where_clause,
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
