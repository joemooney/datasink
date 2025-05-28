use super::{ColumnDef, Schema};
use crate::db::{traits::ColumnDef as DbColumnDef, traits::ColumnType, traits::DbValue};
use crate::proto::common::{value, Value};
use std::collections::HashMap;
use std::path::Path;

pub async fn load_schema(path: &Path) -> Result<Schema, Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string(path).await?;
    let schema: Schema = toml::from_str(&content)?;
    Ok(schema)
}

pub fn column_def_to_db(col: &ColumnDef) -> Result<DbColumnDef, Box<dyn std::error::Error>> {
    let col_type = match col.col_type.to_uppercase().as_str() {
        "INTEGER" => ColumnType::Integer,
        "REAL" => ColumnType::Real,
        "TEXT" => ColumnType::Text,
        "BLOB" => ColumnType::Blob,
        "BOOLEAN" => ColumnType::Boolean,
        "TIMESTAMP" => ColumnType::Timestamp,
        _ => return Err(format!("Unknown column type: {}", col.col_type).into()),
    };

    // Handle nullable logic - if primary_key is true, nullable must be false
    let nullable = if col.primary_key {
        false
    } else {
        col.nullable
    };

    Ok(DbColumnDef {
        name: col.name.clone(),
        col_type,
        nullable,
        primary_key: col.primary_key,
        unique: col.unique,
        default_value: col.default.clone(),
    })
}

pub fn toml_value_to_proto(
    value: &toml::Value,
    expected_type: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let proto_value = match (value, expected_type.to_uppercase().as_str()) {
        (toml::Value::Integer(i), "INTEGER") => Value {
            value: Some(value::Value::IntValue(*i)),
        },
        (toml::Value::Float(f), "REAL") => Value {
            value: Some(value::Value::RealValue(*f)),
        },
        (toml::Value::String(s), "TEXT") => Value {
            value: Some(value::Value::TextValue(s.clone())),
        },
        (toml::Value::Boolean(b), "BOOLEAN") => Value {
            value: Some(value::Value::BoolValue(*b)),
        },
        (toml::Value::Integer(i), "TIMESTAMP") => Value {
            value: Some(value::Value::TimestampValue(*i)),
        },
        (toml::Value::String(s), "TIMESTAMP") if s == "CURRENT_TIMESTAMP" => {
            // Handle special case for current timestamp
            Value {
                value: Some(value::Value::TimestampValue(
                    chrono::Utc::now().timestamp(),
                )),
            }
        }
        _ => {
            return Err(format!(
                "Type mismatch: cannot convert {:?} to {}",
                value, expected_type
            )
            .into())
        }
    };

    Ok(proto_value)
}

pub fn prepare_insert_data(
    table_def: &super::TableDef,
    row_data: &HashMap<String, toml::Value>,
) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    let mut values = HashMap::new();

    for col in &table_def.columns {
        // Skip auto-increment columns
        if col.auto_increment {
            continue;
        }

        if let Some(value) = row_data.get(&col.name) {
            let proto_value = toml_value_to_proto(value, &col.col_type)?;
            values.insert(col.name.clone(), proto_value);
        } else if let Some(default) = &col.default {
            // Handle default values
            match default.as_str() {
                "CURRENT_TIMESTAMP" => {
                    values.insert(
                        col.name.clone(),
                        Value {
                            value: Some(value::Value::TimestampValue(
                                chrono::Utc::now().timestamp(),
                            )),
                        },
                    );
                }
                "true" => {
                    values.insert(
                        col.name.clone(),
                        Value {
                            value: Some(value::Value::BoolValue(true)),
                        },
                    );
                }
                "false" => {
                    values.insert(
                        col.name.clone(),
                        Value {
                            value: Some(value::Value::BoolValue(false)),
                        },
                    );
                }
                _ => {
                    // Try to parse the default value based on column type
                    match col.col_type.as_str() {
                        "INTEGER" => {
                            if let Ok(i) = default.parse::<i64>() {
                                values.insert(
                                    col.name.clone(),
                                    Value {
                                        value: Some(value::Value::IntValue(i)),
                                    },
                                );
                            }
                        }
                        "REAL" => {
                            if let Ok(f) = default.parse::<f64>() {
                                values.insert(
                                    col.name.clone(),
                                    Value {
                                        value: Some(value::Value::RealValue(f)),
                                    },
                                );
                            }
                        }
                        "TEXT" => {
                            // Remove quotes if present
                            let text = default.trim_matches('\'').trim_matches('"');
                            values.insert(
                                col.name.clone(),
                                Value {
                                    value: Some(value::Value::TextValue(text.to_string())),
                                },
                            );
                        }
                        _ => {}
                    }
                }
            }
        } else if !col.nullable {
            return Err(format!(
                "Missing required field '{}' and no default value",
                col.name
            )
            .into());
        }
    }

    Ok(values)
}

pub fn prepare_insert_data_db(
    table_def: &super::TableDef,
    row_data: &HashMap<String, toml::Value>,
) -> Result<HashMap<String, DbValue>, Box<dyn std::error::Error>> {
    let mut values = HashMap::new();

    for col in &table_def.columns {
        // Skip auto-increment columns
        if col.auto_increment {
            continue;
        }

        if let Some(value) = row_data.get(&col.name) {
            let db_value = toml_value_to_db(value, &col.col_type)?;
            values.insert(col.name.clone(), db_value);
        } else if let Some(default) = &col.default {
            // Handle default values
            match default.as_str() {
                "CURRENT_TIMESTAMP" => {
                    values.insert(
                        col.name.clone(),
                        DbValue::Timestamp(chrono::Utc::now().timestamp()),
                    );
                }
                "true" => {
                    values.insert(col.name.clone(), DbValue::Boolean(true));
                }
                "false" => {
                    values.insert(col.name.clone(), DbValue::Boolean(false));
                }
                _ => {
                    // Try to parse the default value based on column type
                    match col.col_type.as_str() {
                        "INTEGER" => {
                            if let Ok(i) = default.parse::<i64>() {
                                values.insert(col.name.clone(), DbValue::Integer(i));
                            }
                        }
                        "REAL" => {
                            if let Ok(f) = default.parse::<f64>() {
                                values.insert(col.name.clone(), DbValue::Real(f));
                            }
                        }
                        "TEXT" => {
                            // Remove quotes if present
                            let text = default.trim_matches('\'').trim_matches('"');
                            values.insert(col.name.clone(), DbValue::Text(text.to_string()));
                        }
                        _ => {}
                    }
                }
            }
        } else if !col.nullable {
            return Err(format!(
                "Missing required field '{}' and no default value",
                col.name
            )
            .into());
        }
    }

    Ok(values)
}

fn toml_value_to_db(
    value: &toml::Value,
    expected_type: &str,
) -> Result<DbValue, Box<dyn std::error::Error>> {
    let db_value = match (value, expected_type.to_uppercase().as_str()) {
        (toml::Value::Integer(i), "INTEGER") => DbValue::Integer(*i),
        (toml::Value::Float(f), "REAL") => DbValue::Real(*f),
        (toml::Value::String(s), "TEXT") => DbValue::Text(s.clone()),
        (toml::Value::Boolean(b), "BOOLEAN") => DbValue::Boolean(*b),
        (toml::Value::Integer(i), "TIMESTAMP") => DbValue::Timestamp(*i),
        (toml::Value::String(s), "TIMESTAMP") if s == "CURRENT_TIMESTAMP" => {
            // Handle special case for current timestamp
            DbValue::Timestamp(chrono::Utc::now().timestamp())
        }
        _ => {
            return Err(format!(
                "Type mismatch: cannot convert {:?} to {}",
                value, expected_type
            )
            .into())
        }
    };

    Ok(db_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::TableDef;
    use tempfile::NamedTempFile;
    use tokio;

    #[tokio::test]
    async fn test_load_schema() {
        let schema_content = r#"
[database]
name = "test_db"
description = "Test database"
version = "1.0.0"

[[tables]]
name = "users"

[[tables.columns]]
name = "id"
type = "INTEGER"
primary_key = true
auto_increment = true

[[tables.columns]]
name = "name"
type = "TEXT"
nullable = false
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        write!(temp_file, "{}", schema_content).unwrap();

        let schema = load_schema(temp_file.path()).await.unwrap();
        assert_eq!(schema.database.name, "test_db");
        assert_eq!(schema.database.version, "1.0.0");
        assert_eq!(schema.tables.len(), 1);
        assert_eq!(schema.tables[0].name, "users");
        assert_eq!(schema.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_column_def_to_db() {
        let col = ColumnDef {
            name: "test_col".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: true,
            primary_key: false,
            unique: false,
            auto_increment: false,
            default: Some("0".to_string()),
            foreign_key: None,
        };

        let db_col = column_def_to_db(&col).unwrap();
        assert_eq!(db_col.name, "test_col");
        assert!(matches!(db_col.col_type, ColumnType::Integer));
        assert!(db_col.nullable);
        assert!(!db_col.primary_key);
        assert!(!db_col.unique);
        assert_eq!(db_col.default_value, Some("0".to_string()));
    }

    #[test]
    fn test_column_def_to_db_primary_key_not_nullable() {
        let col = ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: true, // This should be overridden
            primary_key: true,
            unique: false,
            auto_increment: true,
            default: None,
            foreign_key: None,
        };

        let db_col = column_def_to_db(&col).unwrap();
        assert!(!db_col.nullable); // Should be false because it's a primary key
        assert!(db_col.primary_key);
    }

    #[test]
    fn test_column_def_to_db_unknown_type() {
        let col = ColumnDef {
            name: "test_col".to_string(),
            col_type: "UNKNOWN".to_string(),
            nullable: true,
            primary_key: false,
            unique: false,
            auto_increment: false,
            default: None,
            foreign_key: None,
        };

        let result = column_def_to_db(&col);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown column type"));
    }

    #[test]
    fn test_toml_value_to_proto() {
        // Test integer conversion
        let int_val = toml::Value::Integer(42);
        let proto_int = toml_value_to_proto(&int_val, "INTEGER").unwrap();
        assert!(matches!(proto_int.value, Some(value::Value::IntValue(42))));

        // Test real conversion
        let float_val = toml::Value::Float(3.14);
        let proto_real = toml_value_to_proto(&float_val, "REAL").unwrap();
        if let Some(value::Value::RealValue(v)) = proto_real.value {
            assert!((v - 3.14).abs() < f64::EPSILON);
        }

        // Test text conversion
        let str_val = toml::Value::String("hello".to_string());
        let proto_text = toml_value_to_proto(&str_val, "TEXT").unwrap();
        assert!(matches!(&proto_text.value, Some(value::Value::TextValue(s)) if s == "hello"));

        // Test boolean conversion
        let bool_val = toml::Value::Boolean(true);
        let proto_bool = toml_value_to_proto(&bool_val, "BOOLEAN").unwrap();
        assert!(matches!(proto_bool.value, Some(value::Value::BoolValue(true))));
    }

    #[test]
    fn test_toml_value_to_proto_type_mismatch() {
        let int_val = toml::Value::Integer(42);
        let result = toml_value_to_proto(&int_val, "TEXT");
        assert!(result.is_err());
    }

    #[test]
    fn test_toml_value_to_db() {
        // Test integer conversion
        let int_val = toml::Value::Integer(42);
        let db_int = toml_value_to_db(&int_val, "INTEGER").unwrap();
        assert!(matches!(db_int, DbValue::Integer(42)));

        // Test real conversion
        let float_val = toml::Value::Float(3.14);
        let db_real = toml_value_to_db(&float_val, "REAL").unwrap();
        if let DbValue::Real(v) = db_real {
            assert!((v - 3.14).abs() < f64::EPSILON);
        }

        // Test text conversion
        let str_val = toml::Value::String("hello".to_string());
        let db_text = toml_value_to_db(&str_val, "TEXT").unwrap();
        assert!(matches!(db_text, DbValue::Text(s) if s == "hello"));

        // Test boolean conversion
        let bool_val = toml::Value::Boolean(true);
        let db_bool = toml_value_to_db(&bool_val, "BOOLEAN").unwrap();
        assert!(matches!(db_bool, DbValue::Boolean(true)));
    }

    #[test]
    fn test_prepare_insert_data_db() {
        let table_def = TableDef {
            name: "test_table".to_string(),
            description: None,
            columns: vec![
                ColumnDef {
                    name: "id".to_string(),
                    col_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: true,
                    unique: false,
                    auto_increment: true,
                    default: None,
                    foreign_key: None,
                },
                ColumnDef {
                    name: "name".to_string(),
                    col_type: "TEXT".to_string(),
                    nullable: false,
                    primary_key: false,
                    unique: false,
                    auto_increment: false,
                    default: None,
                    foreign_key: None,
                },
                ColumnDef {
                    name: "active".to_string(),
                    col_type: "BOOLEAN".to_string(),
                    nullable: true,
                    primary_key: false,
                    unique: false,
                    auto_increment: false,
                    default: Some("true".to_string()),
                    foreign_key: None,
                },
            ],
        };

        let mut row_data = HashMap::new();
        row_data.insert("name".to_string(), toml::Value::String("Test User".to_string()));

        let result = prepare_insert_data_db(&table_def, &row_data).unwrap();
        
        // Should not include auto-increment id
        assert!(!result.contains_key("id"));
        
        // Should include provided name
        assert!(matches!(result.get("name"), Some(DbValue::Text(s)) if s == "Test User"));
        
        // Should include default value for active
        assert!(matches!(result.get("active"), Some(DbValue::Boolean(true))));
    }

    #[test]
    fn test_prepare_insert_data_db_missing_required() {
        let table_def = TableDef {
            name: "test_table".to_string(),
            description: None,
            columns: vec![
                ColumnDef {
                    name: "name".to_string(),
                    col_type: "TEXT".to_string(),
                    nullable: false,
                    primary_key: false,
                    unique: false,
                    auto_increment: false,
                    default: None,
                    foreign_key: None,
                },
            ],
        };

        let row_data = HashMap::new(); // No data provided

        let result = prepare_insert_data_db(&table_def, &row_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required field"));
    }
}