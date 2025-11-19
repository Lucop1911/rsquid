use std::time::Duration;

/// Formats a duration into a human-readable string
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Truncates a string to a maximum length, adding "..." if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Validates if a string is a valid database type
pub fn is_valid_db_type(db_type: &str) -> bool {
    matches!(db_type, "postgres" | "mysql" | "sqlite")
}

/// Sanitizes input by removing control characters
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Checks if a query is a read-only operation
pub fn is_readonly_query(query: &str) -> bool {
    let lower = query.trim().to_lowercase();
    lower.starts_with("select")
        || lower.starts_with("show")
        || lower.starts_with("describe")
        || lower.starts_with("explain")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }

    #[test]
    fn test_is_valid_db_type() {
        assert!(is_valid_db_type("postgres"));
        assert!(is_valid_db_type("mysql"));
        assert!(is_valid_db_type("sqlite"));
        assert!(!is_valid_db_type("mongodb"));
    }

    #[test]
    fn test_is_readonly_query() {
        assert!(is_readonly_query("SELECT * FROM users"));
        assert!(is_readonly_query("  select id from table"));
        assert!(!is_readonly_query("INSERT INTO users VALUES (1)"));
        assert!(!is_readonly_query("UPDATE users SET name = 'test'"));
    }
}