use blacklight::{indexer, server};
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
        #[arg(long, default_value = "3141")]
        port: u16,

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
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index { full, source, verbose } => {
            let claude_dir = source.or(cli.claude_dir)
                .unwrap_or_else(|| dirs::home_dir().unwrap().join(".claude"));
            let db_path = cli.db.unwrap_or_else(blacklight::db::default_db_path);
            let extra_dirs = blacklight::indexer::scanner::discover_extra_sources();
            match indexer::run_index(indexer::IndexConfig {
                claude_dir,
                extra_dirs,
                db_path,
                full,
                verbose,
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
        Commands::Serve { port, no_open } => {
            let db_path = cli.db.unwrap_or_else(blacklight::db::default_db_path);
            let claude_dir = cli.claude_dir
                .unwrap_or_else(|| dirs::home_dir().unwrap().join(".claude"));
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(async {
                if let Err(e) = server::start_server(&db_path, &claude_dir, port, no_open).await {
                    eprintln!("server error: {e:#}");
                    std::process::exit(1);
                }
            });
        }
        Commands::Search { .. } => {
            eprintln!("blacklight search: not yet implemented");
        }
        Commands::Stats { .. } => {
            eprintln!("blacklight stats: not yet implemented");
        }
    }
}
