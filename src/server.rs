//! Server side UDP/DTLS.
use crate::session::Session;
use crate::udp::io::UdpIo;
use crate::udp::stream::UdpStream;
use core::pin::Pin;
use openssl::ssl::{Ssl, SslContext};
use std::io::{Error as StdError, ErrorKind, Result};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio_openssl::SslStream;

/// Instance of a UDP (+ DTLS) server that can accept new connections in form of
/// of a packet stream/sink.
pub struct Server {
    stop: Option<oneshot::Sender<()>>,
    accept_rx: mpsc::Receiver<Result<UdpStream>>,
}

impl Server {
    /// Create a new server instance for a bound UdpSocket.
    pub fn new(socket: UdpSocket) -> Self {
        let (stop, stop_rx) = oneshot::channel();
        let (accept_tx, accept_rx) = mpsc::channel(10);

        let io = UdpIo::new(socket);
        tokio::spawn(async move { io.run(stop_rx, Some(accept_tx)).await });
        Self {
            stop: Some(stop),
            accept_rx,
        }
    }

    /// Accept a connection and perform DTLS handshake if context is provided.
    pub async fn accept(&mut self, tls: Option<SslContext>) -> Result<Session> {
        match self.accept_rx.recv().await {
            Some(s) => {
                let s = s?;
                let peer = s.peer();
                if let Some(ctx) = &tls {
                    let mut dtls = SslStream::new(Ssl::new(&ctx)?, s)?;
                    Pin::new(&mut dtls).accept().await.map_err(|_| {
                        StdError::new(ErrorKind::ConnectionReset, "Error during TLS handshake")
                    })?;
                    Ok(Session::new_dtls(dtls, peer))
                } else {
                    Ok(Session::new_udp(s, peer))
                }
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "Acceptor closed",
            )),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
