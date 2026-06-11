// servers/rmp-server/src/consumer.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use crate::fencing::FencingToken;
use crate::offset_index::OffsetIndex;
use crate::partition::PartitionRouter;
use crate::metrics::RmpMetrics;
use runter_database::wal::WalEntry;
use runter_database::wal::WriteAheadLog;
use runter_database::materializer::Materializer;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct PartitionConsumer {
    pub partition_id: usize,
    pub token: FencingToken,
    pub wal: Arc<WriteAheadLog>,
    pub offset_index: Arc<OffsetIndex>,
    pub materializer: Arc<Materializer>,
    pub router: Arc<PartitionRouter>,
    pub dispatch_tx: mpsc::UnboundedSender<(FencingToken, WalEntry)>,
    pub config: RmpConfig,
    pub metrics: Arc<RmpMetrics>,
}

impl PartitionConsumer {
    pub async fn run(&self) -> Result<(), RmpError> {
        let mut next_index = self.materializer.get_last_applied_index()?
            .max(self.offset_index.get_last_index()?);
        next_index += 1;

        loop {
            let entries = self.wal.read_from(next_index)?;
            if entries.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.config.consumer_poll_interval_ms)).await;
                continue;
            }

            let mut count = 0;
            for entry in entries {
                if self.router.route(&entry.message_id) != self.partition_id {
                    continue;
                }
                if self.dispatch_tx.send((self.token.clone(), entry)).is_err() {
                    return Err(RmpError::Shutdown);
                }
                count += 1;
                if count >= self.config.consumer_batch_size {
                    break;
                }
            }

            next_index += self.config.consumer_batch_size as u64;
            self.metrics.consumer_batches.inc();
            self.metrics.consumer_entries.inc_by(count as u64);
        }
    }
}