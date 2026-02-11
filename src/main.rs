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
        Commands::Index { .. } => {
            eprintln!("blacklight index: not yet implemented");
        }
        Commands::Serve { .. } => {
            eprintln!("blacklight serve: not yet implemented");
        }
        Commands::Search { .. } => {
            eprintln!("blacklight search: not yet implemented");
        }
        Commands::Stats { .. } => {
            eprintln!("blacklight stats: not yet implemented");
        }
    }
}
