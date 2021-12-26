use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::time::sleep;
pub use server::util::spawn_test_server;

static FREE_PORT: AtomicU16 = AtomicU16::new(34879);
pub fn free_port() -> u16 {
    FREE_PORT.fetch_add(1, Ordering::Relaxed)
}

pub async fn test_conn(port: u16) -> protocol::ServiceClient {
    loop {
        sleep(Duration::from_millis(50)).await;
        match protocol::connect("127.0.0.1", port).await {
            Ok(client) => break client,
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => continue,
            Err(e) => panic!("could not connect to server: {:?}", e),
        }
    }
}
