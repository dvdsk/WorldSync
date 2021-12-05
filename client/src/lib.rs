use shared::tarpc;
use tarpc::client::Config;
pub use tarpc::context;

pub use protocol::ServiceClient;
use tarpc::tokio_serde::formats::Bincode;
use tokio::net::TcpStream;

mod error;
mod events;
pub mod gui;
pub mod mc;
mod world_dl;
pub use error::Error;
pub use events::Event;
pub use world_dl::WorldDl;

#[cfg(feature = "deployed")]
use tokio_rustls::{client::TlsStream, rustls, TlsConnector};

#[cfg(feature = "deployed")]
async fn connect_tcp(domain: &str, port: u16) -> Result<TlsStream<TcpStream>, std::io::Error> {
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
    connector.connect(servername, stream).await
}

#[cfg(not(feature = "deployed"))]
async fn connect_tcp(_domain: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    TcpStream::connect(format!("127.0.0.1:{}", port)).await
}

pub async fn connect(domain: &str, port: u16) -> Result<ServiceClient, std::io::Error> {
    let conn = connect_tcp(domain, port).await?;
    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let mut codec_builder = LengthDelimitedCodec::builder();

    let framed = codec_builder.max_frame_length(usize::MAX).new_framed(conn);
    let transport = tarpc::serde_transport::new(framed, Bincode::default());
    let client = ServiceClient::new(Config::default(), transport).spawn();
    Ok(client)
}
