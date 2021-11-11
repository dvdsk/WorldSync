use client::{connect, context};

#[tokio::main]
async fn main() {
    let client = connect(8080).await.unwrap();

    let version = client.version(context::current()).await;
    dbg!(version);
}
