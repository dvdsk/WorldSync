use tokio::net::TcpStream;
use protocol::{tarpc, WorldClient};
use tarpc::tokio_serde::formats::Json;
use tarpc::client::Config;

mod client;
mod tui;

pub async fn connect(port: u16) -> WorldClient {
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let transport = tarpc::serde_transport::Transport::from((stream, Json::default()));
    let client = WorldClient::new(Config::default(), transport).spawn();
    client
}

pub async fn run(port: u16) {
    let client = connect(port).await;
    tui::main_menu(client).await;
}
