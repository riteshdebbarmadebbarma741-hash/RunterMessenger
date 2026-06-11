// core/protocol/src/protocol.rs
use crate::error::ProtocolError;
use crate::types::{Frame, Command, AuthRequest};
use crate::transport::Transport;
use crate::session::Session;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};

pub async fn send_frame(transport: &mut Transport, frame: &Frame) -> Result<(), ProtocolError> {
    transport.send(frame).await
}

pub async fn receive_frame(transport: &mut Transport) -> Result<Frame, ProtocolError> {
    transport.receive().await
}

pub async fn handshake(transport: &mut Transport) -> Result<Vec<u8>, ProtocolError> {
    let ping = Frame::new(Command::Ping, vec![]);
    transport.send_with_timeout(&ping, 5).await?;
    let response = transport.receive_with_timeout(5).await?;
    if !matches!(response.command, Command::Pong) {
        return Err(ProtocolError::InvalidFrame("Handshake failed: expected Pong".into()));
    }
    if response.request_nonce.as_deref() != Some(&ping.nonce) {
        return Err(ProtocolError::NonceMismatch);
    }
    if response.nonce.is_empty() {
        return Err(ProtocolError::InvalidFrame("Server nonce empty".into()));
    }
    Ok(response.nonce)
}

pub async fn server_handshake(
    session: &mut Session,
    request: &Frame,
) -> Result<Frame, ProtocolError> {
    let server_nonce = session.issue_nonce();
    Ok(Frame::response_to(request, Command::Pong, server_nonce))
}

pub async fn authenticate(
    transport: &mut Transport,
    identity_key: &[u8],
    signature: &[u8],
    server_nonce: &[u8],
) -> Result<(), ProtocolError> {
    let auth_req = AuthRequest {
        identity_key: identity_key.to_vec(),
        signature: signature.to_vec(),
        server_nonce: server_nonce.to_vec(),
    };
    let frame = Frame::new(
        Command::Auth,
        bincode::serialize(&auth_req)
            .map_err(|e| ProtocolError::EncodingFailed(e.to_string()))?,
    );
    transport.send(&frame).await?;
    let response = transport.receive().await?;
    if response.request_nonce.as_deref() != Some(&frame.nonce) {
        return Err(ProtocolError::NonceMismatch);
    }
    match response.command {
        Command::AuthResponse => Ok(()),
        Command::Error => {
            let err: String = bincode::deserialize(&response.payload).unwrap_or_default();
            Err(ProtocolError::AuthenticationFailed(err))
        }
        _ => Err(ProtocolError::UnexpectedCommand(format!("{:?}", response.command))),
    }
}

pub async fn server_authenticate(
    session: &mut Session,
    request: &Frame,
) -> Result<Frame, ProtocolError> {
    let req: AuthRequest = bincode::deserialize(&request.payload)
        .map_err(|e| ProtocolError::DecodingFailed(e.to_string()))?;
    if req.identity_key.len() != 32 {
        return Err(ProtocolError::AuthenticationFailed("Invalid identity key length".into()));
    }
    if req.signature.len() != 64 {
        return Err(ProtocolError::AuthenticationFailed("Invalid signature length".into()));
    }
    if !session.verify_and_consume_nonce(&req.server_nonce) {
        return Err(ProtocolError::AuthenticationFailed("Invalid or reused server nonce".into()));
    }
    let identity_key_bytes: [u8; 32] = req.identity_key[..32].try_into().unwrap();
    let signature_bytes: [u8; 64] = req.signature[..64].try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&identity_key_bytes)
        .map_err(|_| ProtocolError::AuthenticationFailed("Invalid identity key".into()))?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key.verify_strict(&req.server_nonce, &signature)
        .map_err(|_| ProtocolError::AuthenticationFailed("Signature verification failed".into()))?;
    session.identity_key = req.identity_key;
    session.authenticated = true;
    Ok(Frame::response_to(request, Command::AuthResponse, vec![]))
}

pub fn validate_queue_name(name: &str) -> Result<(), ProtocolError> {
    if name.is_empty() || name.len() > crate::constants::MAX_QUEUE_NAME_LENGTH {
        return Err(ProtocolError::InvalidQueueName(name.to_string()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(ProtocolError::InvalidQueueName(name.to_string()));
    }
    if name.starts_with("system_") || name.starts_with("admin_") || name.starts_with("__") {
        return Err(ProtocolError::InvalidQueueName("Reserved name prefix".into()));
    }
    Ok(())
}