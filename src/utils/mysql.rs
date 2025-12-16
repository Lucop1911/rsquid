use crate::utils::query_executor::QueryExecutor;
use anyhow::{Result};
use sqlx::mysql::{MySqlColumn, MySqlPool, MySqlRow};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use bigdecimal::BigDecimal;

impl QueryExecutor {
    pub async fn execute_mysql(
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

            "FLOAT" | "DOUBLE" => row
                .try_get::<f64, _>(index)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "err".to_string()),
                
            "DECIMAL" | "NEWDECIMAL" => row
                .try_get::<BigDecimal, _>(index)
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
                if let Ok(bytes) = row.try_get::<Vec<u8>, _>(index) {
                    return String::from_utf8_lossy(&bytes).to_string();
                }
                format!("<{}>", type_name)
            }

            _ => {
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
}