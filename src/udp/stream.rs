use bytes::Bytes;
use futures::SinkExt;

use core::pin::Pin;
use core::task::{Context, Poll};
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::io::{AsyncRead, AsyncWrite};

use tokio::sync::mpsc;

use tokio_util::sync::PollSender;

/// Represents a stream of UDP datagrams between ourselves and a peer.
pub(crate) struct UdpStream {
    peer: SocketAddr,
    tx: PollSender<(SocketAddr, Bytes)>,
    rx: mpsc::Receiver<Bytes>,
    done: mpsc::Sender<SocketAddr>,
}

impl UdpStream {
    pub(crate) fn new<S: ToSocketAddrs>(
        peer: S,
        tx: mpsc::Sender<(SocketAddr, Bytes)>,
        rx: mpsc::Receiver<Bytes>,
        done: mpsc::Sender<SocketAddr>,
    ) -> std::io::Result<Self> {
        let mut bind = peer.to_socket_addrs()?;
        if let Some(peer) = bind.next() {
            Ok(Self {
                peer,
                tx: PollSender::new(tx),
                rx,
                done,
            })
        } else {
            Err(std::io::Error::new(
                ErrorKind::AddrNotAvailable,
                "Peer address not available",
            ))
        }
    }
}

impl Drop for UdpStream {
    fn drop(&mut self) {
        let _ = self.done.try_send(self.peer);
    }
}

impl AsyncRead for UdpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        rb: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.rx.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(message)) => {
                rb.put_slice(&message[..]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Err(std::io::Error::new(
                ErrorKind::BrokenPipe,
                "Channel closed",
            ))),
        }
    }
}

impl AsyncWrite for UdpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let peer = self.peer;
        match self.tx.poll_reserve(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => match r {
                Ok(_) => match self.tx.send_item((peer, Bytes::copy_from_slice(buf))) {
                    Ok(_) => Poll::Ready(Ok(buf.len())),
                    Err(_) => Poll::Ready(Err(std::io::Error::new(
                        ErrorKind::BrokenPipe,
                        "Channel closed",
                    ))),
                },
                Err(_) => Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "Channel closed",
                ))),
            },
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.tx
            .poll_flush_unpin(cx)
            .map_err(|_| std::io::Error::new(ErrorKind::BrokenPipe, "Flush error"))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        if !self.tx.is_closed() {
            self.tx.close();
        }
        Poll::Ready(Ok(()))
    }
}
