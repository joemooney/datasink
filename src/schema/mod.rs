pub mod parser;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Schema {
    pub database: DatabaseInfo,
    pub tables: Vec<TableDef>,
    #[serde(default)]
    pub data: HashMap<String, Vec<HashMap<String, toml::Value>>>,
    #[serde(default)]
    pub indexes: Vec<IndexDef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TableDef {
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub auto_increment: bool,
    pub default: Option<String>,
    pub foreign_key: Option<ForeignKeyDef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForeignKeyDef {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexDef {
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
}