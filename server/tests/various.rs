mod util;
use protocol::tarpc::context;
use util::{free_port, test_conn, test_server};

async fn version(port: u16) -> protocol::Version {
    let client = test_conn(port).await;
    client.version(context::current()).await.unwrap()
}

#[tokio::test]
async fn version_matches() {
    let correct = protocol::Version {
        protocol: protocol::version().to_owned(),
        server: server::version().to_owned(),
    };

    let port = free_port();
    let server = test_server(port);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = version(port) => r,
    };
    assert_eq!(correct, result);
}
