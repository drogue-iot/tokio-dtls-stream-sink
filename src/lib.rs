#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub mod client;
pub use client::*;

pub mod server;
pub use server::*;

pub(crate) mod packet_stream;
mod udp;
pub use packet_stream::PacketStream;
