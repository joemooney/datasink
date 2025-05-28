use datasink::db::{traits::*, SqliteDatabase, Database};
use std::collections::HashMap;
use tempfile::NamedTempFile;
use tokio;

#[tokio::test]
async fn test_sqlite_database_operations() {
    // Create a temporary database file
    let temp_file = NamedTempFile::new().unwrap();
    let db_url = format!("sqlite://{}?mode=rwc", temp_file.path().display());
    
    let db = SqliteDatabase::connect(&db_url).await.unwrap();
    
    // Test create table
    let columns = vec![
        ColumnDef {
            name: "id".to_string(),
            col_type: ColumnType::Integer,
            nullable: false,
            primary_key: true,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "name".to_string(),
            col_type: ColumnType::Text,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "active".to_string(),
            col_type: ColumnType::Boolean,
            nullable: true,
            primary_key: false,
            unique: false,
            default_value: Some("true".to_string()),
        },
    ];
    
    db.create_table("test_users", columns).await.unwrap();
    
    // Test insert
    let mut values = HashMap::new();
    values.insert("id".to_string(), DbValue::Integer(1));
    values.insert("name".to_string(), DbValue::Text("Alice".to_string()));
    values.insert("active".to_string(), DbValue::Boolean(true));
    
    let insert_id = db.insert("test_users", values).await.unwrap();
    assert_eq!(insert_id, 1);
    
    // Test query
    let results = db.query("SELECT * FROM test_users WHERE id = 1", HashMap::new()).await.unwrap();
    assert_eq!(results.rows.len(), 1);
    assert_eq!(results.rows[0].len(), 3);
    
    // Test update
    let mut update_values = HashMap::new();
    update_values.insert("name".to_string(), DbValue::Text("Alice Smith".to_string()));
    
    let rows_affected = db.update("test_users", update_values, "id = 1").await.unwrap();
    assert_eq!(rows_affected, 1);
    
    // Verify update
    let results = db.query("SELECT name FROM test_users WHERE id = 1", HashMap::new()).await.unwrap();
    assert!(matches!(&results.rows[0][0], DbValue::Text(s) if s == "Alice Smith"));
    
    // Test delete
    let rows_deleted = db.delete("test_users", "id = 1").await.unwrap();
    assert_eq!(rows_deleted, 1);
    
    // Verify delete
    let results = db.query("SELECT COUNT(*) FROM test_users", HashMap::new()).await.unwrap();
    assert!(matches!(results.rows[0][0], DbValue::Integer(0)));
}

#[tokio::test]
async fn test_batch_insert() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_url = format!("sqlite://{}?mode=rwc", temp_file.path().display());
    
    let db = SqliteDatabase::connect(&db_url).await.unwrap();
    
    // Create table
    let columns = vec![
        ColumnDef {
            name: "id".to_string(),
            col_type: ColumnType::Integer,
            nullable: false,
            primary_key: true,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "value".to_string(),
            col_type: ColumnType::Real,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
    ];
    
    db.create_table("numbers", columns).await.unwrap();
    
    // Batch insert
    let rows: Vec<HashMap<String, DbValue>> = (1..=5)
        .map(|i| {
            let mut row = HashMap::new();
            row.insert("id".to_string(), DbValue::Integer(i));
            row.insert("value".to_string(), DbValue::Real(i as f64 * 1.5));
            row
        })
        .collect();
    
    let count = db.batch_insert("numbers", rows).await.unwrap();
    assert_eq!(count, 5);
    
    // Verify
    let results = db.query("SELECT COUNT(*) FROM numbers", HashMap::new()).await.unwrap();
    assert!(matches!(results.rows[0][0], DbValue::Integer(5)));
}

#[tokio::test]
async fn test_null_values() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_url = format!("sqlite://{}?mode=rwc", temp_file.path().display());
    
    let db = SqliteDatabase::connect(&db_url).await.unwrap();
    
    // Create table with nullable column
    let columns = vec![
        ColumnDef {
            name: "id".to_string(),
            col_type: ColumnType::Integer,
            nullable: false,
            primary_key: true,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "optional_text".to_string(),
            col_type: ColumnType::Text,
            nullable: true,
            primary_key: false,
            unique: false,
            default_value: None,
        },
    ];
    
    db.create_table("nullable_test", columns).await.unwrap();
    
    // Insert with null value
    let mut values = HashMap::new();
    values.insert("id".to_string(), DbValue::Integer(1));
    values.insert("optional_text".to_string(), DbValue::Null);
    
    db.insert("nullable_test", values).await.unwrap();
    
    // Query and verify null
    let results = db.query("SELECT optional_text FROM nullable_test WHERE id = 1", HashMap::new()).await.unwrap();
    assert!(matches!(results.rows[0][0], DbValue::Null));
}

#[tokio::test]
async fn test_data_types() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_url = format!("sqlite://{}?mode=rwc", temp_file.path().display());
    
    let db = SqliteDatabase::connect(&db_url).await.unwrap();
    
    // Create table with all supported types
    let columns = vec![
        ColumnDef {
            name: "id".to_string(),
            col_type: ColumnType::Integer,
            nullable: false,
            primary_key: true,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "int_val".to_string(),
            col_type: ColumnType::Integer,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "real_val".to_string(),
            col_type: ColumnType::Real,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "text_val".to_string(),
            col_type: ColumnType::Text,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "blob_val".to_string(),
            col_type: ColumnType::Blob,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "bool_val".to_string(),
            col_type: ColumnType::Boolean,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
        ColumnDef {
            name: "timestamp_val".to_string(),
            col_type: ColumnType::Timestamp,
            nullable: false,
            primary_key: false,
            unique: false,
            default_value: None,
        },
    ];
    
    db.create_table("type_test", columns).await.unwrap();
    
    // Insert test data
    let mut values = HashMap::new();
    values.insert("id".to_string(), DbValue::Integer(1));
    values.insert("int_val".to_string(), DbValue::Integer(42));
    values.insert("real_val".to_string(), DbValue::Real(3.14159));
    values.insert("text_val".to_string(), DbValue::Text("Hello, World!".to_string()));
    values.insert("blob_val".to_string(), DbValue::Blob(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]));
    values.insert("bool_val".to_string(), DbValue::Boolean(true));
    values.insert("timestamp_val".to_string(), DbValue::Timestamp(1640995200)); // 2022-01-01 00:00:00 UTC
    
    db.insert("type_test", values).await.unwrap();
    
    // Query and verify all types
    let results = db.query("SELECT * FROM type_test WHERE id = 1", HashMap::new()).await.unwrap();
    assert_eq!(results.rows.len(), 1);
    let row = &results.rows[0];
    
    assert!(matches!(row[0], DbValue::Integer(1)));
    assert!(matches!(row[1], DbValue::Integer(42)));
    
    if let DbValue::Real(v) = row[2] {
        assert!((v - 3.14159).abs() < f64::EPSILON);
    } else {
        panic!("Expected Real value");
    }
    
    assert!(matches!(&row[3], DbValue::Text(s) if s == "Hello, World!"));
    assert!(matches!(&row[4], DbValue::Blob(b) if b == &vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]));
    // SQLite stores booleans as integers (0/1)
    assert!(matches!(row[5], DbValue::Integer(1)));
    // SQLite stores timestamps as integers
    assert!(matches!(row[6], DbValue::Integer(1640995200)));
}

