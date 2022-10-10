use futures::SinkExt;
use futures::StreamExt;
use openssl::ssl::{SslContext, SslFiletype, SslMethod};
use std::path::PathBuf;
use tokio::net::UdpSocket;
use tokio::sync::oneshot;

use tokio_dtls_stream::Client;
use tokio_dtls_stream::Server;

#[tokio::test]
async fn test_plain_client() {
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let saddr = server.local_addr().unwrap();

    let s = tokio::spawn(async move {
        let mut buf = [0; 2048];
        match server.recv_from(&mut buf).await {
            Ok((len, src)) => {
                assert!(server.send_to(&buf[..len], src).await.is_ok());
            }
            Err(e) => {
                assert!(false, "Error while receiving data: {:?}", e);
            }
        }
    });

    let client = Client::new(client);

    let mut stream = client.connect(saddr, None).await.unwrap();
    stream.send("PING".into()).await.unwrap();
    let rx = stream.next().await.unwrap().unwrap();
    s.await.unwrap();

    assert_eq!(b"PING", &rx[..]);

    drop(stream);
    drop(client);
}

#[tokio::test]
async fn test_plain_server() {
    let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let saddr = server.local_addr().unwrap();

    let (sig, mut stop) = oneshot::channel();
    let mut server = Server::new(server);
    let s = tokio::spawn(async move {
        let mut c = server.accept(None).await.unwrap();
        loop {
            tokio::select! {
                _ = &mut stop => {
                    break;
                }
                r = c.next() => match r {
                    Some(Ok(rx)) => {
                        assert!(c.send(rx.into()).await.is_ok());
                    }
                    Some(Err(e)) => {
                        assert!(false, "Error while receiving data: {:?}", e);
                    }
                    _ => {
                        assert!(false, "Stream closed unexpectedly");
                    }
                }
            }
        }
    });

    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut rx = [0; 4];
    let tx_fut = client.send_to(b"PING", saddr);
    let rx_fut = client.recv_from(&mut rx[..]);

    let (rxr, _) = tokio::join!(rx_fut, tx_fut);

    let (len, from) = rxr.unwrap();

    assert_eq!(4, len);
    assert_eq!(saddr, from);
    sig.send(()).unwrap();
    s.await.unwrap();

    assert_eq!(b"PING", &rx[..]);
}

#[tokio::test]
async fn test_dtls() {
    let base = env!("CARGO_MANIFEST_DIR");
    let key: PathBuf = [base, "tests", "certs", "server-key.pem"].iter().collect();
    let cert: PathBuf = [base, "tests", "certs", "server-cert.pem"].iter().collect();
    let ca: PathBuf = [base, "tests", "certs", "ca-cert.pem"].iter().collect();

    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let saddr = server.local_addr().unwrap();

    let mut ctx = SslContext::builder(SslMethod::dtls()).unwrap();
    ctx.set_private_key_file(key, SslFiletype::PEM).unwrap();
    ctx.set_certificate_chain_file(cert).unwrap();
    ctx.set_ca_file(ca).unwrap();
    ctx.check_private_key().unwrap();
    let ctx = ctx.build();

    let client = Client::new(client);
    let mut server = Server::new(server);
    let (sig, mut stop) = oneshot::channel();

    let c = ctx.clone();
    let s = tokio::spawn(async move {
        let mut c = server.accept(Some(c)).await.unwrap();
        loop {
            tokio::select! {
                _ = &mut stop => {
                    break;
                }
                r = c.next() => match r {
                    Some(Ok(rx)) => {
                        assert!(c.send(rx.into()).await.is_ok());
                    }
                    Some(Err(e)) => {
                        assert!(false, "Error while receiving data: {:?}", e);
                    }
                    _ => {
                        assert!(false, "Stream closed unexpectedly");
                    }
                }
            }
        }
    });

    let mut stream = client.connect(saddr, Some(ctx)).await.unwrap();
    stream.send("PING".into()).await.unwrap();
    let rx = stream.next().await.unwrap().unwrap();
    sig.send(()).unwrap();
    s.await.unwrap();

    assert_eq!(b"PING", &rx[..]);
}
