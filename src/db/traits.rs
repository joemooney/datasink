use async_trait::async_trait;
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::db::error::Result;

#[derive(Debug, Clone)]
pub enum ColumnType {
    Integer,
    Real,
    Text,
    Blob,
    Boolean,
    Timestamp,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DbValue {
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Boolean(bool),
    Timestamp(i64),
    Null,
}

#[derive(Debug)]
pub struct QueryResult {
    pub columns: Vec<(String, ColumnType)>,
    pub rows: Vec<Vec<DbValue>>,
}

pub type StreamedQueryResult = Pin<Box<dyn Stream<Item = Result<Vec<DbValue>>> + Send>>;

#[async_trait]
pub trait Database: Send + Sync {
    async fn connect(connection_string: &str) -> Result<Self>
    where
        Self: Sized;

    async fn create_table(&self, table_name: &str, columns: Vec<ColumnDef>) -> Result<()>;

    async fn drop_table(&self, table_name: &str) -> Result<()>;

    async fn insert(&self, table_name: &str, values: HashMap<String, DbValue>) -> Result<i64>;

    async fn update(
        &self,
        table_name: &str,
        values: HashMap<String, DbValue>,
        where_clause: &str,
    ) -> Result<u64>;

    async fn delete(&self, table_name: &str, where_clause: &str) -> Result<u64>;

    async fn query(&self, sql: &str, params: HashMap<String, DbValue>) -> Result<QueryResult>;

    async fn query_stream(
        &self,
        sql: &str,
        params: HashMap<String, DbValue>,
    ) -> Result<(Vec<(String, ColumnType)>, StreamedQueryResult)>;

    async fn batch_insert(
        &self,
        table_name: &str,
        rows: Vec<HashMap<String, DbValue>>,
    ) -> Result<u64>;
}
