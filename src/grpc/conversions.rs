use crate::db::traits::{ColumnDef, ColumnType, DbValue};
use crate::proto::{self, ColumnDefinition, DataType, Value as ProtoValue};
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
        Some(proto::value::Value::IntValue(v)) => DbValue::Integer(v),
        Some(proto::value::Value::RealValue(v)) => DbValue::Real(v),
        Some(proto::value::Value::TextValue(v)) => DbValue::Text(v),
        Some(proto::value::Value::BlobValue(v)) => DbValue::Blob(v),
        Some(proto::value::Value::BoolValue(v)) => DbValue::Boolean(v),
        Some(proto::value::Value::TimestampValue(v)) => DbValue::Timestamp(v),
        Some(proto::value::Value::NullValue(_)) => DbValue::Null,
        None => DbValue::Null,
    }
}

pub fn db_value_to_proto(value: DbValue) -> ProtoValue {
    let proto_value = match value {
        DbValue::Integer(v) => proto::value::Value::IntValue(v),
        DbValue::Real(v) => proto::value::Value::RealValue(v),
        DbValue::Text(v) => proto::value::Value::TextValue(v),
        DbValue::Blob(v) => proto::value::Value::BlobValue(v),
        DbValue::Boolean(v) => proto::value::Value::BoolValue(v),
        DbValue::Timestamp(v) => proto::value::Value::TimestampValue(v),
        DbValue::Null => proto::value::Value::NullValue(true),
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
