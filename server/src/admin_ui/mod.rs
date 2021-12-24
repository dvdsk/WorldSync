use protocol::ServiceClient;

mod client;
mod tui;

pub async fn run(port: u16) {
    let client = protocol::connect_local(port).await.unwrap();
    tui::main_menu(client).await;
}
