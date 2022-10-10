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
use tokio_util::codec::{BytesCodec, Decoder};

use super::packet_stream::*;

pub struct Client {
    io: Arc<UdpIo>,
    stop: Option<oneshot::Sender<()>>,
}

impl Client {
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

    pub async fn connect<S: ToSocketAddrs>(
        &self,
        peer: S,
        tls: Option<SslContext>,
    ) -> Result<Box<dyn PacketStream>> {
        let mut bind = peer.to_socket_addrs()?;
        if let Some(peer) = bind.next() {
            let r = self.io.connect(peer).await?;
            if let Some(ctx) = &tls {
                let mut dtls = SslStream::new(Ssl::new(&ctx)?, r)?;
                Pin::new(&mut dtls).connect().await.map_err(|_| {
                    StdError::new(ErrorKind::ConnectionReset, "Error during TLS handshake")
                })?;
                Ok(Box::new(FramedPacketStream(BytesCodec::new().framed(dtls))))
            } else {
                Ok(Box::new(FramedPacketStream(BytesCodec::new().framed(r))))
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
