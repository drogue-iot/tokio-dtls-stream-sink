use crate::packet_stream::PacketFramed;
use crate::udp::stream::UdpStream;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use openssl::ssl::SslRef;
use std::io::{Error as StdError, ErrorKind, Result};
use tokio_openssl::SslStream;
use tokio_util::codec::{BytesCodec, Decoder, Framed};

/// A UDP + DTLS session.
pub struct Session {
    inner: Inner,
}

impl Session {
    pub(crate) fn new_udp(stream: UdpStream) -> Self {
        let inner = Inner::UDP(BytesCodec::new().framed(stream));
        Self { inner }
    }

    pub(crate) fn new_dtls(stream: SslStream<UdpStream>) -> Self {
        let inner = Inner::DTLS(BytesCodec::new().framed(stream));
        Self { inner }
    }

    /// Get a reference to the SSL context, if present
    pub fn ssl(&self) -> Option<&SslRef> {
        match &self.inner {
            Inner::UDP(_) => None,
            Inner::DTLS(s) => Some(s.get_ref().ssl()),
        }
    }

    /// Turn into the underlying stream/sink.
    pub fn stream_sink(&mut self) -> &mut dyn PacketFramed {
        match &mut self.inner {
            Inner::UDP(inner) => inner,
            Inner::DTLS(inner) => inner,
        }
    }

    /// Write the provided buffer as a datagram over DTLS.
    pub async fn write(&mut self, buf: &[u8]) -> Result<()> {
        match &mut self.inner {
            Inner::UDP(inner) => inner.send(Bytes::copy_from_slice(buf)).await,
            Inner::DTLS(inner) => inner.send(Bytes::copy_from_slice(buf)).await,
        }
    }

    /// Read a datagram and copy it into buffer.
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let rx = match &mut self.inner {
            Inner::UDP(inner) => inner.next().await,
            Inner::DTLS(inner) => inner.next().await,
        }
        .ok_or(StdError::new(ErrorKind::ConnectionReset, "Stream closed"))??;
        let to_copy = core::cmp::min(buf.len(), rx.len());
        buf[..to_copy].copy_from_slice(&rx[..to_copy]);
        Ok(to_copy)
    }
}

enum Inner {
    DTLS(Framed<SslStream<UdpStream>, BytesCodec>),
    UDP(Framed<UdpStream, BytesCodec>),
}
