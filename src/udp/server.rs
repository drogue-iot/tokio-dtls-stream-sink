use crate::udp::io::UdpIo;
use crate::udp::stream::UdpStream;

use std::io::Result;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};

pub struct UdpServer {
    stop: Option<oneshot::Sender<()>>,
    accept_rx: mpsc::Receiver<Result<UdpStream>>,
}

impl UdpServer {
    pub fn new(socket: UdpSocket) -> Self {
        let (stop, stop_rx) = oneshot::channel();
        let (accept_tx, accept_rx) = mpsc::channel(10);

        let io = UdpIo::new(socket);
        tokio::spawn(async move {
            io.run(stop_rx, Some(accept_tx)).await
        });
        Self {
            stop: Some(stop),
            accept_rx,
        }
    }

    pub async fn accept(&mut self) -> Result<UdpStream> {
        match self.accept_rx.recv().await {
            Some(s) => s,
            None => Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "Acceptor closed")),
        }
    }
}


impl Drop for UdpServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
