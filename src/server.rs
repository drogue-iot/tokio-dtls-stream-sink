use super::packet_stream::*;
use crate::udp::io::UdpIo;
use crate::udp::stream::UdpStream;
use core::pin::Pin;
use openssl::ssl::{Ssl, SslContext};
use std::io::{Error as StdError, ErrorKind, Result};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio_openssl::SslStream;
use tokio_util::codec::{BytesCodec, Decoder};

pub struct Server {
    stop: Option<oneshot::Sender<()>>,
    accept_rx: mpsc::Receiver<Result<UdpStream>>,
}

impl Server {
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

    pub async fn accept(&mut self, tls: Option<SslContext>) -> Result<Box<dyn PacketStream>> {
        match self.accept_rx.recv().await {
            Some(s) => {
                let s = s?;
                if let Some(ctx) = &tls {
                    let mut dtls = SslStream::new(Ssl::new(&ctx)?, s)?;
                    Pin::new(&mut dtls).accept().await.map_err(|_| {
                        StdError::new(ErrorKind::ConnectionReset, "Error during TLS handshake")
                    })?;
                    Ok(Box::new(FramedPacketStream(BytesCodec::new().framed(dtls))))
                } else {
                    Ok(Box::new(FramedPacketStream(BytesCodec::new().framed(s))))
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
