use tokio::net::TcpStream;
use protocol::{tarpc, ServiceClient};
use tarpc::tokio_serde::formats::Json;
use tarpc::client::Config;

mod client;
mod tui;

pub async fn connect(port: u16) -> ServiceClient {
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let transport = tarpc::serde_transport::Transport::from((stream, Json::default()));
    let client = ServiceClient::new(Config::default(), transport).spawn();
    client
}

pub async fn run(port: u16) {
    let client = connect(port).await;
    tui::main_menu(client).await;
}
