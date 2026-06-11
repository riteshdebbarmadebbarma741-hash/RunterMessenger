// core/protocol/src/transport.rs
use crate::constants::MAX_FRAME_SIZE;
use crate::encoding::{encode_frame, decode_frame};
use crate::error::ProtocolError;
use crate::types::Frame;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

pub struct Transport {
    stream: TcpStream,
}

impl Transport {
    pub fn new(stream: TcpStream) -> Self {
        Transport { stream }
    }

    pub async fn send(&mut self, frame: &Frame) -> Result<(), ProtocolError> {
        let data = encode_frame(frame)?;
        timeout(
            Duration::from_secs(crate::constants::TRANSPORT_TIMEOUT_SECS),
            self.stream.write_all(&data),
        )
        .await
        .map_err(|_| ProtocolError::Timeout)?
        .map_err(|e| ProtocolError::TransportError(e.to_string()))
    }

    pub async fn receive(&mut self) -> Result<Frame, ProtocolError> {
        let mut len_buf = [0u8; 4];
        match timeout(
            Duration::from_secs(crate::constants::TRANSPORT_TIMEOUT_SECS),
            self.stream.read_exact(&mut len_buf),
        )
        .await
        {
            Err(_) => {
                let _ = self.stream.shutdown().await;
                return Err(ProtocolError::Timeout);
            }
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(ProtocolError::ConnectionClosed);
            }
            Ok(Err(e)) => return Err(ProtocolError::TransportError(e.to_string())),
            Ok(Ok(_)) => {}
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(ProtocolError::InvalidFrame(format!("Frame size {} exceeds max", len)));
        }
        let mut data = vec![0u8; len];
        match timeout(
            Duration::from_secs(crate::constants::TRANSPORT_TIMEOUT_SECS),
            self.stream.read_exact(&mut data),
        )
        .await
        {
            Err(_) => {
                let _ = self.stream.shutdown().await;
                return Err(ProtocolError::Timeout);
            }
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(ProtocolError::ConnectionClosed);
            }
            Ok(Err(e)) => return Err(ProtocolError::TransportError(e.to_string())),
            Ok(Ok(_)) => {}
        }
        decode_frame(&data)
    }

    pub async fn send_with_timeout(&mut self, frame: &Frame, timeout_secs: u64) -> Result<(), ProtocolError> {
        timeout(Duration::from_secs(timeout_secs), self.send(frame))
            .await
            .map_err(|_| ProtocolError::Timeout)?
    }

    pub async fn receive_with_timeout(&mut self, timeout_secs: u64) -> Result<Frame, ProtocolError> {
        timeout(Duration::from_secs(timeout_secs), self.receive())
            .await
            .map_err(|_| ProtocolError::Timeout)?
    }
}