use tokio_dtls_stream::udp::UdpClient;
use tokio_dtls_stream::udp::UdpServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;

#[tokio::test]
async fn test_client() {
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

    let client = UdpClient::new(client);

    let mut stream = client.connect(saddr).await.unwrap();
    let mut rx = [0; 4];
    stream.write(b"PING").await.unwrap();
    stream.read(&mut rx).await.unwrap();
    s.await.unwrap();

    assert_eq!(b"PING", &rx[..]);

    drop(stream);
    drop(client);
}

#[tokio::test]
async fn test_server() {
    let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let saddr = server.local_addr().unwrap();

    let mut server = UdpServer::new(server);
    let s = tokio::spawn(async move {
        let mut c = server.accept().await.unwrap();
        let mut buf = [0; 2048];
        match c.read(&mut buf).await {
            Ok(len) => {
                assert!(c.write(&buf[..len]).await.is_ok());
            }
            Err(e) => {
                assert!(false, "Error while receiving data: {:?}", e);
            }
        }
    });

    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut rx = [0; 4];
    client.send_to(b"PING", saddr).await.unwrap();
    let (len, from) = client.recv_from(&mut rx[..]).await.unwrap();
    assert_eq!(4, len);
    assert_eq!(saddr, from);
    s.await.unwrap();

    assert_eq!(b"PING", &rx[..]);
}
