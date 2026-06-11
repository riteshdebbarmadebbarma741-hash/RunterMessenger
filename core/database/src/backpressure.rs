// core/database/src/backpressure.rs
use crate::Database;
use crate::error::DatabaseError;

pub fn check(db: &Database, queue_id: &[u8]) -> Result<(), DatabaseError> {
    let conn = db.materializer.sqlite.get()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM messages WHERE queue_id = ?1 AND acked = 0",
        rusqlite::params![queue_id],
        |row| row.get(0),
    )?;
    if count as u64 >= db.config.max_queue_capacity {
        db.metrics.backpressure_hits.inc();
        return Err(DatabaseError::Backpressure(format!("Queue {} at capacity", hex::encode(queue_id))));
    }
    Ok(())
}