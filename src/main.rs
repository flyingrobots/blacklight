use blacklight::config::{self, BlacklightConfig};
use blacklight::{enrich, indexer, server};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "blacklight", version, about = "Index and explore ~/.claude/ data")]
struct Cli {
    /// Custom database path
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    /// Custom ~/.claude/ directory path
    #[arg(long, global = true)]
    claude_dir: Option<PathBuf>,

    /// Path to config file (default: ~/.blacklight/blacklight.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index ~/.claude/ data into the local database
    Index {
        /// Force full re-index (ignore previously indexed state)
        #[arg(long)]
        full: bool,

        /// Source directory to index (defaults to ~/.claude/)
        #[arg(long)]
        source: Option<PathBuf>,

        /// Enable verbose per-file logging
        #[arg(long)]
        verbose: bool,
    },

    /// Start the web server and open the dashboard
    Serve {
        /// Port to listen on
        #[arg(long)]
        port: Option<u16>,

        /// Don't auto-open browser
        #[arg(long)]
        no_open: bool,
    },

    /// Full-text search across indexed content
    Search {
        /// Search query
        query: String,

        /// Filter by project slug
        #[arg(long)]
        project: Option<String>,

        /// Filter by content kind (text, tool_output, thinking, plan)
        #[arg(long)]
        kind: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: u32,

        /// Filter by start date (ISO 8601)
        #[arg(long)]
        from: Option<String>,

        /// Filter by end date (ISO 8601)
        #[arg(long)]
        to: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Generate AI titles, summaries, and tags for sessions using Claude
    Enrich {
        /// Max sessions to enrich
        #[arg(long)]
        limit: Option<usize>,

        /// Number of concurrent enrichment calls
        #[arg(long)]
        concurrency: Option<usize>,

        /// Re-enrich already enriched sessions
        #[arg(long)]
        force: bool,
    },

    /// Show usage statistics
    Stats {
        /// Show daily activity breakdown
        #[arg(long)]
        daily: bool,

        /// Show per-model token breakdown
        #[arg(long)]
        models: bool,

        /// Show per-project breakdown
        #[arg(long)]
        projects: bool,
    },

    /// Write a default config file to ~/.blacklight/blacklight.toml
    Init,
}

fn main() {
    let cli = Cli::parse();

    // Load config before tracing init so log_level from config works.
    let cfg = match config::load_config(cli.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e:#}");
            std::process::exit(1);
        }
    };

    // Priority: RUST_LOG env > config file log_level > "info"
    let log_level = cfg.resolved_log_level().to_string();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
        )
        .init();

    match cli.command {
        Commands::Init => {
            run_init();
        }
        Commands::Index { full, ref source, verbose } => {
            run_index(&cli, &cfg, full, source.clone(), verbose);
        }
        Commands::Serve { port, no_open } => {
            run_serve(&cli, &cfg, port, no_open);
        }
        Commands::Enrich { limit, concurrency, force } => {
            run_enrich(&cli, &cfg, limit, concurrency, force);
        }
        Commands::Search { .. } => {
            eprintln!("blacklight search: not yet implemented");
        }
        Commands::Stats { .. } => {
            eprintln!("blacklight stats: not yet implemented");
        }
    }
}

fn run_init() {
    let path = config::default_config_path();
    if path.exists() {
        eprintln!("config file already exists: {}", path.display());
        std::process::exit(1);
    }
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("failed to create directory {}: {e}", parent.display());
            std::process::exit(1);
        }
    }
    let template = include_str!("config_template.toml");
    if let Err(e) = std::fs::write(&path, template) {
        eprintln!("failed to write config file: {e}");
        std::process::exit(1);
    }
    println!("wrote default config to {}", path.display());
}

fn resolve_db_path(cli: &Cli, cfg: &BlacklightConfig) -> PathBuf {
    cli.db
        .clone()
        .unwrap_or_else(|| cfg.resolved_db_path())
}

fn resolve_claude_dir(cli: &Cli, cfg: &BlacklightConfig) -> PathBuf {
    cli.claude_dir
        .clone()
        .unwrap_or_else(|| cfg.resolved_claude_dir())
}

fn run_index(
    cli: &Cli,
    cfg: &BlacklightConfig,
    full: bool,
    source: Option<PathBuf>,
    verbose: bool,
) {
    let claude_dir = source.unwrap_or_else(|| resolve_claude_dir(cli, cfg));
    let db_path = resolve_db_path(cli, cfg);
    let extra_dirs = blacklight::indexer::scanner::discover_extra_sources();
    let skip_dirs = cfg.indexer.skip_dirs.clone();
    let verbose = verbose || cfg.indexer.verbose;

    match indexer::run_index(indexer::IndexConfig {
        claude_dir,
        extra_dirs,
        db_path,
        full,
        verbose,
        skip_dirs,
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    }) {
        Ok(report) => print!("{report}"),
        Err(e) => {
            eprintln!("indexing failed: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run_serve(
    cli: &Cli,
    cfg: &BlacklightConfig,
    port: Option<u16>,
    no_open: bool,
) {
    let db_path = resolve_db_path(cli, cfg);
    let claude_dir = resolve_claude_dir(cli, cfg);
    let port = port.unwrap_or(cfg.server.port);
    let no_open = no_open || cfg.server.no_open;

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        if let Err(e) = server::start_server(&db_path, &claude_dir, port, no_open, cfg).await {
            eprintln!("server error: {e:#}");
            std::process::exit(1);
        }
    });
}

fn run_enrich(
    cli: &Cli,
    cfg: &BlacklightConfig,
    limit: Option<usize>,
    concurrency: Option<usize>,
    force: bool,
) {
    let db_path = resolve_db_path(cli, cfg);
    let concurrency = concurrency.unwrap_or(cfg.enrichment.concurrency);

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        match enrich::run_enrich(enrich::EnrichConfig {
            db_path,
            limit,
            concurrency,
            force,
            ollama_url: cfg.enrichment.ollama_url.clone(),
            ollama_model: cfg.enrichment.ollama_model.clone(),
            google_api_key: cfg.enrichment.google_api_key.clone(),
            auto_approve_threshold: cfg.enrichment.auto_approve_threshold,
            preferred_backend: cfg.enrichment.preferred_backend.clone(),
            progress: None,
            cancel_flag: None,
            log_lines: None,
        }).await {
            Ok(report) => print!("{report}"),
            Err(e) => {
                eprintln!("enrichment failed: {e:#}");
                std::process::exit(1);
            }
        }
    });
}
