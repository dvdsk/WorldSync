use protocol::ServiceClient;

mod client;
mod tui;

pub async fn run(port: u16) {
    let client = protocol::connect("127.0.0.1", port).await.unwrap();
    tui::main_menu(client).await;
}
