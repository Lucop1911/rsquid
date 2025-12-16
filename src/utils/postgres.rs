use anyhow::{Result};
use sqlx::postgres::{PgColumn, PgPool, PgRow};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use crate::utils::query_executor::QueryExecutor;

impl QueryExecutor {
    pub async fn execute_postgres(
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
}