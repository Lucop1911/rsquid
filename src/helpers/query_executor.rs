use crate::helpers::connection::Connection;
use anyhow::{Result, anyhow};
use sqlx::mysql::{MySqlColumn, MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::postgres::{PgColumn, PgPool, PgPoolOptions, PgRow};
use sqlx::sqlite::{SqliteColumn, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use std::time::Duration;
use tokio::time::timeout;

pub enum DbPool {
    Postgres(PgPool),
    MySql(MySqlPool),
    Sqlite(SqlitePool),
}

pub struct QueryExecutor {
    pool: DbPool,
}

impl QueryExecutor {
    pub async fn new(connection: &Connection) -> Result<Self> {
        let conn_str = connection.to_connection_string();
        let timeout_duration = Duration::from_secs(5);

        let pool = match connection.db_type.as_str() {
            "postgres" => {
                let p = timeout(
                    timeout_duration,
                    PgPoolOptions::new().max_connections(5).connect(&conn_str),
                )
                .await??;
                DbPool::Postgres(p)
            }
            "mysql" | "mariadb" => {
                let p = timeout(
                    timeout_duration,
                    MySqlPoolOptions::new()
                        .max_connections(5)
                        .connect(&conn_str),
                )
                .await??;
                DbPool::MySql(p)
            }
            "sqlite" => {
                let p = timeout(
                    timeout_duration,
                    SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect(&conn_str),
                )
                .await??;
                DbPool::Sqlite(p)
            }
            _ => return Err(anyhow!("Unsupported database type")),
        };

        Ok(Self { pool })
    }

    pub async fn execute(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        // Split queries by semicolon to handle multiple statements
        let queries: Vec<&str> = query
            .split(';')
            .map(|q| q.trim())
            .filter(|q| !q.is_empty())
            .collect();

        if queries.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut all_headers = Vec::new();
        let mut all_rows = Vec::new();

        for (i, q) in queries.iter().enumerate() {
            // Check if it's a SELECT-like query or an Action query
            let trimmed = q.to_lowercase();
            let query_type = trimmed.starts_with("select")
                || trimmed.starts_with("show")
                || trimmed.starts_with("describe")
                || trimmed.starts_with("explain")
                || trimmed.starts_with("with")
                || trimmed.starts_with("values");

            let (headers, rows) = match &self.pool {
                DbPool::Postgres(p) => self.execute_postgres(p, q, query_type).await?,
                DbPool::MySql(p) => self.execute_mysql(p, q, query_type).await?,
                DbPool::Sqlite(p) => self.execute_sqlite(p, q, query_type).await?,
            };

            // Separator for multiple queries
            if i > 0 && !all_rows.is_empty() {
                all_rows.push(vec!["---".to_string(); headers.len().max(1)]);
            }

            if all_headers.is_empty() {
                all_headers = headers;
            }
            all_rows.extend(rows);
        }

        Ok((all_headers, all_rows))
    }

    pub async fn close(self) -> Result<()> {
        match self.pool {
            DbPool::Postgres(p) => p.close().await,
            DbPool::MySql(p) => p.close().await,
            DbPool::Sqlite(p) => p.close().await,
        }
        Ok(())
    }

    // --- Postgresql Implementation ---

    async fn execute_postgres(
        &self,
        pool: &PgPool,
        query: &str,
        is_query: bool,
    ) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        if !is_query {
            let result = sqlx::query(query).execute(pool).await?;
            return Ok((
                vec!["Result".to_string()],
                vec![vec![format!("{} row(s) affected", result.rows_affected())]],
            ));
        }

        let rows = sqlx::query(query).fetch_all(pool).await?;
        if rows.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let headers: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let mut result_rows = Vec::new();

        for row in rows {
            let mut row_data = Vec::new();
            for (i, col) in row.columns().iter().enumerate() {
                row_data.push(self.pg_value_to_string(&row, i, col));
            }
            result_rows.push(row_data);
        }

        Ok((headers, result_rows))
    }

    fn pg_value_to_string(&self, row: &PgRow, index: usize, col: &PgColumn) -> String {
        if row.try_get_raw(index).map_or(true, |v| v.is_null()) {
            return "NULL".to_string();
        }

        let type_name = col.type_info().name();

        match type_name {
            "BOOL" => row
                .try_get::<bool, _>(index)
                .map(|b| b.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "INT2" | "INT4" | "INT8" => row
                .try_get::<i64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "FLOAT4" | "FLOAT8" | "NUMERIC" => row
                .try_get::<f64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "TEXT" | "VARCHAR" | "CHAR" | "NAME" => {
                row.try_get::<String, _>(index).unwrap_or_default()
            }

            "TIMESTAMP" => row
                .try_get::<chrono::NaiveDateTime, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "TIMESTAMPTZ" => row
                .try_get::<chrono::DateTime<chrono::Utc>, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "DATE" => row
                .try_get::<chrono::NaiveDate, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "UUID" => row
                .try_get::<sqlx::types::Uuid, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "JSON" | "JSONB" => row
                .try_get::<serde_json::Value, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            _ => {
                // Fallback: try as string, then generic debug
                if let Ok(s) = row.try_get::<String, _>(index) {
                    s
                } else {
                    format!("<{}>", type_name)
                }
            }
        }
    }

    // --- MySQL / MariaDB Implementation ---

    async fn execute_mysql(
        &self,
        pool: &MySqlPool,
        query: &str,
        is_query: bool,
    ) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        // MySQL `EXPLAIN` and `DESCRIBE` act like queries
        let actual_is_query = is_query
            || query.to_lowercase().starts_with("describe")
            || query.to_lowercase().starts_with("explain");

        if !actual_is_query {
            let result = sqlx::query(query).execute(pool).await?;
            return Ok((
                vec!["Result".to_string()],
                vec![vec![format!("{} row(s) affected", result.rows_affected())]],
            ));
        }

        let rows = sqlx::query(query).fetch_all(pool).await?;
        if rows.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let headers: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let mut result_rows = Vec::new();

        for row in rows {
            let mut row_data = Vec::new();
            for (i, col) in row.columns().iter().enumerate() {
                row_data.push(self.mysql_value_to_string(&row, i, col));
            }
            result_rows.push(row_data);
        }

        Ok((headers, result_rows))
    }

    fn mysql_value_to_string(&self, row: &MySqlRow, index: usize, col: &MySqlColumn) -> String {
        if row.try_get_raw(index).map_or(true, |v| v.is_null()) {
            return "NULL".to_string();
        }

        let type_name = col.type_info().name();

        match type_name {
            "BOOLEAN" => row
                .try_get::<bool, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "TINYINT" | "SMALLINT" | "INT" | "BIGINT" => row
                .try_get::<i64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "INT UNSIGNED" | "BIGINT UNSIGNED" => row
                .try_get::<u64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "FLOAT" | "DOUBLE" | "DECIMAL" => row
                .try_get::<f64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "DATETIME" | "TIMESTAMP" => row
                .try_get::<chrono::NaiveDateTime, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "DATE" => row
                .try_get::<chrono::NaiveDate, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "JSON" => row
                .try_get::<serde_json::Value, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "VARCHAR" | "CHAR" | "TEXT" | "VAR_STRING" | "BLOB" | "BINARY" => {
                if let Ok(s) = row.try_get::<String, _>(index) {
                    return s;
                }
                // Since reading as string might fail, i attempt to convert bytes to a string
                if let Ok(bytes) = row.try_get::<Vec<u8>, _>(index) {
                    return String::from_utf8_lossy(&bytes).to_string();
                }
                format!("<{}>", type_name)
            }

            _ => {
                // Fallback for any other type: try String, then bytes, then type name
                if let Ok(s) = row.try_get::<String, _>(index) {
                    s
                } else if let Ok(bytes) = row.try_get::<Vec<u8>, _>(index) {
                    String::from_utf8_lossy(&bytes).to_string()
                } else {
                    format!("<{}>", type_name)
                }
            }
        }
    }

    // --- SQLite Implementation ---

    async fn execute_sqlite(
        &self,
        pool: &SqlitePool,
        query: &str,
        is_query: bool,
    ) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        if !is_query {
            let result = sqlx::query(query).execute(pool).await?;
            return Ok((
                vec!["Result".to_string()],
                vec![vec![format!("{} row(s) affected", result.rows_affected())]],
            ));
        }

        let rows = sqlx::query(query).fetch_all(pool).await?;
        if rows.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let headers: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let mut result_rows = Vec::new();

        for row in rows {
            let mut row_data = Vec::new();
            for (i, col) in row.columns().iter().enumerate() {
                row_data.push(self.sqlite_value_to_string(&row, i, col));
            }
            result_rows.push(row_data);
        }

        Ok((headers, result_rows))
    }

    fn sqlite_value_to_string(&self, row: &SqliteRow, index: usize, col: &SqliteColumn) -> String {
        if row.try_get_raw(index).map_or(true, |v| v.is_null()) {
            return "NULL".to_string();
        }

        let type_name = col.type_info().name();

        match type_name {
            "BOOLEAN" => row
                .try_get::<bool, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "INTEGER" => row
                .try_get::<i64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "REAL" => row
                .try_get::<f64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),

            "TEXT" => row.try_get::<String, _>(index).unwrap_or_default(),

            "DATETIME" => row
                .try_get::<chrono::NaiveDateTime, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| {
                    // Sometimes SQLite stores dates as strings
                    row.try_get::<String, _>(index)
                        .unwrap_or_else(|_| "err".to_string())
                }),
                
            _ => {
                if let Ok(s) = row.try_get::<String, _>(index) {
                    s
                } else {
                    format!("<{}>", type_name)
                }
            }
        }
    }
}
