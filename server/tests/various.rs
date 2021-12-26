mod util;
use std::net::SocketAddr;

use protocol::ServiceClient;
use shared::tarpc::client::Config;
use shared::tarpc::{context, self};
use shared::tarpc::tokio_serde::formats::Bincode;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use util::{free_port, test_conn, spawn_test_server};

async fn version(port: u16) -> protocol::Version {
    let client = test_conn(port).await;
    client.version(context::current()).await.unwrap()
}

#[tokio::test]
async fn version_matches() {
    let correct = protocol::current_version();

    let port = free_port();
    spawn_test_server(port).await;
    let result = version(port).await;
    assert_eq!(correct, result);
}

fn pp_header() -> Vec<u8> {
    use std::str::FromStr;
    use ppp::v2::{Version, Command, Protocol, Builder};
    let source = SocketAddr::from_str("192.168.42.42:80").unwrap();
    let dest = SocketAddr::from_str("192.168.2.2:80").unwrap();
    Builder::with_addresses(
        Version::Two | Command::Proxy,
        Protocol::Stream,
        (source, dest),
    )
    .build().unwrap()
}

async fn req_client_with_header(port: u16) -> protocol::Version {
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    conn.write(&pp_header()).await.unwrap();

    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let mut codec_builder = LengthDelimitedCodec::builder();

    let framed = codec_builder.max_frame_length(usize::MAX).new_framed(conn);
    let transport = tarpc::serde_transport::new(framed, Bincode::default());
    let client = ServiceClient::new(Config::default(), transport).spawn();
    client.version(context::current()).await.unwrap()
}

#[tokio::test]
async fn test_header_stripping() {
    shared::setup_tracing() ;

    let port = free_port();
    spawn_test_server(port).await;

    let result = req_client_with_header(port).await;
    assert_eq!(result, protocol::current_version());
}
