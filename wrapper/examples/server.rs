use wrapper::Instance;

#[tokio::main]
async fn main() {
    let (mut instance, _handle) = Instance::start("tests/data", 2).await.unwrap();

    loop {
        let event = instance.next_event().await.unwrap();
        println!("event: {:?}", event);
    }
}
