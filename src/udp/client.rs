use crate::udp::io::UdpIo;
use crate::udp::stream::UdpStream;

use std::io::Result;
use std::net::ToSocketAddrs;
use tokio::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct UdpClient {
    io: Arc<UdpIo>,
    stop: Option<oneshot::Sender<()>>,
}

impl UdpClient {
    pub fn new(socket: UdpSocket) -> Self {
        let (stop, stop_rx) = oneshot::channel();
        let io = Arc::new(UdpIo::new(socket));
        let runner = io.clone();
        tokio::spawn(async move {
            runner.run(stop_rx, None).await
        });
        Self {
            io,
            stop: Some(stop),
        }
    }

    pub async fn connect<S: ToSocketAddrs>(&self, peer: S) -> Result<UdpStream> {
        let mut bind = peer.to_socket_addrs()?;
        if let Some(peer) = bind.next() {
            self.io.connect(peer).await
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "Peer address not available",
            ))
        }
    }
}


impl Drop for UdpClient {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
