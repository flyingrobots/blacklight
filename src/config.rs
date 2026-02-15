use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level configuration loaded from `blacklight.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BlacklightConfig {
    /// Path to the SQLite database (supports `~` expansion).
    pub db: String,
    /// Path to the Claude data directory (supports `~` expansion).
    pub claude_dir: String,
    /// Default log level when `RUST_LOG` is not set.
    pub log_level: String,

    pub server: ServerConfig,
    pub indexer: IndexerConfig,
    pub enrichment: EnrichmentConfig,
    pub scheduler: SchedulerConfig,
    pub sqlite: SqliteConfig,
}

impl Default for BlacklightConfig {
    fn default() -> Self {
        Self {
            db: "~/.blacklight/blacklight.db".to_string(),
            claude_dir: "~/.claude/".to_string(),
            log_level: "info".to_string(),
            server: ServerConfig::default(),
            indexer: IndexerConfig::default(),
            enrichment: EnrichmentConfig::default(),
            scheduler: SchedulerConfig::default(),
            sqlite: SqliteConfig::default(),
        }
    }
}

impl BlacklightConfig {
    /// Resolve the database path, expanding `~`.
    pub fn resolved_db_path(&self) -> PathBuf {
        expand_tilde(&self.db)
    }

    /// Resolve the Claude data directory, expanding `~`.
    pub fn resolved_claude_dir(&self) -> PathBuf {
        expand_tilde(&self.claude_dir)
    }

    /// Resolve the log level string.
    pub fn resolved_log_level(&self) -> &str {
        &self.log_level
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub no_open: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3141,
            no_open: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IndexerConfig {
    pub verbose: bool,
    pub skip_dirs: Vec<String>,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            skip_dirs: vec![
                "cache".into(),
                "statsig".into(),
                "shell-snapshots".into(),
                "session-env".into(),
                "ide".into(),
                "paste-cache".into(),
                "debug".into(),
                "telemetry".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EnrichmentConfig {
    pub concurrency: usize,
    pub auto_approve_threshold: f64,
    pub ollama_url: String,
    pub ollama_model: String,
    pub google_api_key: String,
    pub preferred_backend: String,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            concurrency: 5,
            auto_approve_threshold: 0.80,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: String::new(),
            google_api_key: String::new(),
            preferred_backend: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub run_enrichment: bool,
    pub enrichment_concurrency: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 60,
            run_enrichment: true,
            enrichment_concurrency: 5,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SqliteConfig {
    pub cache_size_mb: u32,
    pub mmap_size_mb: u32,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            cache_size_mb: 64,
            mmap_size_mb: 256,
        }
    }
}

/// Expand a leading `~` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

/// Returns the default config file path: `~/.blacklight/blacklight.toml`.
pub fn default_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".blacklight")
        .join("blacklight.toml")
}

/// Load configuration from a TOML file.
///
/// - If `path` is `Some`, reads that file (errors if missing or invalid).
/// - If `path` is `None`, tries the default path; returns defaults if the file doesn't exist.
pub fn load_config(path: Option<&Path>) -> Result<BlacklightConfig> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => default_config_path(),
    };

    if !config_path.exists() {
        if path.is_some() {
            // User explicitly specified a path that doesn't exist — error
            anyhow::bail!("config file not found: {}", config_path.display());
        }
        // Default path doesn't exist — use defaults
        return Ok(BlacklightConfig::default());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;

    let config: BlacklightConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", config_path.display()))?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BlacklightConfig::default();
        assert_eq!(config.server.port, 3141);
        assert_eq!(config.enrichment.concurrency, 5);
        assert_eq!(config.sqlite.cache_size_mb, 64);
        assert_eq!(config.indexer.skip_dirs.len(), 8);
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/foo/bar");
        assert!(expanded.to_string_lossy().ends_with("foo/bar"));
        assert!(!expanded.to_string_lossy().starts_with("~"));

        // Non-tilde path stays unchanged
        let plain = expand_tilde("/absolute/path");
        assert_eq!(plain, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_load_missing_default_returns_defaults() {
        let config = load_config(None).unwrap();
        assert_eq!(config.server.port, 3141);
    }

    #[test]
    fn test_load_explicit_missing_errors() {
        let result = load_config(Some(Path::new("/nonexistent/blacklight.toml")));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
            log_level = "debug"

            [server]
            port = 9999
        "#;
        let config: BlacklightConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.server.port, 9999);
        // Unset fields get defaults
        assert_eq!(config.enrichment.concurrency, 5);
        assert_eq!(config.sqlite.cache_size_mb, 64);
    }

    #[test]
    fn test_default_config_path() {
        let path = default_config_path();
        assert!(path.to_string_lossy().ends_with(".blacklight/blacklight.toml"));
    }
}
