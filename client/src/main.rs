use client::{connect, context};

#[tokio::main]
async fn main() {
    let client = connect(8080).await;

    let hello = async move {
        // Send the request twice, just to be safe! ;)
        tokio::select! {
            hello1 = client.version(context::current()) => { hello1 }
            hello2 = client.version(context::current()) => { hello2 }
        }
    }
    .await;
    dbg!(hello);
}
