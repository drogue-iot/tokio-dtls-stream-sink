# tokio-dtls-stream

[![CI](https://github.com/drogue-iot/tokio-dtls-stream/workflows/CI/badge.svg)](https://github.com/drogue-iot/tokio-dtls-stream/actions?query=workflow%3A%22CI%22)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/tag/drogue-iot/tokio-dtls-stream?sort=semver)](https://github.com/drogue-iot/tokio-dtls-stream/releases)
[![Matrix](https://img.shields.io/matrix/drogue-iot:matrix.org)](https://matrix.to/#/#drogue-iot:matrix.org)

Tokio-based streaming API for UDP datagrams over DTLS. 

This crate combines [openssl](https://crates.io/crates/openssl), [tokio-openssl](https://crates.io/crates/tokio-openssl) and [tokio](https://tokio.rs/) into "client" and "server" APIs with support for establishing DTLS sessions.
