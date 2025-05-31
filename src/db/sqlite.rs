use async_trait::async_trait;
use futures::stream;
use sqlx::{sqlite::SqlitePool, Row, Sqlite, Column};
use std::collections::HashMap;

use crate::db::{
    error::{DatabaseError, Result},
    traits::{ColumnDef, ColumnType, Database, DbValue, QueryResult, StreamedQueryResult},
};

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    fn column_type_to_sql(col_type: &ColumnType) -> &'static str {
        match col_type {
            ColumnType::Integer => "INTEGER",
            ColumnType::Real => "REAL",
            ColumnType::Text => "TEXT",
            ColumnType::Blob => "BLOB",
            ColumnType::Boolean => "INTEGER",
            ColumnType::Timestamp => "INTEGER",
        }
    }

    fn build_create_table_sql(table_name: &str, columns: &[ColumnDef]) -> String {
        let mut sql = format!("CREATE TABLE {} (", table_name);

        let column_defs: Vec<String> = columns
            .iter()
            .map(|col| {
                let mut def = format!("{} {}", col.name, Self::column_type_to_sql(&col.col_type));

                if col.primary_key {
                    def.push_str(" PRIMARY KEY");
                }
                if !col.nullable && !col.primary_key {
                    def.push_str(" NOT NULL");
                }
                if col.unique && !col.primary_key {
                    def.push_str(" UNIQUE");
                }
                if let Some(default) = &col.default_value {
                    def.push_str(&format!(" DEFAULT {}", default));
                }

                def
            })
            .collect();

        sql.push_str(&column_defs.join(", "));
        sql.push(')');

        sql
    }

    fn bind_value<'q>(
        query: sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
        value: &'q DbValue,
    ) -> sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        match value {
            DbValue::Integer(v) => query.bind(v),
            DbValue::Real(v) => query.bind(v),
            DbValue::Text(v) => query.bind(v),
            DbValue::Blob(v) => query.bind(v),
            DbValue::Boolean(v) => query.bind(*v as i32),
            DbValue::Timestamp(v) => query.bind(v),
            DbValue::Null => query.bind(None::<i32>),
        }
    }

    async fn row_to_values(row: &sqlx::sqlite::SqliteRow) -> Result<Vec<DbValue>> {
        use sqlx::Row;
        use sqlx::ValueRef;
        
        let mut values = Vec::new();
        let column_count = row.len();

        for i in 0..column_count {
            // First check if the value is NULL
            let raw_value = row.try_get_raw(i)?;
            if raw_value.is_null() {
                values.push(DbValue::Null);
                continue;
            }
            
            // Try different types in order
            let value = if let Ok(v) = row.try_get::<i64, _>(i) {
                // Check if this might be a boolean (0 or 1)
                // We need column type info to determine this properly
                // For now, we'll treat all integers as integers
                DbValue::Integer(v)
            } else if let Ok(v) = row.try_get::<f64, _>(i) {
                DbValue::Real(v)
            } else if let Ok(v) = row.try_get::<String, _>(i) {
                DbValue::Text(v)
            } else if let Ok(v) = row.try_get::<Vec<u8>, _>(i) {
                DbValue::Blob(v)
            } else {
                DbValue::Null
            };

            values.push(value);
        }

        Ok(values)
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn connect(connection_string: &str) -> Result<Self> {
        let pool = SqlitePool::connect(connection_string)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        Ok(SqliteDatabase { pool })
    }

    async fn create_table(&self, table_name: &str, columns: Vec<ColumnDef>) -> Result<()> {
        let sql = Self::build_create_table_sql(table_name, &columns);

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(db_err) if db_err.message().contains("already exists") => {
                    DatabaseError::TableAlreadyExists(table_name.to_string())
                }
                _ => DatabaseError::from(e),
            })?;

        Ok(())
    }

    async fn drop_table(&self, table_name: &str) -> Result<()> {
        let sql = format!("DROP TABLE IF EXISTS {}", table_name);

        sqlx::query(&sql).execute(&self.pool).await?;

        Ok(())
    }

    async fn insert(&self, table_name: &str, values: HashMap<String, DbValue>) -> Result<i64> {
        if values.is_empty() {
            return Err(DatabaseError::QueryError("No values provided".to_string()));
        }

        let columns: Vec<&String> = values.keys().collect();
        let placeholders: Vec<String> = (0..columns.len()).map(|i| format!("?{}", i + 1)).collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            placeholders.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for (_, value) in values.iter() {
            query = Self::bind_value(query, value);
        }

        let result = query.execute(&self.pool).await?;
        Ok(result.last_insert_rowid())
    }

    async fn update(
        &self,
        table_name: &str,
        values: HashMap<String, DbValue>,
        where_clause: &str,
    ) -> Result<u64> {
        if values.is_empty() {
            return Err(DatabaseError::QueryError("No values provided".to_string()));
        }

        let set_clauses: Vec<String> = values
            .keys()
            .enumerate()
            .map(|(i, col)| format!("{} = ?{}", col, i + 1))
            .collect();

        let sql = format!(
            "UPDATE {} SET {} WHERE {}",
            table_name,
            set_clauses.join(", "),
            where_clause
        );

        let mut query = sqlx::query(&sql);
        for (_, value) in values.iter() {
            query = Self::bind_value(query, value);
        }

        let result = query.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn delete(&self, table_name: &str, where_clause: &str) -> Result<u64> {
        let sql = format!("DELETE FROM {} WHERE {}", table_name, where_clause);

        let result = sqlx::query(&sql).execute(&self.pool).await?;

        Ok(result.rows_affected())
    }

    async fn query(&self, sql: &str, params: HashMap<String, DbValue>) -> Result<QueryResult> {
        let mut query = sqlx::query(sql);

        for (_, value) in params.iter() {
            query = Self::bind_value(query, value);
        }

        let rows = query.fetch_all(&self.pool).await?;

        if rows.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
            });
        }

        // Get column information from the first row
        let first_row = &rows[0];
        let columns = first_row.columns()
            .iter()
            .map(|col| {
                let col_name = col.name().to_string();
                // Try to determine column type - default to Text for simplicity
                // In a more robust implementation, we'd query sqlite_master or use type info
                (col_name, ColumnType::Text)
            })
            .collect();

        let mut result_rows = Vec::new();
        for row in rows.iter() {
            result_rows.push(Self::row_to_values(row).await?);
        }

        Ok(QueryResult {
            columns,
            rows: result_rows,
        })
    }

    async fn query_stream(
        &self,
        sql: &str,
        params: HashMap<String, DbValue>,
    ) -> Result<(Vec<(String, ColumnType)>, StreamedQueryResult)> {
        // Check if this is a non-SELECT query (INSERT, UPDATE, DELETE)
        let trimmed_sql = sql.trim().to_uppercase();
        if trimmed_sql.starts_with("INSERT") || trimmed_sql.starts_with("UPDATE") || trimmed_sql.starts_with("DELETE") {
            // Execute the non-SELECT query
            let mut query = sqlx::query(sql);
            for (_, value) in params.iter() {
                query = Self::bind_value(query, value);
            }
            let result = query.execute(&self.pool).await?;
            let affected_rows = result.rows_affected();
            
            // Return a synthetic result set with the affected rows count
            let columns = vec![("affected_rows".to_string(), ColumnType::Integer)];
            let rows = vec![vec![DbValue::Integer(affected_rows as i64)]];
            
            let stream = Box::pin(stream::iter(
                rows.into_iter()
                    .map(Ok)
                    .collect::<Vec<Result<Vec<DbValue>>>>(),
            ));
            
            return Ok((columns, stream));
        }
        
        // For SELECT queries, execute normally
        let result = self.query(sql, params).await?;

        let columns = result.columns;
        let rows = result.rows;

        // Convert the rows into a stream
        let stream = Box::pin(stream::iter(
            rows.into_iter()
                .map(Ok)
                .collect::<Vec<Result<Vec<DbValue>>>>(),
        ));

        Ok((columns, stream))
    }

    async fn batch_insert(
        &self,
        table_name: &str,
        rows: Vec<HashMap<String, DbValue>>,
    ) -> Result<u64> {
        if rows.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for row in rows {
            if row.is_empty() {
                continue;
            }

            let columns: Vec<&String> = row.keys().collect();
            let placeholders: Vec<String> =
                (0..columns.len()).map(|i| format!("?{}", i + 1)).collect();

            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns
                    .iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                placeholders.join(", ")
            );

            let mut query = sqlx::query(&sql);
            for (_, value) in row.iter() {
                query = Self::bind_value(query, value);
            }

            query.execute(&mut *tx).await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }
}
