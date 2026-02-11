use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A pool of SQLite connections for use with tokio::spawn_blocking.
/// rusqlite Connection is !Send, so we hold them behind a Mutex and
/// move them into blocking tasks.
pub struct DbPool {
    connections: Mutex<Vec<Connection>>,
    db_path: PathBuf,
    pool_size: usize,
}

impl DbPool {
    pub fn new(db_path: &Path, pool_size: usize) -> Result<Self> {
        let mut connections = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            connections.push(crate::db::open(db_path)?);
        }
        Ok(Self {
            connections: Mutex::new(connections),
            db_path: db_path.to_path_buf(),
            pool_size,
        })
    }

    /// Execute a closure with a connection from the pool inside spawn_blocking.
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.checkout()?;
        let result = tokio::task::spawn_blocking(move || f(&conn))
            .await
            .context("spawn_blocking join error")?;
        // Note: connection is dropped after spawn_blocking completes.
        // For a production pool we'd return it, but opening is cheap with WAL.
        result
    }

    fn checkout(&self) -> Result<Connection> {
        let mut pool = self.connections.lock().unwrap();
        if let Some(conn) = pool.pop() {
            Ok(conn)
        } else {
            // Pool exhausted — open a new connection
            crate::db::open(&self.db_path)
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    #[allow(dead_code)]
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }
}

/// Shared application state passed to all axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: std::sync::Arc<DbPool>,
    pub source_dir: PathBuf,
}
