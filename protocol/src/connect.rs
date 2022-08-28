use shared::tarpc;
use tarpc::tokio_util;
use tarpc::client::Config;

use crate::ServiceClient;
use tarpc::tokio_serde::formats::Bincode;
use tokio::net::TcpStream;
use tracing::instrument;

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
#[instrument(err)]
async fn connect_tcp(domain: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    TcpStream::connect(format!("{}:{}", domain, port)).await
}

#[instrument(err)]
pub async fn connect(domain: &str, port: u16) -> Result<ServiceClient, std::io::Error> {
    let conn = connect_tcp(domain, port).await?;
    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let mut codec_builder = LengthDelimitedCodec::builder();

    let framed = codec_builder.max_frame_length(usize::MAX).new_framed(conn);
    let transport = tarpc::serde_transport::new(framed, Bincode::default());
    let client = ServiceClient::new(Config::default(), transport).spawn();
    Ok(client)
}

#[instrument(err)]
pub async fn connect_local(port: u16) -> Result<ServiceClient, std::io::Error> {
    let conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let mut codec_builder = LengthDelimitedCodec::builder();

    let framed = codec_builder.max_frame_length(usize::MAX).new_framed(conn);
    let transport = tarpc::serde_transport::new(framed, Bincode::default());
    let client = ServiceClient::new(Config::default(), transport).spawn();
    Ok(client)
}
