use protocol::{tarpc, ServiceClient};
use tarpc::client::Config;
use tarpc::tokio_serde::formats::Json;
use tokio::net::TcpStream;

mod client;
mod tui;

pub async fn connect(port: u16) -> ServiceClient {
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    let transport = tarpc::serde_transport::Transport::from((stream, Json::default()));
    ServiceClient::new(Config::default(), transport).spawn()
}

pub async fn run(port: u16) {
    let client = connect(port).await;
    tui::main_menu(client).await;
}
