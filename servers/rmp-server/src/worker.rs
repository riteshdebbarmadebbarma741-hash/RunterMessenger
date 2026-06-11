// servers/rmp-server/src/worker.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use crate::fencing::FencingToken;
use crate::idempotency::IdempotencyRegistry;
use crate::retry::RetryManager;
use crate::dlq::DeadLetterQueue;
use crate::metrics::RmpMetrics;
use runter_database::wal::WalEntry;
use runter_database::materializer::Materializer;
use runter_database::Database;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ApplyResult {
    pub index: u64,
    pub success: bool,
}

pub struct Worker {
    pub id: usize,
    pub rx: mpsc::UnboundedReceiver<(FencingToken, WalEntry)>,
    pub result_tx: mpsc::UnboundedSender<ApplyResult>,
    pub materializer: Arc<Materializer>,
    pub idempotency: Arc<IdempotencyRegistry>,
    pub retry_manager: Arc<RetryManager>,
    pub dlq: Arc<DeadLetterQueue>,
    pub db: Arc<Database>,
    pub config: RmpConfig,
    pub metrics: Arc<RmpMetrics>,
}

impl Worker {
    pub async fn run(mut self) {
        while let Some((token, entry)) = self.rx.recv().await {
            if !self.idempotency.try_claim(&entry.message_id, token.epoch).unwrap_or(false) {
                continue;
            }

            let conn = self.db.materializer.sqlite.get().unwrap();
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
            tx.execute_batch(
                "INSERT OR IGNORE INTO applied_log (index_val, epoch, message_id) VALUES (?1, ?2, ?3)",
            ).unwrap();
            tx.execute(
                "INSERT OR IGNORE INTO applied_log (index_val, epoch, message_id) VALUES (?1, ?2, ?3)",
                rusqlite::params![entry.index as i64, token.epoch as i64, entry.message_id],
            ).unwrap();

            let applied: bool = tx.query_row(
                "SELECT index_val FROM applied_log WHERE index_val = ?1",
                rusqlite::params![entry.index as i64],
                |_| Ok(true),
            ).unwrap_or(false);

            if applied {
                match self.materializer.apply(&entry) {
                    Ok(()) => {
                        tx.commit().unwrap();
                        let _ = self.result_tx.send(ApplyResult { index: entry.index, success: true });
                    }
                    Err(e) => {
                        tx.rollback().unwrap();
                        let _ = self.result_tx.send(ApplyResult { index: entry.index, success: false });
                        self.metrics.worker_errors.inc();
                    }
                }
            }

            self.metrics.worker_processed.inc();
        }
    }
}