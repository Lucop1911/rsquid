use crate::helpers::connection::Connection;
use anyhow::{Context, Result};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::{Column, Row};
use tokio::time::{timeout, Duration};

pub struct QueryExecutor {
    pool: MySqlPool,
}

impl QueryExecutor {
    /// Create a new executor with a 5-second connection timeout
    pub async fn new(connection: &Connection) -> Result<Self> {
        let conn_str = connection.to_connection_string();

        let pool = match timeout(
            Duration::from_secs(5),
            MySqlPoolOptions::new()
                .max_connections(5)
                .connect(&conn_str),
        )
        .await
        {
            Ok(Ok(pool)) => pool,
            Ok(Err(e)) => anyhow::bail!("Failed to connect to database: {e}"),
            Err(_) => anyhow::bail!("Failed to connect to database: connection timed out"),
        };

        Ok(Self { pool })
    }

    /// Executes a query, distinguishing between SELECT-like and non-SELECT queries
    pub async fn execute(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let trimmed = query.trim().to_lowercase();

        if trimmed.starts_with("select")
            || trimmed.starts_with("show")
            || trimmed.starts_with("describe")
            || trimmed.starts_with("explain")
        {
            self.execute_query(query).await
        } else {
            self.execute_non_query(query).await
        }
    }

    /// Executes SELECT/SHOW/DESCRIBE/EXPLAIN queries
    async fn execute_query(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let rows: Vec<MySqlRow> = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("SQL execution failed: {:?}", e))?;

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
            for i in 0..row.columns().len() {
                let val = Self::row_value_to_string(&row, i);
                row_data.push(val);
            }
            result_rows.push(row_data);
        }

        Ok((headers, result_rows))
    }

    /// Executes INSERT/UPDATE/DELETE/other commands
    async fn execute_non_query(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let result = sqlx::query(query)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("SQL execution failed: {:?}", e))?;

        let rows_affected = result.rows_affected();

        let headers = vec!["Result".to_string()];
        let rows = vec![vec![format!("{} row(s) affected", rows_affected)]];

        Ok((headers, rows))
    }

    /// Converts MySQL row values into strings safely, handling NULLs
    fn row_value_to_string(row: &MySqlRow, index: usize) -> String {
        if let Ok(Some(val)) = row.try_get::<Option<String>, _>(index) {
            return val;
        }
        if let Ok(Some(val)) = row.try_get::<Option<i32>, _>(index) {
            return val.to_string();
        }
        if let Ok(Some(val)) = row.try_get::<Option<i64>, _>(index) {
            return val.to_string();
        }
        if let Ok(Some(val)) = row.try_get::<Option<f64>, _>(index) {
            return val.to_string();
        }
        if let Ok(Some(val)) = row.try_get::<Option<bool>, _>(index) {
            return val.to_string();
        }

        "NULL".to_string()
    }

    /// Close the pool gracefully
    pub async fn close(self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::connection::Connection;

    #[tokio::test]
    async fn test_connection_and_query() {
        let conn = Connection {
            name: "test".to_string(),
            db_type: "mysql".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3306,
            database: "dbname".to_string(),
            username: "root".to_string(),
            password: "admin".to_string(),
        };

        let executor = QueryExecutor::new(&conn).await.unwrap();
        let (headers, rows) = executor.execute("SELECT 1 AS test_col;").await.unwrap();

        assert_eq!(headers, vec!["test_col".to_string()]);
        assert_eq!(rows, vec![vec!["1".to_string()]]);
    }
}
