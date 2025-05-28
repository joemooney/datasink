use crate::db::traits::{ColumnDef, ColumnType, DbValue};
use crate::proto::common::{ColumnDefinition, DataType, Value as ProtoValue, value};
use std::collections::HashMap;

pub fn proto_to_column_type(data_type: DataType) -> ColumnType {
    match data_type {
        DataType::Integer => ColumnType::Integer,
        DataType::Real => ColumnType::Real,
        DataType::Text => ColumnType::Text,
        DataType::Blob => ColumnType::Blob,
        DataType::Boolean => ColumnType::Boolean,
        DataType::Timestamp => ColumnType::Timestamp,
    }
}

pub fn column_type_to_proto(col_type: &ColumnType) -> DataType {
    match col_type {
        ColumnType::Integer => DataType::Integer,
        ColumnType::Real => DataType::Real,
        ColumnType::Text => DataType::Text,
        ColumnType::Blob => DataType::Blob,
        ColumnType::Boolean => DataType::Boolean,
        ColumnType::Timestamp => DataType::Timestamp,
    }
}

pub fn proto_to_column_def(def: ColumnDefinition) -> ColumnDef {
    ColumnDef {
        name: def.name,
        col_type: proto_to_column_type(DataType::try_from(def.r#type).unwrap_or(DataType::Text)),
        nullable: def.nullable,
        primary_key: def.primary_key,
        unique: def.unique,
        default_value: if def.default_value.is_empty() {
            None
        } else {
            Some(def.default_value)
        },
    }
}

pub fn proto_to_db_value(value: ProtoValue) -> DbValue {
    match value.value {
        Some(value::Value::IntValue(v)) => DbValue::Integer(v),
        Some(value::Value::RealValue(v)) => DbValue::Real(v),
        Some(value::Value::TextValue(v)) => DbValue::Text(v),
        Some(value::Value::BlobValue(v)) => DbValue::Blob(v),
        Some(value::Value::BoolValue(v)) => DbValue::Boolean(v),
        Some(value::Value::TimestampValue(v)) => DbValue::Timestamp(v),
        Some(value::Value::NullValue(_)) => DbValue::Null,
        None => DbValue::Null,
    }
}

pub fn db_value_to_proto(value: DbValue) -> ProtoValue {
    let proto_value = match value {
        DbValue::Integer(v) => value::Value::IntValue(v),
        DbValue::Real(v) => value::Value::RealValue(v),
        DbValue::Text(v) => value::Value::TextValue(v),
        DbValue::Blob(v) => value::Value::BlobValue(v),
        DbValue::Boolean(v) => value::Value::BoolValue(v),
        DbValue::Timestamp(v) => value::Value::TimestampValue(v),
        DbValue::Null => value::Value::NullValue(true),
    };

    ProtoValue {
        value: Some(proto_value),
    }
}

pub fn proto_values_to_db_values(values: HashMap<String, ProtoValue>) -> HashMap<String, DbValue> {
    values
        .into_iter()
        .map(|(k, v)| (k, proto_to_db_value(v)))
        .collect()
}

pub fn db_values_to_proto_values(values: Vec<DbValue>) -> Vec<ProtoValue> {
    values.into_iter().map(db_value_to_proto).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proto_to_column_type() {
        assert!(matches!(proto_to_column_type(DataType::Integer), ColumnType::Integer));
        assert!(matches!(proto_to_column_type(DataType::Real), ColumnType::Real));
        assert!(matches!(proto_to_column_type(DataType::Text), ColumnType::Text));
        assert!(matches!(proto_to_column_type(DataType::Blob), ColumnType::Blob));
        assert!(matches!(proto_to_column_type(DataType::Boolean), ColumnType::Boolean));
        assert!(matches!(proto_to_column_type(DataType::Timestamp), ColumnType::Timestamp));
    }

    #[test]
    fn test_column_type_to_proto() {
        assert_eq!(column_type_to_proto(&ColumnType::Integer), DataType::Integer);
        assert_eq!(column_type_to_proto(&ColumnType::Real), DataType::Real);
        assert_eq!(column_type_to_proto(&ColumnType::Text), DataType::Text);
        assert_eq!(column_type_to_proto(&ColumnType::Blob), DataType::Blob);
        assert_eq!(column_type_to_proto(&ColumnType::Boolean), DataType::Boolean);
        assert_eq!(column_type_to_proto(&ColumnType::Timestamp), DataType::Timestamp);
    }

    #[test]
    fn test_proto_to_column_def() {
        let proto_def = ColumnDefinition {
            name: "test_col".to_string(),
            r#type: DataType::Integer as i32,
            nullable: false,
            primary_key: true,
            unique: false,
            default_value: "0".to_string(),
        };

        let db_def = proto_to_column_def(proto_def);
        assert_eq!(db_def.name, "test_col");
        assert!(matches!(db_def.col_type, ColumnType::Integer));
        assert!(!db_def.nullable);
        assert!(db_def.primary_key);
        assert!(!db_def.unique);
        assert_eq!(db_def.default_value, Some("0".to_string()));
    }

    #[test]
    fn test_proto_to_column_def_empty_default() {
        let proto_def = ColumnDefinition {
            name: "test_col".to_string(),
            r#type: DataType::Text as i32,
            nullable: true,
            primary_key: false,
            unique: true,
            default_value: "".to_string(),
        };

        let db_def = proto_to_column_def(proto_def);
        assert_eq!(db_def.default_value, None);
    }

    #[test]
    fn test_proto_to_db_value() {
        // Test integer
        let proto_int = ProtoValue {
            value: Some(value::Value::IntValue(42)),
        };
        assert!(matches!(proto_to_db_value(proto_int), DbValue::Integer(42)));

        // Test real
        let proto_real = ProtoValue {
            value: Some(value::Value::RealValue(3.14)),
        };
        if let DbValue::Real(v) = proto_to_db_value(proto_real) {
            assert!((v - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("Expected DbValue::Real");
        }

        // Test text
        let proto_text = ProtoValue {
            value: Some(value::Value::TextValue("hello".to_string())),
        };
        assert!(matches!(proto_to_db_value(proto_text), DbValue::Text(s) if s == "hello"));

        // Test boolean
        let proto_bool = ProtoValue {
            value: Some(value::Value::BoolValue(true)),
        };
        assert!(matches!(proto_to_db_value(proto_bool), DbValue::Boolean(true)));

        // Test null
        let proto_null = ProtoValue {
            value: Some(value::Value::NullValue(true)),
        };
        assert!(matches!(proto_to_db_value(proto_null), DbValue::Null));

        // Test None
        let proto_none = ProtoValue { value: None };
        assert!(matches!(proto_to_db_value(proto_none), DbValue::Null));
    }

    #[test]
    fn test_db_value_to_proto() {
        // Test integer
        let db_int = DbValue::Integer(42);
        let proto_int = db_value_to_proto(db_int);
        assert!(matches!(proto_int.value, Some(value::Value::IntValue(42))));

        // Test real
        let db_real = DbValue::Real(3.14);
        let proto_real = db_value_to_proto(db_real);
        if let Some(value::Value::RealValue(v)) = proto_real.value {
            assert!((v - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("Expected RealValue");
        }

        // Test text
        let db_text = DbValue::Text("hello".to_string());
        let proto_text = db_value_to_proto(db_text);
        assert!(matches!(proto_text.value, Some(value::Value::TextValue(s)) if s == "hello"));

        // Test null
        let db_null = DbValue::Null;
        let proto_null = db_value_to_proto(db_null);
        assert!(matches!(proto_null.value, Some(value::Value::NullValue(true))));
    }

    #[test]
    fn test_proto_values_to_db_values() {
        let mut proto_values = HashMap::new();
        proto_values.insert(
            "id".to_string(),
            ProtoValue {
                value: Some(value::Value::IntValue(1)),
            },
        );
        proto_values.insert(
            "name".to_string(),
            ProtoValue {
                value: Some(value::Value::TextValue("test".to_string())),
            },
        );

        let db_values = proto_values_to_db_values(proto_values);
        assert_eq!(db_values.len(), 2);
        assert!(matches!(db_values.get("id"), Some(DbValue::Integer(1))));
        assert!(matches!(db_values.get("name"), Some(DbValue::Text(s)) if s == "test"));
    }

    #[test]
    fn test_db_values_to_proto_values() {
        let db_values = vec![
            DbValue::Integer(1),
            DbValue::Text("test".to_string()),
            DbValue::Boolean(true),
            DbValue::Null,
        ];

        let proto_values = db_values_to_proto_values(db_values);
        assert_eq!(proto_values.len(), 4);
        assert!(matches!(proto_values[0].value, Some(value::Value::IntValue(1))));
        assert!(matches!(&proto_values[1].value, Some(value::Value::TextValue(s)) if s == "test"));
        assert!(matches!(proto_values[2].value, Some(value::Value::BoolValue(true))));
        assert!(matches!(proto_values[3].value, Some(value::Value::NullValue(true))));
    }
}
