//! Client side UDP/DTLS.
use super::session::Session;
use crate::udp::io::UdpIo;
use openssl::ssl::{Ssl, SslContext};
use std::io::Result;
use std::io::{Error as StdError, ErrorKind};
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::oneshot;
use tokio_openssl::SslStream;

/// A client capable of creating multiple UDP + DTLS sessions.
pub struct Client {
    io: Arc<UdpIo>,
    stop: Option<oneshot::Sender<()>>,
}

impl Client {
    /// Create a new client instance with a bound UdpSocket.
    pub fn new(socket: UdpSocket) -> Self {
        let (stop, stop_rx) = oneshot::channel();
        let io = Arc::new(UdpIo::new(socket));
        let runner = io.clone();
        tokio::spawn(async move { runner.run(stop_rx, None).await });
        Self {
            io,
            stop: Some(stop),
        }
    }

    /// Create a stream for a peer and perform DTLS handshake if context is provided.
    pub async fn connect<S: ToSocketAddrs>(
        &self,
        peer: S,
        tls: Option<SslContext>,
    ) -> Result<Session> {
        let mut bind = peer.to_socket_addrs()?;
        if let Some(peer) = bind.next() {
            let r = self.io.connect(peer).await?;
            if let Some(ctx) = &tls {
                let mut dtls = SslStream::new(Ssl::new(&ctx)?, r)?;
                Pin::new(&mut dtls).connect().await.map_err(|_| {
                    StdError::new(ErrorKind::ConnectionReset, "Error during TLS handshake")
                })?;
                Ok(Session::new_dtls(dtls, peer))
            } else {
                Ok(Session::new_udp(r, peer))
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "Peer address not available",
            ))
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
