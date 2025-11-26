use crate::helpers::connection::Connection;
use anyhow::{Result, anyhow};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
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
}
