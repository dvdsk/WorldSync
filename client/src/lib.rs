use tokio::net::TcpStream;
use protocol::tarpc;
use tarpc::{client, tokio_serde::formats::Json};
pub use protocol::WorldClient;
pub use tarpc::context;

#[cfg(feature = "deployed")]
use tokio_rustls::{rustls, TlsConnector, client::TlsStream};

#[cfg(feature = "deployed")]
async fn connect_tcp(domain: &str, port: u16) -> TlsStream<TcpStream> {
    use std::sync::Arc;

    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("could not load os certificates") {
        roots.add(&rustls::Certificate(cert.0)).unwrap();
    }
    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let servername = rustls::ServerName::try_from(domain).unwrap();

    let host = format!("{}:{}", domain, port);
    let stream = TcpStream::connect(host).await.unwrap();
    connector.connect(servername, stream).await.unwrap()
}

#[cfg(not(feature = "deployed"))]
async fn connect_tcp(_domain: &str, port: u16) -> TcpStream {
    TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap()
}

pub async fn connect(port: u16) -> WorldClient {
    let stream = connect_tcp("davidsk.dev", port).await;
    let transport = tarpc::serde_transport::Transport::from((stream, Json::default()));
    let client = WorldClient::new(client::Config::default(), transport).spawn();
    client
}
