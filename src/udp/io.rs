use bytes::Bytes;
use std::collections::{hash_map::Entry, HashMap};
use std::io::Result;
use std::io::{Error as StdError, ErrorKind};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use super::stream::UdpStream;

/// UDP I/O layer providing UDP sessions with a packet stream interface.
pub struct UdpIo {
    socket: UdpSocket,

    tx_out: mpsc::Sender<(SocketAddr, Bytes)>,
    rx_out: Mutex<mpsc::Receiver<(SocketAddr, Bytes)>>,

    tx_done: mpsc::Sender<SocketAddr>,
    rx_done: Mutex<mpsc::Receiver<SocketAddr>>,

    peers: Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>,
}

impl UdpIo {
    pub(crate) fn new(socket: UdpSocket) -> Self {
        let (tx_out, rx_out) = mpsc::channel(10);
        let (tx_done, rx_done) = mpsc::channel(10);
        Self {
            socket,
            tx_out,
            rx_out: Mutex::new(rx_out),
            tx_done,
            rx_done: Mutex::new(rx_done),
            peers: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn connect(&self, peer: SocketAddr) -> Result<UdpStream> {
        let mut peers = self.peers.lock().await;
        if peers.contains_key(&peer) {
            return Err(StdError::new(ErrorKind::AlreadyExists, "Already connected"));
        }

        let (tx_in, rx_in) = mpsc::channel(10);
        let tx_out = self.tx_out.clone();
        let tx_done = self.tx_done.clone();
        let udp = UdpStream::new(peer, tx_out, rx_in, tx_done)?;
        peers.insert(peer, tx_in);
        Ok(udp)
    }

    pub(crate) async fn run(
        &self,
        mut stop: oneshot::Receiver<()>,
        mut acceptor: Option<mpsc::Sender<Result<UdpStream>>>,
    ) -> Result<()> {
        let mut buf = [0; 2048];
        let mut rx_out = self.rx_out.lock().await;
        let mut rx_done = self.rx_done.lock().await;
        loop {
            tokio::select! {
                inbound = self.socket.recv_from(&mut buf) => {
                    match inbound {
                        Ok((size, src)) => {
                            let mut peers = self.peers.lock().await;
                            let entry = match peers.entry(src) {
                                Entry::Occupied(o) => {
                                    Some(o.into_mut())
                                }
                                Entry::Vacant(v) => {
                                    if let Some(acceptor) = &mut acceptor {
                                        let (tx_in, rx_in) = mpsc::channel(10);
                                        let tx_out = self.tx_out.clone();
                                        let tx_done = self.tx_done.clone();
                                        let udp = UdpStream::new(src, tx_out, rx_in, tx_done);
                                        let r = if udp.is_ok() {
                                            Some(v.insert(tx_in))
                                        } else {
                                            None
                                        };

                                        let _ = acceptor.try_send(udp);
                                        r
                                    } else {
                                        None
                                    }
                                }
                            };
                            if let Some(tx_in) = entry {
                                if let Err(e) = tx_in.send(Bytes::copy_from_slice(&buf[..size])).await {
                                    log::warn!("IO error sending data to session (closing): {:?}", e);
                                    let _ = peers.remove(&src);
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("IO error: {:?}", e);
                            return Err(e);
                        }
                    }
                }
                outbound = rx_out.recv() => {
                    match outbound {
                        Some((dest, data)) => {
                            match self.socket.send_to(&data[..], &dest).await {
                                Ok(_) => {}
                                Err(e) => {
                                    log::warn!("IO error while sending data to {}: {:?}", dest, e);
                                }
                            }
                        }
                        None => {
                            return Ok(())
                        }
                    }
                }
                done = rx_done.recv() => {
                    if let Some(peer) = done {
                        let mut peers = self.peers.lock().await;
                        let _ = peers.remove(&peer);
                    }
                }
                _ = &mut stop => {
                    return Ok(());
                }
            }
        }
    }
}
