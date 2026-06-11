// core/database/src/wal.rs
use crate::config::DatabaseConfig;
use crate::error::DatabaseError;
use crate::metrics::DatabaseMetrics;
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write, BufReader, Read, Seek, SeekFrom};
use std::sync::Arc;
use parking_lot::RwLock;

pub const WAL_MAGIC: u32 = 0x52554E54;
pub const WAL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub index: u64,
    pub entry_type: WalEntryType,
    pub queue_id: Vec<u8>,
    pub sequence_id: i64,
    pub message_id: Vec<u8>,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub ttl: Option<u64>,
    pub expires_at: Option<u64>,
    pub crc: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalEntryType {
    MessageInsert,
    MessageAck,
    QueueCreate,
    QueueDelete,
}

pub struct WriteAheadLog {
    writer: RwLock<BufWriter<File>>,
    reader: RwLock<BufReader<File>>,
    applied_index: RwLock<u64>,
    config: DatabaseConfig,
    metrics: Arc<DatabaseMetrics>,
}

impl WriteAheadLog {
    pub fn open(config: &DatabaseConfig, metrics: &Arc<DatabaseMetrics>) -> Result<Self, DatabaseError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&config.wal_path)?;

        let file_len = file.metadata()?.len();
        let reader = BufReader::new(file.try_clone()?);
        let writer = BufWriter::new(file);

        let wal = WriteAheadLog {
            writer: RwLock::new(writer),
            reader: RwLock::new(reader),
            applied_index: RwLock::new(0),
            config: config.clone(),
            metrics: metrics.clone(),
        };

        if file_len == 0 {
            wal.write_header()?;
        } else {
            wal.validate_header()?;
        }

        Ok(wal)
    }

    fn write_header(&self) -> Result<(), DatabaseError> {
        let mut writer = self.writer.write();
        writer.write_all(&WAL_MAGIC.to_le_bytes())?;
        writer.write_all(&WAL_VERSION.to_le_bytes())?;
        writer.flush()?;
        Ok(())
    }

    fn validate_header(&self) -> Result<(), DatabaseError> {
        let mut reader = self.reader.write();
        reader.seek(SeekFrom::Start(0))?;
        let mut magic_buf = [0u8; 4];
        reader.read_exact(&mut magic_buf)?;
        let magic = u32::from_le_bytes(magic_buf);
        if magic != WAL_MAGIC {
            return Err(DatabaseError::Wal("Invalid WAL magic".into()));
        }
        let mut version_buf = [0u8; 4];
        reader.read_exact(&mut version_buf)?;
        Ok(())
    }

    pub fn append(&self, entry: &mut WalEntry) -> Result<u64, DatabaseError> {
        let mut writer = self.writer.write();
        let offset = writer.seek(SeekFrom::End(0))?;
        let index = offset / 128;
        entry.index = index;
        let serialized = bincode::serialize(entry)?;
        entry.crc = crc32fast::hash(&serialized);
        let final_data = bincode::serialize(entry)?;
        let len = final_data.len() as u32;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&final_data)?;
        writer.flush()?;
        self.metrics.wal_bytes_written.inc_by((4 + len) as u64);
        self.metrics.wal_entries_written.inc();
        Ok(index)
    }

    pub fn append_batch(&self, entries: &mut [WalEntry]) -> Result<u64, DatabaseError> {
        let mut writer = self.writer.write();
        let mut last_index = 0;
        let mut buffer = Vec::with_capacity(entries.len() * 256);
        for entry in entries.iter_mut() {
            let offset = writer.seek(SeekFrom::End(0))? + buffer.len() as u64;
            let index = offset / 128;
            entry.index = index;
            let serialized = bincode::serialize(entry)?;
            entry.crc = crc32fast::hash(&serialized);
            let final_data = bincode::serialize(entry)?;
            let len = final_data.len() as u32;
            buffer.extend_from_slice(&len.to_le_bytes());
            buffer.extend_from_slice(&final_data);
            last_index = index;
        }
        writer.write_all(&buffer)?;
        writer.flush()?;
        self.metrics.wal_bytes_written.inc_by(buffer.len() as u64);
        self.metrics.wal_entries_written.inc_by(entries.len() as u64);
        Ok(last_index)
    }

    pub fn read_from(&self, start_index: u64) -> Result<Vec<WalEntry>, DatabaseError> {
        let mut reader = self.reader.write();
        reader.seek(SeekFrom::Start(8))?;
        let mut entries = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut data = vec![0u8; len];
            reader.read_exact(&mut data)?;
            let entry: WalEntry = bincode::deserialize(&data)?;
            if entry.index >= start_index {
                let verify_data = bincode::serialize(&entry)?;
                let verify_crc = crc32fast::hash(&verify_data);
                if verify_crc != entry.crc {
                    self.metrics.wal_crc_errors.inc();
                    return Err(DatabaseError::CrcMismatch { index: entry.index });
                }
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn mark_applied(&self, index: &u64) -> Result<(), DatabaseError> {
        let mut applied = self.applied_index.write();
        *applied = (*applied).max(*index);
        Ok(())
    }

    pub fn get_applied_index(&self) -> u64 {
        *self.applied_index.read()
    }
}