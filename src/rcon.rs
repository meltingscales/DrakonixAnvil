//! Simple RCON (Remote Console) client for Minecraft servers
//!
//! Protocol: https://wiki.vg/RCON
//!
//! Packet structure:
//! - 4 bytes: length (little-endian, excludes these 4 bytes)
//! - 4 bytes: request ID (little-endian)
//! - 4 bytes: packet type (little-endian)
//! - N bytes: payload (null-terminated string)
//! - 2 bytes: padding (two null bytes)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Packet types
const SERVERDATA_AUTH: i32 = 3;
#[allow(dead_code)]
const SERVERDATA_AUTH_RESPONSE: i32 = 2; // Same as EXECCOMMAND, used for documentation
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_RESPONSE_VALUE: i32 = 0;

/// RCON connection to a Minecraft server
pub struct RconClient {
    stream: TcpStream,
    request_id: i32,
}

#[derive(Debug)]
pub enum RconError {
    ConnectionFailed(String),
    AuthFailed,
    SendFailed(String),
    ReceiveFailed(String),
    InvalidResponse(String),
    Timeout,
}

impl std::fmt::Display for RconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RconError::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            RconError::AuthFailed => write!(f, "Authentication failed (wrong password?)"),
            RconError::SendFailed(e) => write!(f, "Failed to send: {}", e),
            RconError::ReceiveFailed(e) => write!(f, "Failed to receive: {}", e),
            RconError::InvalidResponse(e) => write!(f, "Invalid response: {}", e),
            RconError::Timeout => write!(f, "Connection timed out"),
        }
    }
}

impl RconClient {
    /// Connect to an RCON server and authenticate
    pub fn connect(address: &str, password: &str) -> Result<Self, RconError> {
        tracing::debug!("RCON: Connecting to {}", address);

        // Parse address
        let addr: std::net::SocketAddr = address
            .parse()
            .map_err(|e| RconError::ConnectionFailed(format!("Invalid address: {}", e)))?;

        // Connect with timeout
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|e| {
            tracing::error!("RCON: Connection failed: {}", e);
            RconError::ConnectionFailed(e.to_string())
        })?;

        // Set timeouts
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| {
                RconError::ConnectionFailed(format!("Failed to set read timeout: {}", e))
            })?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| {
                RconError::ConnectionFailed(format!("Failed to set write timeout: {}", e))
            })?;

        tracing::debug!("RCON: Connected, authenticating...");

        let mut client = Self {
            stream,
            request_id: 1,
        };

        // Authenticate
        client.authenticate(password)?;

        tracing::info!("RCON: Authenticated successfully");
        Ok(client)
    }

    /// Authenticate with the server
    fn authenticate(&mut self, password: &str) -> Result<(), RconError> {
        let auth_id = self.request_id;
        self.send_packet(SERVERDATA_AUTH, password)?;

        // Read auth response
        let (resp_id, resp_type, _payload) = self.receive_packet()?;

        tracing::debug!("RCON: Auth response - id: {}, type: {}", resp_id, resp_type);

        // Auth failure returns request_id = -1
        if resp_id == -1 {
            tracing::error!("RCON: Authentication failed");
            return Err(RconError::AuthFailed);
        }

        // Some servers send an empty RESPONSE_VALUE before the AUTH_RESPONSE
        if resp_type == SERVERDATA_RESPONSE_VALUE {
            // Read the actual auth response
            let (resp_id2, _resp_type2, _payload2) = self.receive_packet()?;
            if resp_id2 == -1 {
                return Err(RconError::AuthFailed);
            }
        }

        if resp_id != auth_id {
            tracing::warn!(
                "RCON: Unexpected auth response ID: {} (expected {})",
                resp_id,
                auth_id
            );
        }

        Ok(())
    }

    /// Send a command and get the response
    pub fn command(&mut self, cmd: &str) -> Result<String, RconError> {
        tracing::debug!("RCON: Sending command: {}", cmd);

        self.send_packet(SERVERDATA_EXECCOMMAND, cmd)?;

        let (resp_id, resp_type, payload) = self.receive_packet()?;

        tracing::debug!(
            "RCON: Response - id: {}, type: {}, len: {}",
            resp_id,
            resp_type,
            payload.len()
        );

        if resp_type != SERVERDATA_RESPONSE_VALUE {
            tracing::warn!(
                "RCON: Unexpected response type: {} (expected {})",
                resp_type,
                SERVERDATA_RESPONSE_VALUE
            );
        }

        Ok(payload)
    }

    /// Send a packet to the server
    fn send_packet(&mut self, packet_type: i32, payload: &str) -> Result<(), RconError> {
        let request_id = self.request_id;
        self.request_id += 1;

        let payload_bytes = payload.as_bytes();
        // Length = request_id(4) + type(4) + payload + null(1) + padding(1)
        let length = 4 + 4 + payload_bytes.len() + 1 + 1;

        let mut packet = Vec::with_capacity(4 + length);

        // Length (little-endian)
        packet.extend_from_slice(&(length as i32).to_le_bytes());
        // Request ID (little-endian)
        packet.extend_from_slice(&request_id.to_le_bytes());
        // Type (little-endian)
        packet.extend_from_slice(&packet_type.to_le_bytes());
        // Payload
        packet.extend_from_slice(payload_bytes);
        // Null terminator
        packet.push(0);
        // Padding
        packet.push(0);

        tracing::trace!("RCON: Sending {} bytes", packet.len());

        self.stream.write_all(&packet).map_err(|e| {
            tracing::error!("RCON: Send failed: {}", e);
            RconError::SendFailed(e.to_string())
        })?;

        self.stream
            .flush()
            .map_err(|e| RconError::SendFailed(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Receive a packet from the server
    fn receive_packet(&mut self) -> Result<(i32, i32, String), RconError> {
        // Read length (4 bytes)
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).map_err(|e| {
            tracing::error!("RCON: Failed to read length: {}", e);
            if e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::WouldBlock
            {
                RconError::Timeout
            } else {
                RconError::ReceiveFailed(format!("Failed to read length: {}", e))
            }
        })?;

        let length = i32::from_le_bytes(len_buf) as usize;
        tracing::trace!("RCON: Receiving packet of {} bytes", length);

        if length < 10 {
            return Err(RconError::InvalidResponse(format!(
                "Packet too small: {}",
                length
            )));
        }
        if length > 4096 {
            return Err(RconError::InvalidResponse(format!(
                "Packet too large: {}",
                length
            )));
        }

        // Read rest of packet
        let mut buf = vec![0u8; length];
        self.stream.read_exact(&mut buf).map_err(|e| {
            tracing::error!("RCON: Failed to read packet body: {}", e);
            RconError::ReceiveFailed(format!("Failed to read body: {}", e))
        })?;

        // Parse packet
        let request_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let packet_type = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);

        // Payload is everything after the type, minus the 2-byte padding
        let payload_end = length - 2;
        let payload_bytes = &buf[8..payload_end];

        // Convert to string, stripping null terminator if present
        let payload = String::from_utf8_lossy(payload_bytes)
            .trim_end_matches('\0')
            .to_string();

        Ok((request_id, packet_type, payload))
    }
}
