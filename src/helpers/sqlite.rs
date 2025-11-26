use crate::helpers::query_executor::QueryExecutor;
use anyhow::{Result};
use sqlx::sqlite::{SqliteColumn, SqlitePool, SqliteRow};
use sqlx::{Column, Row, TypeInfo, ValueRef};

impl QueryExecutor {
    pub async fn execute_sqlite(
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