use bytes::{Bytes, BytesMut};
use std::io::Error as StdError;
use std::io::Result;

/// Unified trait for a packet stream and sink
pub trait PacketFramed:
    futures::Sink<Bytes, Error = StdError> + futures::Stream<Item = Result<BytesMut>>
{
}

impl<T> PacketFramed for T where
    T: futures::Sink<Bytes, Error = StdError> + futures::Stream<Item = Result<BytesMut>>
{
}
