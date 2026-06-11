// core/database/src/metrics.rs
use crate::error::DatabaseError;
use prometheus::{Counter, Histogram, IntGauge, Registry, Encoder, TextEncoder};
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::default();
}

#[derive(Clone)]
pub struct DatabaseMetrics {
    pub wal_entries_written: Counter,
    pub wal_bytes_written: Counter,
    pub wal_crc_errors: Counter,
    pub materialized_messages: Counter,
    pub materialized_acks: Counter,
    pub recovery_duration_seconds: Histogram,
    pub recovery_entries: IntGauge,
    pub backpressure_hits: Counter,
}

impl DatabaseMetrics {
    pub fn register() -> Result<Self, DatabaseError> {
        let wal_entries_written = Counter::new("runter_wal_entries_written_total", "WAL entries written")?;
        let wal_bytes_written = Counter::new("runter_wal_bytes_written_total", "WAL bytes written")?;
        let wal_crc_errors = Counter::new("runter_wal_crc_errors_total", "WAL CRC errors")?;
        let materialized_messages = Counter::new("runter_materialized_messages_total", "Materialized messages")?;
        let materialized_acks = Counter::new("runter_materialized_acks_total", "Materialized acks")?;
        let recovery_duration_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new("runter_recovery_duration_seconds", "Recovery duration")
        )?;
        let recovery_entries = IntGauge::new("runter_recovery_entries", "Recovery entries")?;
        let backpressure_hits = Counter::new("runter_backpressure_hits_total", "Backpressure hits")?;

        REGISTRY.register(Box::new(wal_entries_written.clone()))?;
        REGISTRY.register(Box::new(wal_bytes_written.clone()))?;
        REGISTRY.register(Box::new(wal_crc_errors.clone()))?;
        REGISTRY.register(Box::new(materialized_messages.clone()))?;
        REGISTRY.register(Box::new(materialized_acks.clone()))?;
        REGISTRY.register(Box::new(recovery_duration_seconds.clone()))?;
        REGISTRY.register(Box::new(recovery_entries.clone()))?;
        REGISTRY.register(Box::new(backpressure_hits.clone()))?;

        Ok(DatabaseMetrics {
            wal_entries_written,
            wal_bytes_written,
            wal_crc_errors,
            materialized_messages,
            materialized_acks,
            recovery_duration_seconds,
            recovery_entries,
            backpressure_hits,
        })
    }

    pub fn export(&self) -> Result<String, DatabaseError> {
        let encoder = TextEncoder::new();
        let metric_families = REGISTRY.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}