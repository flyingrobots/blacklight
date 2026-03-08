use regex::Regex;
use std::borrow::Cow;

pub struct Redactor {
    patterns: Vec<Regex>,
}

impl Redactor {
    pub fn new(custom_patterns: &[String]) -> Self {
        let mut patterns = Vec::new();

        // Default sensitive patterns
        let defaults = [
            r"sk-[a-zA-Z0-9]{48}", // OpenAI
            r"xox[bapts]-[a-zA-Z0-9-]{10,}", // Slack
            r"ghp_[a-zA-Z0-9]{36}", // GitHub
            r"(?:https?://)[a-zA-Z0-9]{20,}:[a-zA-Z0-9]{20,}@", // Basic auth
            r"AIza[0-9A-Za-z-_]{35}", // Google API Key
        ];

        for p in defaults {
            if let Ok(re) = Regex::new(p) {
                patterns.push(re);
            }
        }

        for p in custom_patterns {
            if let Ok(re) = Regex::new(p) {
                patterns.push(re);
            } else {
                tracing::warn!("invalid redaction regex: {}", p);
            }
        }

        Self { patterns }
    }

    pub fn redact<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let mut current = Cow::Borrowed(text);
        
        for re in &self.patterns {
            if re.is_match(&current) {
                let owned = re.replace_all(&current, "[REDACTED]").into_owned();
                current = Cow::Owned(owned);
            }
        }

        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redaction() {
        let r = Redactor::new(&[]);
        assert_eq!(r.redact("my key is sk-123456789012345678901234567890123456789012345678"), "my key is [REDACTED]");
        assert_eq!(r.redact("ghp_123456789012345678901234567890123456"), "[REDACTED]");
        assert_eq!(r.redact("safe text"), "safe text");
    }
}
