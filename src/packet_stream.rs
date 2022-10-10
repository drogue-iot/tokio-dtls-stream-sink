use bytes::{Bytes, BytesMut};
use std::io::Error as StdError;
use std::io::Result;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{BytesCodec, Framed};

/// Unified trait for a packet stream and sink
pub trait PacketStream:
    futures::Sink<Bytes, Error = StdError> + futures::Stream<Item = Result<BytesMut>> + Unpin + Send
{
}

pub(crate) struct FramedPacketStream<T>(pub(crate) Framed<T, BytesCodec>)
where
    T: AsyncRead + AsyncWrite + Unpin;

impl<T> PacketStream for FramedPacketStream<T> where T: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> futures::Sink<Bytes> for FramedPacketStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Error = std::io::Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        <Framed<T, BytesCodec> as futures::Sink<Bytes>>::poll_ready(Pin::new(&mut self.0), cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Bytes) -> Result<()> {
        <Framed<T, BytesCodec> as futures::Sink<Bytes>>::start_send(Pin::new(&mut self.0), item)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        <Framed<T, BytesCodec> as futures::Sink<Bytes>>::poll_flush(Pin::new(&mut self.0), cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        <Framed<T, BytesCodec> as futures::Sink<Bytes>>::poll_close(Pin::new(&mut self.0), cx)
    }
}

impl<T> futures::Stream for FramedPacketStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<BytesMut>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx)
    }
}
