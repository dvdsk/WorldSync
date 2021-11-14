use wrapper::{Handle, Instance};

#[tokio::test]
async fn test_start() {
    let (instance, handle) = Instance::start("tests/data", 2).await.unwrap();
    instance.maintain().await.unwrap();
}
