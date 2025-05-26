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