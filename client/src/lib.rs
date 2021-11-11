use tokio::net::TcpStream;
use protocol::tarpc;
use tarpc::tokio_serde::formats::Json;
use tarpc::client::Config;
pub use protocol::WorldClient;
pub use tarpc::context;

#[cfg(feature = "deployed")]
use tokio_rustls::{rustls, TlsConnector, client::TlsStream};

#[cfg(feature = "deployed")]
async fn connect_tcp(domain: &str, port: u16) -> Result<TlsStream<TcpStream>, std::io::Error>{
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
    let stream = TcpStream::connect(host).await?;
    connector.connect(servername, stream).await.unwrap()
}

#[cfg(not(feature = "deployed"))]
async fn connect_tcp(_domain: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    TcpStream::connect(format!("127.0.0.1:{}", port)).await
}

pub async fn connect(port: u16) -> Result<WorldClient, std::io::Error> {
    let stream = connect_tcp("davidsk.dev", port).await?;
    let transport = tarpc::serde_transport::Transport::from((stream, Json::default()));
    let client = WorldClient::new(Config::default(), transport).spawn();
    Ok(client)
}
