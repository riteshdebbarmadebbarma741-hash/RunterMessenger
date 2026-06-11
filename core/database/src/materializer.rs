// core/database/src/materializer.rs
use crate::config::DatabaseConfig;
use crate::error::DatabaseError;
use crate::metrics::DatabaseMetrics;
use crate::wal::WalEntry;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rocksdb::{Options, DB};
use std::sync::Arc;

pub struct Materializer {
    pub sqlite: Pool<SqliteConnectionManager>,
    pub rocks: Arc<DB>,
    metrics: Arc<DatabaseMetrics>,
}

impl Materializer {
    pub fn open(config: &DatabaseConfig, metrics: &Arc<DatabaseMetrics>) -> Result<Self, DatabaseError> {
        let manager = SqliteConnectionManager::file(&config.sqlite_path);
        let sqlite = Pool::builder().max_size(16).build(manager)?;

        let mut opts = Options::default();
        opts.create_if_missing(true);
        let rocks = Arc::new(DB::open(&opts, &config.rocks_path)?);

        {
            let conn = sqlite.get()?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS messages (
                    id BLOB PRIMARY KEY,
                    queue_id BLOB NOT NULL,
                    sequence_id INTEGER NOT NULL,
                    timestamp INTEGER NOT NULL,
                    ttl INTEGER,
                    acked INTEGER NOT NULL DEFAULT 0,
                    expires_at INTEGER
                );
                CREATE INDEX IF NOT EXISTS idx_queue_seq ON messages(queue_id, sequence_id);
                CREATE INDEX IF NOT EXISTS idx_queue_acked ON messages(queue_id, acked, sequence_id);
                CREATE TABLE IF NOT EXISTS applied_index (index_val INTEGER PRIMARY KEY);"
            )?;
        }

        Ok(Materializer { sqlite, rocks, metrics: metrics.clone() })
    }

    pub fn apply(&self, entry: &WalEntry) -> Result<(), DatabaseError> {
        match entry.entry_type {
            crate::wal::WalEntryType::MessageInsert => {
                let conn = self.sqlite.get()?;
                conn.execute(
                    "INSERT OR IGNORE INTO messages (id, queue_id, sequence_id, timestamp, ttl, acked, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
                    rusqlite::params![entry.message_id, entry.queue_id, entry.sequence_id, entry.timestamp, entry.ttl, entry.expires_at],
                )?;
                self.rocks.put(&build_materialized_key(&entry.queue_id, entry.sequence_id), &entry.payload)?;
                self.metrics.materialized_messages.inc();
            }
            crate::wal::WalEntryType::MessageAck => {
                let conn = self.sqlite.get()?;
                conn.execute("UPDATE messages SET acked = 1 WHERE id = ?1", rusqlite::params![entry.message_id])?;
                self.rocks.delete(&build_materialized_key(&entry.queue_id, entry.sequence_id))?;
                self.metrics.materialized_acks.inc();
            }
            _ => {}
        }
        let conn = self.sqlite.get()?;
        conn.execute("INSERT OR REPLACE INTO applied_index (index_val) VALUES (?1)", rusqlite::params![entry.index as i64])?;
        Ok(())
    }

    pub fn get_last_applied_index(&self) -> Result<u64, DatabaseError> {
        let conn = self.sqlite.get()?;
        Ok(conn.query_row("SELECT COALESCE(MAX(index_val), 0) FROM applied_index", [], |row| row.get::<_, i64>(0))? as u64)
    }
}

fn build_materialized_key(queue_id: &[u8], sequence_id: i64) -> Vec<u8> {
    let mut key = Vec::with_capacity(queue_id.len() + 8);
    key.extend_from_slice(queue_id);
    key.extend_from_slice(&sequence_id.to_be_bytes());
    key
}