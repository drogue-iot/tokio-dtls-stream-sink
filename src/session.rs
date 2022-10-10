use crate::packet_stream::PacketFramed;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use openssl::x509::X509;
use std::boxed::Box;
use std::io::{Error as StdError, ErrorKind, Result};

/// A UDP + DTLS session.
pub struct Session {
    stream_sink: Box<dyn PacketFramed>,
    peer_certificate: Option<X509>,
}

impl Session {
    /// Create a new session instance
    pub fn new(stream_sink: Box<dyn PacketFramed>, peer_certificate: Option<X509>) -> Self {
        Self {
            stream_sink,
            peer_certificate,
        }
    }

    /// Retrieve the DTLS peer certificate if available.
    pub fn peer_certificate(&self) -> &Option<X509> {
        &self.peer_certificate
    }

    /// Turn into the underlying stream/sink.
    pub fn into_stream_sink(self) -> Box<dyn PacketFramed> {
        self.stream_sink
    }

    /// Write the provided buffer as a datagram over DTLS.
    pub async fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.stream_sink.send(Bytes::copy_from_slice(buf)).await
    }

    /// Read a datagram and copy it into buffer.
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let rx = self
            .stream_sink
            .next()
            .await
            .ok_or(StdError::new(ErrorKind::ConnectionReset, "Stream closed"))??;

        let to_copy = core::cmp::min(buf.len(), rx.len());
        buf[..to_copy].copy_from_slice(&rx[..to_copy]);
        Ok(to_copy)
    }
}
