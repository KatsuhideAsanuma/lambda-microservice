
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_script_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        format!("{:.2}m", ms as f64 / 60000.0)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes < KB {
        format!("{}B", bytes)
    } else if bytes < MB {
        format!("{:.2}KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2}MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2}GB", bytes as f64 / GB as f64)
    }
}

pub fn truncate_string(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_length - 3])
    }
}

pub fn sanitize_for_logging(s: &str) -> String {
    s.replace('\n', "\\n").replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_script_hash() {
        let content = "function test() { return 42; }";
        let hash = generate_script_hash(content);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hash is 64 hex characters
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_ms(500), "500ms");
        assert_eq!(format_duration_ms(1500), "1.50s");
        assert_eq!(format_duration_ms(90000), "1.50m");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1500), "1.46KB");
        assert_eq!(format_bytes(1500000), "1.43MB");
        assert_eq!(format_bytes(1500000000), "1.40GB");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("Hello", 10), "Hello");
        assert_eq!(truncate_string("Hello, world!", 10), "Hello,...");
    }

    #[test]
    fn test_sanitize_for_logging() {
        assert_eq!(sanitize_for_logging("Hello\nworld"), "Hello\\nworld");
        assert_eq!(sanitize_for_logging("Hello\r\nworld"), "Hello\\r\\nworld");
    }
}
