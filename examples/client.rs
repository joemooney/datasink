//! Example client demonstrating how to use the DataSink gRPC service

use std::collections::HashMap;
use tokio_stream::StreamExt;

// Include the proto modules
pub mod proto {
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

use proto::data_sink_client::DataSinkClient;
use proto::admin::{CreateTableRequest};
use proto::crud::{
    BatchInsertRequest, DeleteRequest, InsertRequest, InsertRow, 
    QueryRequest, QueryResponse, UpdateRequest, query_response,
};
use proto::common::{ColumnDefinition, DataType, Value, value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let mut client = DataSinkClient::connect("http://127.0.0.1:50051").await?;

    // Create a table
    println!("Creating users table...");
    let create_table_req = CreateTableRequest {
        table_name: "users".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                r#type: DataType::Integer as i32,
                nullable: false,
                primary_key: true,
                unique: true,
                default_value: String::new(),
            },
            ColumnDefinition {
                name: "name".to_string(),
                r#type: DataType::Text as i32,
                nullable: false,
                primary_key: false,
                unique: false,
                default_value: String::new(),
            },
            ColumnDefinition {
                name: "email".to_string(),
                r#type: DataType::Text as i32,
                nullable: false,
                primary_key: false,
                unique: true,
                default_value: String::new(),
            },
            ColumnDefinition {
                name: "created_at".to_string(),
                r#type: DataType::Timestamp as i32,
                nullable: false,
                primary_key: false,
                unique: false,
                default_value: String::new(),
            },
        ],
    };

    let response = client.create_table(create_table_req).await?;
    println!("Create table response: {:?}", response.into_inner());

    // Insert a single row
    println!("\nInserting a user...");
    let mut values = HashMap::new();
    values.insert(
        "id".to_string(),
        Value {
            value: Some(value::Value::IntValue(1)),
        },
    );
    values.insert(
        "name".to_string(),
        Value {
            value: Some(value::Value::TextValue("Alice Smith".to_string())),
        },
    );
    values.insert(
        "email".to_string(),
        Value {
            value: Some(value::Value::TextValue("alice@example.com".to_string())),
        },
    );
    values.insert(
        "created_at".to_string(),
        Value {
            value: Some(value::Value::TimestampValue(chrono::Utc::now().timestamp())),
        },
    );

    let insert_req = InsertRequest {
        table_name: "users".to_string(),
        values,
    };

    let response = client.insert(insert_req).await?;
    println!("Insert response: {:?}", response.into_inner());

    // Batch insert multiple rows
    println!("\nBatch inserting users...");
    let batch_req = BatchInsertRequest {
        table_name: "users".to_string(),
        rows: vec![
            InsertRow {
                values: {
                    let mut values = HashMap::new();
                    values.insert(
                        "id".to_string(),
                        Value {
                            value: Some(value::Value::IntValue(2)),
                        },
                    );
                    values.insert(
                        "name".to_string(),
                        Value {
                            value: Some(value::Value::TextValue("Bob Jones".to_string())),
                        },
                    );
                    values.insert(
                        "email".to_string(),
                        Value {
                            value: Some(value::Value::TextValue("bob@example.com".to_string())),
                        },
                    );
                    values.insert(
                        "created_at".to_string(),
                        Value {
                            value: Some(value::Value::TimestampValue(
                                chrono::Utc::now().timestamp(),
                            )),
                        },
                    );
                    values
                },
            },
            InsertRow {
                values: {
                    let mut values = HashMap::new();
                    values.insert(
                        "id".to_string(),
                        Value {
                            value: Some(value::Value::IntValue(3)),
                        },
                    );
                    values.insert(
                        "name".to_string(),
                        Value {
                            value: Some(value::Value::TextValue("Carol Davis".to_string())),
                        },
                    );
                    values.insert(
                        "email".to_string(),
                        Value {
                            value: Some(value::Value::TextValue("carol@example.com".to_string())),
                        },
                    );
                    values.insert(
                        "created_at".to_string(),
                        Value {
                            value: Some(value::Value::TimestampValue(
                                chrono::Utc::now().timestamp(),
                            )),
                        },
                    );
                    values
                },
            },
        ],
    };

    let response = client.batch_insert(batch_req).await?;
    println!("Batch insert response: {:?}", response.into_inner());

    // Update a row
    println!("\nUpdating user...");
    let mut update_values = HashMap::new();
    update_values.insert(
        "name".to_string(),
        Value {
            value: Some(value::Value::TextValue("Alice Johnson".to_string())),
        },
    );

    let update_req = UpdateRequest {
        table_name: "users".to_string(),
        values: update_values,
        where_clause: "id = 1".to_string(),
    };

    let response = client.update(update_req).await?;
    println!("Update response: {:?}", response.into_inner());

    // Query with streaming response
    println!("\nQuerying all users...");
    let query_req = QueryRequest {
        sql: "SELECT * FROM users ORDER BY id".to_string(),
        parameters: HashMap::new(),
    };

    let mut stream = client.query(query_req).await?.into_inner();

    println!("Query results:");
    while let Some(response) = stream.next().await {
        match response? {
            QueryResponse {
                response: Some(query_response::Response::ResultSet(result_set)),
            } => {
                // First response contains columns
                if !result_set.columns.is_empty() {
                    println!(
                        "Columns: {}",
                        result_set
                            .columns
                            .iter()
                            .map(|c| &c.name)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                // Process rows
                for row in result_set.rows {
                    let values: Vec<String> = row
                        .values
                        .into_iter()
                        .map(|v| match v.value {
                            Some(value::Value::IntValue(i)) => i.to_string(),
                            Some(value::Value::TextValue(s)) => s,
                            Some(value::Value::RealValue(f)) => f.to_string(),
                            Some(value::Value::BoolValue(b)) => b.to_string(),
                            Some(value::Value::TimestampValue(t)) => {
                                chrono::DateTime::from_timestamp(t, 0)
                                    .map(|dt| dt.to_rfc3339())
                                    .unwrap_or_else(|| t.to_string())
                            }
                            _ => "NULL".to_string(),
                        })
                        .collect();
                    println!("Row: {}", values.join(", "));
                }
            }
            QueryResponse {
                response: Some(query_response::Response::Error(error)),
            } => {
                eprintln!("Query error: {} - {}", error.code, error.message);
            }
            _ => {}
        }
    }

    // Delete a row
    println!("\nDeleting user...");
    let delete_req = DeleteRequest {
        table_name: "users".to_string(),
        where_clause: "id = 3".to_string(),
    };

    let response = client.delete(delete_req).await?;
    println!("Delete response: {:?}", response.into_inner());

    Ok(())
}
