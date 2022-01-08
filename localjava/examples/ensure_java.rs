use futures::TryStreamExt;
use localjava::Version;
use std::path::Path;

#[tokio::main]
async fn main() {
    let path = Path::new("examples/test_local_java/");
    std::fs::create_dir_all(&path).unwrap();
    let res = localjava::version(&path);
    const NEEDED: Version = Version::new(17, 0, 1);
    match res {
        Ok(v) if v == NEEDED => println!("local java installed, working and up to date"),
        Ok(_) | Err(_) => {
            println!("java outdated or corrupt, updating");
            std::fs::remove_dir_all(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
            let stream = localjava::download_stream(path.to_owned()).await.unwrap();
            let _v: Vec<_> = stream
                .inspect_ok(|p| println!("\rprogress: {}", p))
                .try_collect()
                .await
                .unwrap();
        }
    }
}
