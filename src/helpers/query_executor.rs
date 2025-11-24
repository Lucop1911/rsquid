use crate::helpers::connection::Connection;
use anyhow::Result;
use sqlx::any::{ AnyPoolOptions, AnyRow};
use sqlx::{AnyPool, Column, Row, ValueRef};
use tokio::time::{timeout, Duration};

pub struct QueryExecutor {
    pool: AnyPool,
}

impl QueryExecutor {
    // Executor, 5 sec timeout
    pub async fn new(connection: &Connection) -> Result<Self> {
        let conn_str = connection.to_connection_string();

        let pool = match timeout(
            Duration::from_secs(5),
            AnyPoolOptions::new()
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

    // Execute query, distinguish from SELECT-like and non-SELECT queries
    pub async fn execute(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        // Split by semicolon and filter out empty queries
        let queries: Vec<&str> = query
            .split(';')
            .map(|q| q.trim())
            .filter(|q| !q.is_empty())
            .collect();

        if queries.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        // 1 query
        if queries.len() == 1 {
            let trimmed = queries[0].trim().to_lowercase();
            if trimmed.starts_with("select")
                || trimmed.starts_with("show")
                || trimmed.starts_with("describe")
                || trimmed.starts_with("explain")
            {
                return self.execute_query(queries[0]).await;
            } else {
                return self.execute_non_query(queries[0]).await;
            }
        }

        // Multiple queries
        let mut all_headers = Vec::new();
        let mut all_rows = Vec::new();

        for (i, q) in queries.iter().enumerate() {
            let trimmed = q.trim().to_lowercase();
            let (headers, rows) = if trimmed.starts_with("select")
                || trimmed.starts_with("show")
                || trimmed.starts_with("describe")
                || trimmed.starts_with("explain")
            {
                self.execute_query(q).await?
            } else {
                self.execute_non_query(q).await?
            };

            if i > 0 && !all_rows.is_empty() {
                all_rows.push(vec!["---".to_string(); headers.len().max(1)]);
            }

            // Combine results
            if all_headers.is_empty() {
                all_headers = headers.clone();
            }
            all_rows.extend(rows);
        }

        Ok((all_headers, all_rows))
    }

    // Execute SELECT/SHOW/DESCRIBE/EXPLAIN queries
    async fn execute_query(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let rows: Vec<AnyRow> = sqlx::query(query)
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

    // Execute INSERT/UPDATE/DELETE/other commands
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

    // Convert types
    fn row_value_to_string(row: &AnyRow, index: usize) -> String {
        let value_ref = row.try_get_raw(index);
        
        // NULL
        if let Ok(val) = value_ref {
            if val.is_null() {
                return "NULL".to_string();
            }
        }
                
        // String
        if let Ok(v) = row.try_get::<String, _>(index) {
            return v;
        }
        
        // Integers
        if let Ok(v) = row.try_get::<i32, _>(index) {
            return v.to_string();
        }
        if let Ok(v) = row.try_get::<i64, _>(index) {
            return v.to_string();
        }
        
        // Floating point
        if let Ok(v) = row.try_get::<f32, _>(index) {
            return v.to_string();
        }
        if let Ok(v) = row.try_get::<f64, _>(index) {
            return v.to_string();
        }
        
        // Bool
        if let Ok(v) = row.try_get::<bool, _>(index) {
            return v.to_string();
        }
        
        // Byte slice to UTF-8
        if let Ok(bytes) = row.try_get::<&[u8], _>(index) {
            if let Ok(s) = std::str::from_utf8(bytes) {
                return s.to_string();
            }
            return format!("<binary: {} bytes>", bytes.len());
        }
        
        // Byte vector to UTF-8 string
        if let Ok(bytes) = row.try_get::<Vec<u8>, _>(index) {
            match String::from_utf8(bytes) {
                Ok(s) => return s,
                Err(e) => return format!("<binary: {} bytes>", e.as_bytes().len()),
            }
        }
                
        "<unknown>".to_string()
    }

    // Close pool
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